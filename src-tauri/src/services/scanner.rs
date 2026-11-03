//! Indexes a user-selected wallpaper directory into the SQLite library and keeps
//! thumbnails ahead of the gallery UI.
//!
//! The gallery reads exclusively from the database, so pointing 图库目录 at a
//! folder of previously downloaded images only takes effect once those files are
//! indexed here. A real folder holds thousands of files, so every pass that touches
//! the disk runs on the blocking pool and commits in chunks.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, TimeZone};
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinSet;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{ScanFinished, ScanPhase, ScanProgress};
use crate::infra::db::{Db, IndexedRow, NewImage};
use crate::services::thumbnail;
use crate::state::AppState;

const IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
/// Library rows written per commit, and the granularity of index progress.
const INDEX_CHUNK: usize = 200;
/// Thumbnails between progress events; the UI only needs a smooth bar.
const THUMB_REPORT_EVERY: u32 = 24;
/// Decoding a 4K frame costs tens of megabytes, so cap the workers below the
/// core count on big machines.
const MAX_THUMB_WORKERS: usize = 4;

/// One candidate file found in the wallpaper directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    pub path: PathBuf,
    /// Empty until the date is recovered from the name or the file's own mtime.
    pub date: String,
    pub bytes: u64,
    pub modified: i64,
}

/// An image row ready for the database, owned so plans can be asserted in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedImage {
    pub date: String,
    pub file_path: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub created_at: String,
}

/// What the scanner decided to do with one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    Import(PlannedImage),
    Relink(PlannedImage),
    Skip,
}

impl<'a> From<&'a PlannedImage> for NewImage<'a> {
    fn from(image: &'a PlannedImage) -> Self {
        Self {
            date: &image.date,
            title: "",
            copyright: "",
            copyright_link: "",
            url: "",
            file_path: &image.file_path,
            thumb_path: "",
            width: image.width,
            height: image.height,
            bytes: image.bytes,
            created_at: &image.created_at,
            external: true,
        }
    }
}

/// Case and separator insensitive key for the tombstone table, so the same file
/// stays ignored whatever separators the path was recorded with.
pub fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| IMAGE_EXTENSIONS.contains(&value.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Recover the wallpaper date from a file stem: `20160323`, `bing20261102`,
/// `bing20261102-1` (the de-duplicating suffix) and `2026-11-02` all resolve.
pub fn date_from_stem(stem: &str) -> Option<String> {
    let trimmed = stem.trim();
    let without_prefix = match trimmed.get(..4) {
        Some(head) if head.eq_ignore_ascii_case("bing") => &trimmed[4..],
        _ => trimmed,
    };

    if let Some(date) = parse_date(without_prefix) {
        return Some(date);
    }

    // A trailing `-1`, `-2` … suffix is a collision suffix, not part of the date.
    let (head, suffix) = without_prefix.rsplit_once('-')?;
    if suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return parse_date(head);
    }
    None
}

fn parse_date(value: &str) -> Option<String> {
    ["%Y%m%d", "%Y-%m-%d", "%Y_%m_%d", "%Y.%m.%d"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(value, format).ok())
        .map(|date| date.format("%Y-%m-%d").to_string())
}

fn date_from_timestamp(seconds: i64) -> String {
    chrono::Local
        .timestamp_opt(seconds, 0)
        .single()
        .map(|moment| moment.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

fn file_mtime(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|when| when.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0)
}

/// List image files in `dir`, newest first, skipping paths the user removed from
/// the gallery.
pub fn list_candidates(dir: &Path, removed: &HashSet<String>) -> Vec<ScannedFile> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!("无法读取图库目录 {}：{error}", dir.display());
            return Vec::new();
        }
    };

    let mut files: Vec<ScannedFile> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || !is_image_path(&path) {
                return None;
            }
            if removed.contains(&path_key(&path)) {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            Some(ScannedFile {
                modified: file_mtime(&metadata),
                bytes: metadata.len(),
                date: path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(date_from_stem)
                    .unwrap_or_default(),
                path,
            })
        })
        .collect();

    files.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| right.bytes.cmp(&left.bytes))
            .then_with(|| left.path.cmp(&right.path))
    });
    files
}

/// One entry per date: the newest file wins, and pictures without a date in their
/// name are dated by their own mtime so they still show up.
pub fn dedupe(candidates: Vec<ScannedFile>) -> Vec<ScannedFile> {
    let mut seen: HashSet<String> = HashSet::with_capacity(candidates.len());
    let mut kept: Vec<ScannedFile> = Vec::with_capacity(candidates.len());

    for mut file in candidates {
        if file.date.is_empty() {
            file.date = date_from_timestamp(file.modified);
        }
        if seen.insert(file.date.clone()) {
            kept.push(file);
        }
    }

    kept
}

/// Decide what to do with one deduplicated candidate against the current index.
pub fn plan_file(
    file: &ScannedFile,
    indexed: &HashMap<String, IndexedRow>,
    created_at: &str,
) -> Plan {
    match indexed.get(&file.date) {
        None => Plan::Import(plan(file, created_at)),
        Some(row) => {
            let recorded = Path::new(&row.file_path);
            if path_key(recorded) == path_key(&file.path) {
                // Same file: only a missing thumbnail matters, and the thumbnail
                // phase picks that up.
                return Plan::Skip;
            }
            if recorded.is_file() {
                // An already indexed sibling covers this date and wins.
                return Plan::Skip;
            }
            Plan::Relink(plan(file, created_at))
        }
    }
}

fn plan(file: &ScannedFile, created_at: &str) -> PlannedImage {
    let (width, height) = thumbnail::dimensions(&file.path);
    PlannedImage {
        date: file.date.clone(),
        file_path: file.path.to_string_lossy().to_string(),
        width,
        height,
        bytes: file.bytes,
        created_at: created_at.to_string(),
    }
}

/// A pending thumbnail build: the row, its source and the deterministic target.
struct ThumbTask {
    id: i64,
    source: PathBuf,
    destination: PathBuf,
}

/// Blocking half of the thumbnail phase.
fn build_thumbs(tasks: Vec<ThumbTask>) -> (Vec<(i64, String)>, u32) {
    let mut built = Vec::with_capacity(tasks.len());
    let mut failed = 0u32;
    for task in tasks {
        match thumbnail::generate(&task.source, &task.destination, thumbnail::THUMB_WIDTH) {
            Ok(()) => built.push((task.id, task.destination.to_string_lossy().to_string())),
            Err(error) => {
                failed += 1;
                log::debug!("缩略图生成失败 {}：{error}", task.source.display());
            }
        }
    }
    (built, failed)
}

/// Row id and its recorded thumbnail path: the batch shape `set_thumbs` accepts.
type ThumbWrites = Vec<(i64, String)>;

/// Rows still needing a thumbnail, plus corrections for rows whose deterministic
/// thumbnail already exists but is not recorded.
fn collect_thumb_work(
    db: &Db,
    thumbs_dir: &Path,
) -> AppResult<(Vec<ThumbTask>, ThumbWrites)> {
    let mut tasks = Vec::new();
    let mut corrections = Vec::new();
    for row in db.thumb_rows()? {
        let source = Path::new(&row.file_path);
        if !source.is_file() {
            continue;
        }
        let destination = thumbnail::thumb_path_for(thumbs_dir, &row.date);
        let recorded = destination.to_string_lossy().to_string();
        if thumbnail::is_fresh(source, &destination) {
            if recorded != row.thumb_path {
                corrections.push((row.id, recorded));
            }
            continue;
        }
        tasks.push(ThumbTask {
            id: row.id,
            source: source.to_path_buf(),
            destination,
        });
    }
    Ok((tasks, corrections))
}

fn emit_progress(app: &AppHandle, phase: ScanPhase, current: u32, total: u32, message: String) {
    let percent = if total == 0 {
        0
    } else {
        ((current as f64 / total as f64) * 100.0).round() as u32
    };
    let _ = app.emit(
        "library://scan-progress",
        ScanProgress {
            phase,
            current,
            total,
            percent: percent.min(100),
            message,
        },
    );
}

/// Walk and index the configured directory, then build the missing thumbnails.
pub async fn run(app: AppHandle) -> AppResult<ScanFinished> {
    let (images_dir, thumbs_dir) = {
        let state = app.state::<AppState>();
        (state.images_dir(), state.thumbs_dir.clone())
    };
    crate::infra::fs::ensure_dir(&thumbs_dir)?;
    let images_dir_display = images_dir.to_string_lossy().to_string();

    let (indexed, removed, created_at) = {
        let state = app.state::<AppState>();
        (
            indexed_map(&state.db)?,
            state.db.removed_paths()?,
            AppState::now_rfc3339(),
        )
    };

    emit_progress(
        &app,
        ScanPhase::Index,
        0,
        0,
        format!("正在扫描 {images_dir_display}"),
    );

    // The walk reads thousands of JPEG headers, so it stays on the blocking pool;
    // only the finished plans come back.
    let planned = tokio::task::spawn_blocking(move || {
        let files = dedupe(list_candidates(&images_dir, &removed));
        let total = files.len() as u32;
        let mut imports = Vec::new();
        let mut relinks = Vec::new();
        for file in &files {
            match plan_file(file, &indexed, &created_at) {
                Plan::Import(image) => imports.push(image),
                Plan::Relink(image) => relinks.push(image),
                Plan::Skip => {}
            }
        }
        (imports, relinks, total)
    })
    .await
    .map_err(|error| AppError::Other(format!("扫描任务异常：{error}")))?;

    let (imports, relinks, candidates) = planned;
    let state = app.state::<AppState>();
    let mut imported = 0u32;
    let mut relinked = 0u32;
    let mut committed = 0u32;
    let import_total = imports.len() as u32;

    for chunk in imports.chunks(INDEX_CHUNK) {
        let rows: Vec<NewImage<'_>> = chunk.iter().map(NewImage::from).collect();
        imported += state.db.insert_scanned(&rows)?;
        committed += chunk.len() as u32;
        emit_progress(
            &app,
            ScanPhase::Index,
            committed,
            import_total,
            format!("正在导入图库 {committed}/{import_total}"),
        );
        // The gallery can start filling in while the rest is still committing.
        let _ = app.emit("library://changed", ());
    }

    for image in &relinks {
        if state.db.relink_scanned(&image.date, &NewImage::from(image))? {
            relinked += 1;
        }
    }

    if imported > 0 || relinked > 0 {
        emit_progress(
            &app,
            ScanPhase::Index,
            import_total,
            import_total,
            "索引完成".to_string(),
        );
        let _ = app.emit("library://changed", ());
    }
    log::debug!("图库目录 {images_dir_display} 共 {candidates} 个候选文件");

    let (tasks, corrections) = collect_thumb_work(&state.db, &thumbs_dir)?;
    if !corrections.is_empty() {
        state.db.set_thumbs(&corrections)?;
    }

    let total = tasks.len() as u32;
    let mut built = 0u32;
    let mut failed = 0u32;

    if total > 0 {
        emit_progress(
            &app,
            ScanPhase::Thumbnail,
            0,
            total,
            format!("正在生成缩略图 0/{total}"),
        );

        let workers = worker_count();
        let mut queue: Vec<Vec<ThumbTask>> = (0..workers).map(|_| Vec::new()).collect();
        for (index, task) in tasks.into_iter().enumerate() {
            queue[index % workers].push(task);
        }

        let mut set = JoinSet::new();
        for chunk in queue {
            set.spawn_blocking(move || build_thumbs(chunk));
        }

        // Writing thumbs one-by-one would hold the database lock away from the
        // UI every few frames, so commit in batches.
        let mut batched: Vec<(i64, String)> = Vec::new();
        while let Some(result) = set.join_next().await {
            let (rows, errors) = result
                .map_err(|error| AppError::Other(format!("缩略图任务异常：{error}")))?;
            built += rows.len() as u32;
            failed += errors;
            batched.extend(rows);

            if batched.len() >= THUMB_REPORT_EVERY as usize || set.is_empty() {
                state.db.set_thumbs(&batched)?;
                batched.clear();
                emit_progress(
                    &app,
                    ScanPhase::Thumbnail,
                    built,
                    total,
                    format!("正在生成缩略图 {built}/{total}"),
                );
                let _ = app.emit("library://changed", ());
            }
        }
        state.db.set_thumbs(&batched)?;
    }

    let finished = ScanFinished {
        imported,
        relinked,
        thumbnails: built,
        failed,
        error: None,
    };
    let _ = app.emit("library://scan-finished", &finished);
    log::info!(
        "图库扫描完成：新增 {}，重指向 {}，缩略图 {}，失败 {}",
        finished.imported,
        finished.relinked,
        finished.thumbnails,
        finished.failed
    );
    Ok(finished)
}

fn indexed_map(db: &Db) -> AppResult<HashMap<String, IndexedRow>> {
    Ok(db
        .indexed_rows()?
        .into_iter()
        .map(|row| (row.date.clone(), row))
        .collect())
}

fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2)
        .clamp(1, MAX_THUMB_WORKERS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanned(date: &str, name: &str, bytes: u64, modified: i64) -> ScannedFile {
        ScannedFile {
            path: PathBuf::from(format!("D:/bing_pic/{name}")),
            date: date.to_string(),
            bytes,
            modified,
        }
    }

    fn indexed_row(date: &str, file_path: &str) -> IndexedRow {
        IndexedRow {
            date: date.to_string(),
            file_path: file_path.to_string(),
        }
    }

    #[test]
    fn date_from_stem_handles_every_local_naming() {
        assert_eq!(date_from_stem("20160323").as_deref(), Some("2016-03-23"));
        assert_eq!(date_from_stem("bing20261102").as_deref(), Some("2026-11-02"));
        assert_eq!(date_from_stem("BING20261102").as_deref(), Some("2026-11-02"));
        assert_eq!(
            date_from_stem("bing20261102-1").as_deref(),
            Some("2026-11-02")
        );
        assert_eq!(date_from_stem("2026-11-02").as_deref(), Some("2026-11-02"));
        assert_eq!(date_from_stem("2026_11_02").as_deref(), Some("2026-11-02"));
        assert_eq!(date_from_stem("2026.11.02").as_deref(), Some("2026-11-02"));
    }

    #[test]
    fn date_from_stem_rejects_noise_and_impossible_days() {
        assert_eq!(date_from_stem("sunset"), None);
        assert_eq!(date_from_stem("20161340"), None);
        assert_eq!(date_from_stem("bing2016abc"), None);
        assert_eq!(date_from_stem("2016-03"), None);
        assert_eq!(date_from_stem("201603"), None);
    }

    #[test]
    fn is_image_path_accepts_decodable_extensions_only() {
        assert!(is_image_path(Path::new("a/b.JPG")));
        assert!(is_image_path(Path::new("a/b.webp")));
        assert!(!is_image_path(Path::new("a/b.bmp")));
        assert!(!is_image_path(Path::new("a/b.txt")));
        assert!(!is_image_path(Path::new("a/b")));
    }

    #[test]
    fn path_key_normalises_separators_and_case() {
        assert_eq!(
            path_key(Path::new("D:\\bing_pic\\20160323.JPG")),
            "d:/bing_pic/20160323.jpg"
        );
    }

    #[test]
    fn dedupe_keeps_one_entry_per_date_and_the_first_seen_wins() {
        // list_candidates sorts newest-first, so the newest sibling of 2020-08-31
        // is the one that survives.
        let files = vec![
            scanned("2020-08-31", "bing20200831.jpg", 947_814, 200),
            scanned("2020-08-31", "20200831.jpg", 271_373, 100),
            scanned("2016-03-23", "20160323.jpg", 350_472, 90),
        ];

        let kept = dedupe(files);

        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].bytes, 947_814);
        assert_eq!(kept[0].path.file_name().unwrap(), "bing20200831.jpg");
        let dates: Vec<&str> = kept.iter().map(|file| file.date.as_str()).collect();
        assert_eq!(dates, vec!["2020-08-31", "2016-03-23"]);
    }

    #[test]
    fn dedupe_dates_unnamed_pictures_by_their_own_mtime() {
        let unnamed = scanned("", "holiday.jpg", 10, 1_700_000_000);

        let kept = dedupe(vec![unnamed]);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].date, date_from_timestamp(1_700_000_000));
    }

    #[test]
    fn plan_imports_a_date_the_library_does_not_have() {
        let file = scanned("2016-03-23", "20160323.jpg", 350_472, 1);

        match plan_file(&file, &HashMap::new(), "2026-11-03T00:00:00+08:00") {
            Plan::Import(image) => {
                assert_eq!(image.date, "2016-03-23");
                assert_eq!(image.file_path, "D:/bing_pic/20160323.jpg");
                assert_eq!(image.bytes, 350_472);
                assert_eq!(image.created_at, "2026-11-03T00:00:00+08:00");
                let row = NewImage::from(&image);
                assert!(row.external);
                assert_eq!(row.url, "");
            }
            other => panic!("expected import, got {other:?}"),
        }
    }

    #[test]
    fn plan_skips_a_sibling_while_the_indexed_file_still_exists() {
        // The indexed sibling must really exist on every platform, so build it in
        // a temp dir instead of pointing at an OS file that only answers to `is_file`
        // on Windows.
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("bing20261102.jpg");
        std::fs::write(&existing, b"jpg").unwrap();

        let file = scanned("2026-11-02", "bing20261102-1.jpg", 1, 1);
        let indexed = HashMap::from([(
            "2026-11-02".to_string(),
            indexed_row("2026-11-02", &existing.to_string_lossy()),
        )]);

        assert_eq!(plan_file(&file, &indexed, "now"), Plan::Skip);
    }

    #[test]
    fn plan_skips_the_path_that_is_already_indexed() {
        let file = scanned("2016-03-23", "20160323.jpg", 1, 1);
        let indexed = HashMap::from([(
            "2016-03-23".to_string(),
            indexed_row("2016-03-23", "D:\\bing_pic\\20160323.jpg"),
        )]);

        assert_eq!(plan_file(&file, &indexed, "now"), Plan::Skip);
    }

    #[test]
    fn plan_relinks_when_the_recorded_file_is_gone() {
        let file = scanned("2016-03-24", "20160324.jpg", 341, 1);
        let indexed = HashMap::from([(
            "2016-03-24".to_string(),
            indexed_row("2016-03-24", "D:\\deleted\\20160324.jpg"),
        )]);

        match plan_file(&file, &indexed, "now") {
            Plan::Relink(image) => assert_eq!(image.file_path, "D:/bing_pic/20160324.jpg"),
            other => panic!("expected relink, got {other:?}"),
        }
    }

    #[test]
    fn list_candidates_reads_a_real_directory_and_skips_tombstones() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("20160323.jpg"), b"x").unwrap();
        std::fs::write(root.path().join("bing20261102-1.jpg"), b"yy").unwrap();
        std::fs::write(root.path().join("notes.txt"), b"ignore me").unwrap();
        std::fs::create_dir(root.path().join("sub")).unwrap();

        let removed = HashSet::from([path_key(&root.path().join("20160323.jpg"))]);
        let files = list_candidates(root.path(), &removed);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].date, "2026-11-02");
        assert_eq!(files[0].bytes, 2);
    }
}
