use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::{AppError, AppResult};

pub fn ensure_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path)
        .map_err(|error| AppError::Io(format!("无法创建目录 {}：{error}", path.display())))
}

pub fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

pub fn exists(path: &Path) -> bool {
    path.is_file()
}

pub fn remove_file(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(format!(
            "无法删除 {}：{error}",
            path.display()
        ))),
    }
}

/// Write a file by writing to a sibling temp file first, then renaming it into
/// place. This avoids leaving a half-written JSON file behind on crash.
pub fn atomic_write(path: &Path, contents: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("dat")
    ));
    {
        let mut file = fs::File::create(&tmp).map_err(|error| {
            AppError::Io(format!("无法写入 {}：{error}", tmp.display()))
        })?;
        file.write_all(contents).map_err(|error| {
            AppError::Io(format!("无法写入 {}：{error}", tmp.display()))
        })?;
        file.sync_all().ok();
    }
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        AppError::Io(format!("无法保存 {}：{error}", path.display()))
    })
}

/// Pick a unique file name so a force re-download never clobbers an image that
/// is currently applied as the desktop background.
pub fn unique_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }
    for index in 1..1000 {
        let candidate = dir.join(format!("{stem}-{index}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-{}.{extension}", chrono::Local::now().timestamp()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_avoids_collisions_and_stays_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        let first = unique_path(base, "bing20261102", "jpg");
        assert_eq!(first.file_name().unwrap(), "bing20261102.jpg");
        std::fs::write(&first, b"x").unwrap();

        let second = unique_path(base, "bing20261102", "jpg");
        assert_eq!(second.file_name().unwrap(), "bing20261102-1.jpg");
        std::fs::write(&second, b"x").unwrap();

        let third = unique_path(base, "bing20261102", "jpg");
        assert_eq!(third.file_name().unwrap(), "bing20261102-2.jpg");

        for path in [&first, &second, &third] {
            assert_eq!(path.parent().unwrap(), base);
        }
    }

    #[test]
    fn atomic_write_creates_parent_and_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("settings.json");

        atomic_write(&target, b"{\"ok\":true}").unwrap();

        assert!(target.is_file());
        assert_eq!(std::fs::read(&target).unwrap(), b"{\"ok\":true}");
        assert!(!target.with_extension("json.tmp").exists());
    }

    #[test]
    fn remove_file_is_idempotent_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.jpg");
        assert!(remove_file(&missing).is_ok());

        let existing = dir.path().join("real.jpg");
        std::fs::write(&existing, b"data").unwrap();
        assert!(exists(&existing));
        assert_eq!(file_size(&existing), 4);

        remove_file(&existing).unwrap();
        assert!(!exists(&existing));
    }
}
