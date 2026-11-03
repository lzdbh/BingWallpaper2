use std::collections::HashSet;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use rusqlite::types::Value;

use crate::domain::{
    AppError, AppResult, ImageFilter, ImageMeta, Page, StorageStats,
};

const COLUMNS: &str = "id, date, title, copyright, copyright_link, file_path, thumb_path, \
                       width, height, bytes, favorite, external, created_at";

pub struct NewImage<'a> {
    pub date: &'a str,
    pub title: &'a str,
    pub copyright: &'a str,
    pub copyright_link: &'a str,
    pub url: &'a str,
    pub file_path: &'a str,
    pub thumb_path: &'a str,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub created_at: &'a str,
    /// 1 when the file was found in a user-selected directory; the app must
    /// then never delete it from disk.
    pub external: bool,
}

/// One indexed image, light enough to load for the whole library.
pub struct IndexedRow {
    pub date: String,
    pub file_path: String,
}

/// A row the thumbnail worker still has to (re)build.
pub struct ThumbRow {
    pub id: i64,
    pub date: String,
    pub file_path: String,
    pub thumb_path: String,
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            super::fs::ensure_dir(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::Database("数据库锁已损坏".into()))
    }

    /// In-memory database used by unit tests; migrated like a real one.
    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS images (
                 id             INTEGER PRIMARY KEY AUTOINCREMENT,
                 date           TEXT NOT NULL UNIQUE,
                 title          TEXT NOT NULL DEFAULT '',
                 copyright      TEXT NOT NULL DEFAULT '',
                 copyright_link TEXT NOT NULL DEFAULT '',
                 url            TEXT NOT NULL DEFAULT '',
                 file_path      TEXT NOT NULL,
                 thumb_path     TEXT NOT NULL DEFAULT '',
                 width          INTEGER NOT NULL DEFAULT 0,
                 height         INTEGER NOT NULL DEFAULT 0,
                 bytes          INTEGER NOT NULL DEFAULT 0,
                 favorite       INTEGER NOT NULL DEFAULT 0,
                 external       INTEGER NOT NULL DEFAULT 0,
                 created_at     TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_images_date ON images(date DESC);
             CREATE INDEX IF NOT EXISTS idx_images_favorite ON images(favorite);

             CREATE TABLE IF NOT EXISTS task_runs (
                 id          INTEGER PRIMARY KEY AUTOINCREMENT,
                 kind        TEXT NOT NULL,
                 status      TEXT NOT NULL,
                 message     TEXT NOT NULL DEFAULT '',
                 started_at  TEXT NOT NULL,
                 finished_at TEXT
             );

             CREATE TABLE IF NOT EXISTS removed_paths (
                 path TEXT PRIMARY KEY
             );",
        )?;

        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 2 {
            // Databases created before directory scanning have no `external` flag.
            if !column_exists(&conn, "images", "external")? {
                conn.execute_batch(
                    "ALTER TABLE images ADD COLUMN external INTEGER NOT NULL DEFAULT 0;",
                )?;
            }
            conn.execute_batch("PRAGMA user_version = 2;")?;
        }
        Ok(())
    }

    pub fn upsert(&self, image: &NewImage<'_>) -> AppResult<ImageMeta> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO images
                 (date, title, copyright, copyright_link, url, file_path, thumb_path,
                  width, height, bytes, favorite, external, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12)
             ON CONFLICT(date) DO UPDATE SET
                 title = excluded.title,
                 copyright = excluded.copyright,
                 copyright_link = excluded.copyright_link,
                 url = excluded.url,
                 file_path = excluded.file_path,
                 thumb_path = excluded.thumb_path,
                 width = excluded.width,
                 height = excluded.height,
                 bytes = excluded.bytes,
                 external = excluded.external,
                 created_at = excluded.created_at",
            params![
                image.date,
                image.title,
                image.copyright,
                image.copyright_link,
                image.url,
                image.file_path,
                image.thumb_path,
                image.width as i64,
                image.height as i64,
                image.bytes as i64,
                image.external as i64,
                image.created_at,
            ],
        )?;
        conn.query_row(
            &format!("SELECT {COLUMNS} FROM images WHERE date = ?1"),
            params![image.date],
            row_to_meta,
        )
        .optional()?
        .ok_or_else(|| AppError::Database("写入图片记录后未能读回".into()))
    }

    /// Every indexed date with the path recorded for it, used by the scanner to
    /// decide between import, relink and skip in one pass.
    pub fn indexed_rows(&self) -> AppResult<Vec<IndexedRow>> {
        let conn = self.lock()?;
        let mut statement = conn.prepare("SELECT date, file_path FROM images")?;
        let rows = statement.query_map([], |row| {
            Ok(IndexedRow {
                date: row.get(0)?,
                file_path: row.get(1)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// Paths the user removed from the gallery while their file stayed on disk.
    pub fn removed_paths(&self) -> AppResult<HashSet<String>> {
        let conn = self.lock()?;
        let mut statement = conn.prepare("SELECT path FROM removed_paths")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut items = HashSet::new();
        for row in rows {
            items.insert(row?);
        }
        Ok(items)
    }

    /// Insert scanned files in one transaction, ignoring dates that raced ahead.
    pub fn insert_scanned(&self, images: &[NewImage<'_>]) -> AppResult<u32> {
        if images.is_empty() {
            return Ok(0);
        }
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let inserted = {
            let mut statement = tx.prepare(
                "INSERT INTO images
                     (date, title, copyright, copyright_link, url, file_path, thumb_path,
                      width, height, bytes, favorite, external, created_at)
                 VALUES (?1, ?2, ?3, ?4, '', ?5, '', ?6, ?7, ?8, 0, 1, ?9)
                 ON CONFLICT(date) DO NOTHING",
            )?;
            let mut inserted = 0u32;
            for image in images {
                inserted += statement.execute(params![
                    image.date,
                    image.title,
                    image.copyright,
                    image.copyright_link,
                    image.file_path,
                    image.width as i64,
                    image.height as i64,
                    image.bytes as i64,
                    image.created_at,
                ])? as u32;
            }
            inserted
        };
        tx.commit()?;
        Ok(inserted)
    }

    /// Point a row whose recorded file has vanished at a scanned replacement and
    /// drop the now-invalid thumbnail.
    pub fn relink_scanned(&self, date: &str, image: &NewImage<'_>) -> AppResult<bool> {
        let conn = self.lock()?;
        let updated = conn.execute(
            "UPDATE images
                SET file_path = ?2, thumb_path = '', width = ?3, height = ?4,
                    bytes = ?5, external = 1
              WHERE date = ?1",
            params![date, image.file_path, image.width as i64, image.height as i64, image.bytes as i64],
        )?;
        Ok(updated > 0)
    }

    /// Rows whose thumbnail has to be (re)built.
    pub fn thumb_rows(&self) -> AppResult<Vec<ThumbRow>> {
        let conn = self.lock()?;
        let mut statement = conn.prepare(
            "SELECT id, date, file_path, thumb_path FROM images ORDER BY date DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ThumbRow {
                id: row.get::<_, i64>(0)?,
                date: row.get(1)?,
                file_path: row.get(2)?,
                thumb_path: row.get(3)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// Record freshly built thumbnails in one transaction; the worker batches
    /// these so a full-library pass costs a handful of fsyncs, not thousands.
    pub fn set_thumbs(&self, built: &[(i64, String)]) -> AppResult<()> {
        if built.is_empty() {
            return Ok(());
        }
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        {
            let mut statement = tx.prepare("UPDATE images SET thumb_path = ?2 WHERE id = ?1")?;
            for (id, thumb_path) in built {
                statement.execute(params![id, thumb_path])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Forget a scanned image without deleting the user's file, and remember the
    /// path so the next scan does not re-import it.
    pub fn forget(&self, id: i64, file_path: &str) -> AppResult<bool> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let deleted = tx.execute("DELETE FROM images WHERE id = ?1", params![id])? > 0;
        if deleted {
            tx.execute(
                "INSERT INTO removed_paths (path) VALUES (?1) ON CONFLICT(path) DO NOTHING",
                params![file_path],
            )?;
        }
        tx.commit()?;
        Ok(deleted)
    }

    pub fn get_by_date(&self, date: &str) -> AppResult<Option<ImageMeta>> {
        let conn = self.lock()?;
        Ok(conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM images WHERE date = ?1"),
                params![date],
                row_to_meta,
            )
            .optional()?)
    }

    pub fn get_by_id(&self, id: i64) -> AppResult<Option<ImageMeta>> {
        let conn = self.lock()?;
        Ok(conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM images WHERE id = ?1"),
                params![id],
                row_to_meta,
            )
            .optional()?)
    }

    pub fn latest(&self) -> AppResult<Option<ImageMeta>> {
        let conn = self.lock()?;
        Ok(conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM images ORDER BY date DESC LIMIT 1"),
                [],
                row_to_meta,
            )
            .optional()?)
    }

    pub fn list(
        &self,
        page: u32,
        page_size: u32,
        filter: ImageFilter,
        query: Option<&str>,
    ) -> AppResult<Page<ImageMeta>> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let conn = self.lock()?;

        let mut clauses: Vec<String> = Vec::new();
        let mut values: Vec<Value> = Vec::new();

        match filter {
            ImageFilter::Favorites => clauses.push("favorite = 1".to_string()),
            ImageFilter::Recent => {
                let cutoff = (chrono::Local::now() - chrono::Duration::days(6))
                    .format("%Y-%m-%d")
                    .to_string();
                clauses.push("date >= ?".to_string());
                values.push(Value::Text(cutoff));
            }
            ImageFilter::All => {}
        }

        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            clauses.push("(title LIKE ? OR copyright LIKE ? OR date LIKE ?)".to_string());
            let like = format!("%{query}%");
            values.push(Value::Text(like.clone()));
            values.push(Value::Text(like.clone()));
            values.push(Value::Text(like));
        }

        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };

        let total: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM images{where_sql}"),
            params_from_iter(values.iter()),
            |row| row.get(0),
        )?;

        let mut list_values = values;
        list_values.push(Value::Integer(page_size as i64));
        list_values.push(Value::Integer(((page - 1) * page_size) as i64));

        let mut statement = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM images{where_sql} ORDER BY date DESC LIMIT ? OFFSET ?"
        ))?;
        let rows = statement.query_map(params_from_iter(list_values.iter()), row_to_meta)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        Ok(Page {
            items,
            total: total.max(0) as u32,
            page,
            page_size,
        })
    }

    pub fn set_favorite(&self, id: i64, favorite: bool) -> AppResult<Option<ImageMeta>> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE images SET favorite = ?1 WHERE id = ?2",
            params![if favorite { 1 } else { 0 }, id],
        )?;
        Ok(conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM images WHERE id = ?1"),
                params![id],
                row_to_meta,
            )
            .optional()?)
    }

    pub fn delete(&self, id: i64) -> AppResult<Option<ImageMeta>> {
        let conn = self.lock()?;
        let image = conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM images WHERE id = ?1"),
                params![id],
                row_to_meta,
            )
            .optional()?;
        if image.is_some() {
            conn.execute("DELETE FROM images WHERE id = ?1", params![id])?;
        }
        Ok(image)
    }

    pub fn stats(&self) -> AppResult<StorageStats> {
        let conn = self.lock()?;
        let (count, favorite_count, bytes): (i64, i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(favorite), 0), COALESCE(SUM(bytes), 0) FROM images",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        Ok(StorageStats {
            count: count.max(0) as u32,
            favorite_count: favorite_count.max(0) as u32,
            bytes: bytes.max(0) as u64,
        })
    }

    /// Non-favorite, app-managed images older than the given date, used by cache
    /// cleanup. Scanned `external` rows stay out of reach so cleanup can never
    /// delete a file the user keeps in their own directory.
    pub fn older_than(&self, cutoff: &str) -> AppResult<Vec<ImageMeta>> {
        let conn = self.lock()?;
        let mut statement = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM images
              WHERE favorite = 0 AND external = 0 AND date < ?1 ORDER BY date ASC"
        ))?;
        let rows = statement.query_map(params![cutoff], row_to_meta)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn record_task(
        &self,
        kind: &str,
        status: &str,
        message: &str,
        started_at: &str,
        finished_at: &str,
    ) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO task_runs (kind, status, message, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind, status, message, started_at, finished_at],
        )?;
        Ok(())
    }

    pub fn last_successful_run(&self, kind: &str) -> AppResult<Option<String>> {
        let conn = self.lock()?;
        Ok(conn
            .query_row(
                "SELECT started_at FROM task_runs
                 WHERE kind = ?1 AND status = 'success'
                 ORDER BY id DESC LIMIT 1",
                params![kind],
                |row| row.get::<_, String>(0),
            )
            .optional()?)
    }
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImageMeta> {
    let file_path: String = row.get("file_path")?;
    let downloaded = super::fs::exists(Path::new(&file_path));
    Ok(ImageMeta {
        id: row.get("id")?,
        date: row.get("date")?,
        title: row.get("title")?,
        copyright: row.get("copyright")?,
        copyright_link: row.get("copyright_link")?,
        thumb_path: row.get("thumb_path")?,
        width: row.get::<_, i64>("width")?.max(0) as u32,
        height: row.get::<_, i64>("height")?.max(0) as u32,
        bytes: row.get::<_, i64>("bytes")?.max(0) as u64,
        favorite: row.get::<_, i64>("favorite")? != 0,
        external: row.get::<_, i64>("external")? != 0,
        created_at: row.get("created_at")?,
        downloaded,
        file_path,
    })
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    while let Some(name) = rows.next().transpose()? {
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_image<'a>(date: &'a str, title: &'a str, bytes: u64) -> NewImage<'a> {
        NewImage {
            date,
            title,
            copyright: "© Test",
            copyright_link: "https://example.com",
            url: "https://www.bing.com/x.jpg",
            file_path: "images/x.jpg",
            thumb_path: "",
            width: 1920,
            height: 1080,
            bytes,
            created_at: "2026-11-02T00:00:00+08:00",
            external: false,
        }
    }

    fn scanned<'a>(date: &'a str, file_path: &'a str, bytes: u64) -> NewImage<'a> {
        NewImage {
            date,
            title: "",
            copyright: "",
            copyright_link: "",
            url: "",
            file_path,
            thumb_path: "",
            width: 1920,
            height: 1080,
            bytes,
            created_at: "2026-11-02T00:00:00+08:00",
            external: true,
        }
    }

    #[test]
    fn upsert_inserts_then_updates_same_date() {
        let db = Db::open_in_memory().unwrap();

        let meta = db.upsert(&new_image("2026-11-01", "First", 100)).unwrap();
        assert_eq!(meta.date, "2026-11-01");
        assert_eq!(meta.title, "First");
        assert!(!meta.favorite);

        // The UNIQUE(date) constraint means the same date updates in place.
        let updated = db.upsert(&new_image("2026-11-01", "Second", 200)).unwrap();
        assert_eq!(updated.id, meta.id);
        assert_eq!(updated.title, "Second");
        assert_eq!(updated.bytes, 200);

        assert_eq!(db.stats().unwrap().count, 1);
    }

    #[test]
    fn get_by_date_and_id() {
        let db = Db::open_in_memory().unwrap();
        let meta = db.upsert(&new_image("2026-11-01", "A", 10)).unwrap();

        assert_eq!(db.get_by_date("2026-11-01").unwrap().unwrap().id, meta.id);
        assert!(db.get_by_date("1999-01-01").unwrap().is_none());
        assert_eq!(db.get_by_id(meta.id).unwrap().unwrap().date, "2026-11-01");
        assert!(db.get_by_id(meta.id + 999).unwrap().is_none());
    }

    #[test]
    fn latest_returns_newest_by_date() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&new_image("2026-11-01", "Old", 1)).unwrap();
        db.upsert(&new_image("2026-11-05", "New", 1)).unwrap();
        db.upsert(&new_image("2026-11-03", "Mid", 1)).unwrap();
        assert_eq!(db.latest().unwrap().unwrap().date, "2026-11-05");
    }

    #[test]
    fn list_paginates_in_date_desc_order() {
        let db = Db::open_in_memory().unwrap();
        for day in 1..=5 {
            let date = format!("2026-11-0{day}");
            db.upsert(&new_image(&date, "T", 10)).unwrap();
        }

        let page1 = db.list(1, 2, ImageFilter::All, None).unwrap();
        assert_eq!(page1.total, 5);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].date, "2026-11-05");
        assert_eq!(page1.items[1].date, "2026-11-04");

        let page3 = db.list(3, 2, ImageFilter::All, None).unwrap();
        assert_eq!(page3.items.len(), 1);
        assert_eq!(page3.items[0].date, "2026-11-01");

        // page_size is clamped to a minimum of 1.
        assert_eq!(db.list(1, 0, ImageFilter::All, None).unwrap().page_size, 1);
    }

    #[test]
    fn list_search_query_matches_title() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&new_image("2026-11-01", "Aurora Sky", 1)).unwrap();
        db.upsert(&new_image("2026-11-02", "Mountain", 1)).unwrap();

        let found = db.list(1, 10, ImageFilter::All, Some("aurora")).unwrap();
        assert_eq!(found.total, 1);
        assert_eq!(found.items[0].title, "Aurora Sky");
    }

    #[test]
    fn favorites_roundtrip_and_filter() {
        let db = Db::open_in_memory().unwrap();
        let a = db.upsert(&new_image("2026-11-01", "A", 1)).unwrap();
        db.upsert(&new_image("2026-11-02", "B", 1)).unwrap();

        let fav = db.set_favorite(a.id, true).unwrap().unwrap();
        assert!(fav.favorite);
        assert_eq!(db.stats().unwrap().favorite_count, 1);

        let favorites = db.list(1, 10, ImageFilter::Favorites, None).unwrap();
        assert_eq!(favorites.total, 1);
        assert_eq!(favorites.items[0].id, a.id);

        db.set_favorite(a.id, false).unwrap();
        let favorites = db.list(1, 10, ImageFilter::Favorites, None).unwrap();
        assert_eq!(favorites.total, 0);
    }

    #[test]
    fn delete_removes_row_and_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        let meta = db.upsert(&new_image("2026-11-01", "A", 1)).unwrap();

        let removed = db.delete(meta.id).unwrap();
        assert_eq!(removed.unwrap().id, meta.id);
        assert!(db.get_by_id(meta.id).unwrap().is_none());
        assert!(db.delete(meta.id).unwrap().is_none());
    }

    #[test]
    fn stats_sums_bytes_and_counts() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&new_image("2026-11-01", "A", 100)).unwrap();
        let b = db.upsert(&new_image("2026-11-02", "B", 250)).unwrap();
        db.set_favorite(b.id, true).unwrap();

        let stats = db.stats().unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.favorite_count, 1);
        assert_eq!(stats.bytes, 350);
    }

    #[test]
    fn older_than_excludes_favorites() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&new_image("2026-01-01", "Old", 1)).unwrap();
        let fav = db.upsert(&new_image("2026-02-01", "Fav", 1)).unwrap();
        db.set_favorite(fav.id, true).unwrap();

        let old = db.older_than("2026-06-01").unwrap();
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].date, "2026-01-01");
    }

    #[test]
    fn task_runs_record_and_query_last_success() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.last_successful_run("daily_set").unwrap().is_none());

        db.record_task(
            "daily_set",
            "failed",
            "boom",
            "2026-11-01T00:05:00+08:00",
            "2026-11-01T00:05:01+08:00",
        )
        .unwrap();
        // A failure does not count as a successful run.
        assert!(db.last_successful_run("daily_set").unwrap().is_none());

        db.record_task(
            "daily_set",
            "success",
            "ok",
            "2026-11-02T00:05:00+08:00",
            "2026-11-02T00:05:02+08:00",
        )
        .unwrap();
        let last = db.last_successful_run("daily_set").unwrap().unwrap();
        assert!(last.starts_with("2026-11-02"));
    }

    #[test]
    fn insert_scanned_stores_external_rows_and_skips_known_dates() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&new_image("2016-03-23", "Downloaded", 100)).unwrap();

        let inserted = db
            .insert_scanned(&[
                scanned("2016-03-23", "D:/bing_pic/20160323.jpg", 350),
                scanned("2016-03-24", "D:/bing_pic/20160324.jpg", 341),
            ])
            .unwrap();

        // The date already covered by a download is left alone.
        assert_eq!(inserted, 1);
        let existing = db.get_by_date("2016-03-23").unwrap().unwrap();
        assert_eq!(existing.file_path, "images/x.jpg");
        assert!(!existing.external);

        let imported = db.get_by_date("2016-03-24").unwrap().unwrap();
        assert!(imported.external);
        assert_eq!(imported.file_path, "D:/bing_pic/20160324.jpg");
        assert_eq!(imported.thumb_path, "");
    }

    #[test]
    fn relink_repoints_missing_file_and_keeps_favorite() {
        let db = Db::open_in_memory().unwrap();
        let meta = db.upsert(&new_image("2016-03-23", "Old", 100)).unwrap();
        db.set_favorite(meta.id, true).unwrap();

        let updated = db
            .relink_scanned("2016-03-23", &scanned("2016-03-23", "D:/bing_pic/20160323.jpg", 350))
            .unwrap();

        assert!(updated);
        let relinked = db.get_by_date("2016-03-23").unwrap().unwrap();
        assert_eq!(relinked.file_path, "D:/bing_pic/20160323.jpg");
        assert_eq!(relinked.thumb_path, "");
        assert!(relinked.external);
        assert!(relinked.favorite);
        assert!(!db
            .relink_scanned("1999-01-01", &scanned("1999-01-01", "x", 1))
            .unwrap());
    }

    #[test]
    fn upsert_reclaims_an_external_row_for_the_app_copy() {
        let db = Db::open_in_memory().unwrap();
        db.insert_scanned(&[scanned("2026-11-02", "D:/bing_pic/20261102.jpg", 1)])
            .unwrap();

        db.upsert(&new_image("2026-11-02", "Bing", 100)).unwrap();

        let meta = db.get_by_date("2026-11-02").unwrap().unwrap();
        assert!(!meta.external);
        assert_eq!(meta.file_path, "images/x.jpg");
    }

    #[test]
    fn forget_keeps_the_tombstone_and_cleanup_skips_external_rows() {
        let db = Db::open_in_memory().unwrap();
        db.insert_scanned(&[scanned("2016-03-23", "D:/bing_pic/20160323.jpg", 350)])
            .unwrap();
        db.insert_scanned(&[scanned("2016-03-24", "D:/bing_pic/20160324.jpg", 341)])
            .unwrap();
        let managed = db.upsert(&new_image("2016-03-25", "Bing", 1)).unwrap();

        // Cleanup only ever sees the app-managed row.
        let candidates = db.older_than("2026-01-01").unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, managed.id);
        assert!(candidates.iter().all(|image| !image.external));

        let removed = db.get_by_date("2016-03-23").unwrap().unwrap();
        assert!(db.forget(removed.id, &removed.file_path).unwrap());
        assert!(db.get_by_date("2016-03-23").unwrap().is_none());
        assert!(db
            .removed_paths()
            .unwrap()
            .contains("D:/bing_pic/20160323.jpg"));
        // Idempotent: an unknown id forgets nothing.
        assert!(!db.forget(removed.id, "D:/bing_pic/20160323.jpg").unwrap());
    }

    #[test]
    fn thumb_rows_cover_every_image_newest_first() {
        let db = Db::open_in_memory().unwrap();
        db.insert_scanned(&[scanned("2016-03-23", "D:/a.jpg", 1)]).unwrap();
        db.upsert(&NewImage {
            thumb_path: "D:/thumb.jpg",
            ..new_image("2026-11-02", "Bing", 1)
        })
        .unwrap();

        let rows = db.thumb_rows().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2026-11-02");
        assert_eq!(rows[0].thumb_path, "D:/thumb.jpg");
        assert_eq!(rows[1].thumb_path, "");
    }

    #[test]
    fn set_thumbs_records_generated_paths() {
        let db = Db::open_in_memory().unwrap();
        db.insert_scanned(&[scanned("2016-03-23", "D:/a.jpg", 1)])
            .unwrap();
        db.insert_scanned(&[scanned("2016-03-24", "D:/b.jpg", 1)])
            .unwrap();

        let built: Vec<(i64, String)> = db
            .thumb_rows()
            .unwrap()
            .into_iter()
            .map(|row| (row.id, format!("thumbs/{}.jpg", row.date)))
            .collect();
        db.set_thumbs(&built).unwrap();

        assert_eq!(
            db.get_by_date("2016-03-23").unwrap().unwrap().thumb_path,
            "thumbs/2016-03-23.jpg"
        );
        assert_eq!(db.thumb_rows().unwrap().iter().filter(|row| row.thumb_path.is_empty()).count(), 0);
    }

    #[test]
    fn indexed_rows_cover_the_whole_library() {
        let db = Db::open_in_memory().unwrap();
        db.upsert(&new_image("2026-11-02", "Bing", 1)).unwrap();
        db.insert_scanned(&[scanned("2016-03-23", "D:/a.jpg", 1)]).unwrap();

        let rows = db.indexed_rows().unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.date == "2016-03-23"));
    }
}
