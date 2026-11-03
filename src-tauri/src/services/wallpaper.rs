use std::path::Path;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{ImageMeta, SetWallpaperResult, WallpaperStyle};
use crate::domain::traits::WallpaperBackend;
use crate::infra::db::Db;

pub struct WallpaperService<'a> {
    pub backend: &'a dyn WallpaperBackend,
    pub db: &'a Db,
}

impl<'a> WallpaperService<'a> {
    pub fn new(backend: &'a dyn WallpaperBackend, db: &'a Db) -> Self {
        Self { backend, db }
    }

    pub fn apply_meta(
        &self,
        image: &ImageMeta,
        style: WallpaperStyle,
    ) -> AppResult<SetWallpaperResult> {
        let path = Path::new(&image.file_path);
        if !path.is_file() {
            return Err(AppError::Io(format!(
                "壁纸文件不存在：{}",
                image.file_path
            )));
        }
        self.backend.set_wallpaper(path, style)?;
        Ok(SetWallpaperResult {
            success: true,
            message: "壁纸已更新".to_string(),
            path: image.file_path.clone(),
            date: image.date.clone(),
        })
    }

    pub fn apply_path(&self, path: &Path, style: WallpaperStyle) -> AppResult<()> {
        self.backend.set_wallpaper(path, style)
    }

    /// Resolve the image for a date and apply it.
    pub fn apply_date(
        &self,
        date: &str,
        style: WallpaperStyle,
    ) -> AppResult<SetWallpaperResult> {
        let image = self
            .db
            .get_by_date(date)?
            .ok_or_else(|| AppError::InvalidInput(format!("本地没有 {date} 的壁纸")))?;
        self.apply_meta(&image, style)
    }
}
