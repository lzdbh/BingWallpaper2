use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{ImageMeta, ImageSize};
use crate::infra::db::{Db, NewImage};
use crate::infra::fs;
use crate::infra::http::HttpClient;
use crate::services::bing::{compact_date, BingClient, BingImage};
use crate::services::thumbnail;

pub struct LibraryService<'a> {
    pub http: &'a HttpClient,
    pub db: &'a Db,
    pub images_dir: PathBuf,
    pub thumbs_dir: PathBuf,
}

impl<'a> LibraryService<'a> {
    pub fn new(
        http: &'a HttpClient,
        db: &'a Db,
        images_dir: PathBuf,
        thumbs_dir: PathBuf,
    ) -> Self {
        Self {
            http,
            db,
            images_dir,
            thumbs_dir,
        }
    }

    /// Ensure a given date exists locally, downloading it when necessary.
    pub async fn ensure(
        &self,
        date: &str,
        force: bool,
        size: ImageSize,
        market: &str,
    ) -> AppResult<ImageMeta> {
        if !force {
            if let Some(meta) = self.db.get_by_date(date)? {
                if Path::new(&meta.file_path).is_file() {
                    return Ok(meta);
                }
            }
        }

        let client = BingClient::new(self.http);
        let list = client.fetch(market).await?;
        let image = list
            .into_iter()
            .find(|item| item.date == date)
            .ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "Bing 只提供最近 8 天的图片，{date} 已超出可下载范围"
                ))
            })?;

        self.download_one(&image, size).await
    }

    /// Ensure the newest Bing image exists locally.
    pub async fn latest(
        &self,
        force: bool,
        size: ImageSize,
        market: &str,
    ) -> AppResult<ImageMeta> {
        // Offline-friendly fast path: today's image is already on disk.
        if !force {
            let today = crate::services::bing::today_string();
            if let Some(meta) = self.db.get_by_date(&today)? {
                if Path::new(&meta.file_path).is_file() {
                    return Ok(meta);
                }
            }
        }

        let client = BingClient::new(self.http);
        let image = client.fetch_latest(market).await?;

        if !force {
            if let Some(meta) = self.db.get_by_date(&image.date)? {
                if Path::new(&meta.file_path).is_file() {
                    return Ok(meta);
                }
            }
        }

        self.download_one(&image, size).await
    }

    /// Download a known Bing image and record it in the database.
    pub async fn download_one(
        &self,
        image: &BingImage,
        size: ImageSize,
    ) -> AppResult<ImageMeta> {
        let url = crate::services::bing::image_url(image, size);
        log::info!("下载壁纸 {}：{url}", image.date);
        let bytes = self.http.get_bytes(&url).await?;
        if bytes.len() < 1024 {
            return Err(AppError::Bing("下载到的图片数据过小".into()));
        }

        let (width, height) = read_dimensions(&bytes)?;

        fs::ensure_dir(&self.images_dir)?;
        fs::ensure_dir(&self.thumbs_dir)?;

        let stem = format!("bing{}", compact_date(&image.date));
        let file_path = fs::unique_path(&self.images_dir, &stem, "jpg");
        std::fs::write(&file_path, &bytes).map_err(|error| {
            AppError::Io(format!("无法保存图片 {}：{error}", file_path.display()))
        })?;

        let thumb_path = self.thumbs_dir.join(format!("{}.jpg", image.date));
        if let Err(error) = thumbnail::generate(&file_path, &thumb_path, 960) {
            log::warn!("生成缩略图失败：{error}");
        }
        let thumb = if thumb_path.is_file() {
            thumb_path.to_string_lossy().to_string()
        } else {
            String::new()
        };

        let previous = self.db.get_by_date(&image.date)?;
        let created_at = chrono::Local::now().to_rfc3339();
        let file_path_str = file_path.to_string_lossy().to_string();
        let new_image = NewImage {
            date: &image.date,
            title: &image.title,
            copyright: &image.copyright,
            copyright_link: &image.copyright_link,
            url: &url,
            file_path: &file_path_str,
            thumb_path: &thumb,
            width,
            height,
            bytes: bytes.len() as u64,
            created_at: &created_at,
        };

        let meta = match self.db.upsert(&new_image) {
            Ok(meta) => meta,
            Err(error) => {
                let _ = fs::remove_file(&file_path);
                return Err(error);
            }
        };

        if let Some(previous) = previous {
            if previous.file_path != meta.file_path {
                let _ = fs::remove_file(Path::new(&previous.file_path));
            }
        }

        Ok(meta)
    }

    /// Delete the local files and database row for an image.
    pub fn remove(&self, id: i64) -> AppResult<Option<ImageMeta>> {
        let removed = self.db.delete(id)?;
        if let Some(image) = &removed {
            let _ = fs::remove_file(Path::new(&image.file_path));
            if !image.thumb_path.is_empty() {
                let _ = fs::remove_file(Path::new(&image.thumb_path));
            }
        }
        Ok(removed)
    }
}

fn read_dimensions(bytes: &[u8]) -> AppResult<(u32, u32)> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| AppError::Io(format!("无法识别图片格式：{error}")))?;
    reader
        .into_dimensions()
        .map_err(|error| AppError::Io(format!("无法读取图片尺寸：{error}")))
}
