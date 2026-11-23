use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{ImageMeta, ImageSize};
use crate::infra::db::{Db, NewImage, UrlRow};
use crate::infra::fs;
use crate::infra::http::HttpClient;
use crate::services::bing::{compact_date, strip_query, BingClient, BingImage};
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
        // The same picture can come back under another date (list re-fetched
        // near a rotation, or Bing repeating a feature). Keep one row per
        // picture so the gallery never shows the same image twice.
        if let Some(older) = self.find_same_image(image)? {
            self.purge_downloaded(&older)?;
        }

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

        let thumb_path = thumbnail::thumb_path_for(&self.thumbs_dir, &image.date);
        let thumb = match thumbnail::generate(&file_path, &thumb_path, thumbnail::THUMB_WIDTH) {
            Ok(()) => thumb_path.to_string_lossy().to_string(),
            Err(error) => {
                log::warn!("生成缩略图失败：{error}");
                String::new()
            }
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
            external: false,
        };

        let meta = match self.db.upsert(&new_image) {
            Ok(meta) => meta,
            Err(error) => {
                let _ = fs::remove_file(&file_path);
                return Err(error);
            }
        };

        // Only discard the copy this app wrote; a scanned row points at a file the
        // user keeps in their own directory.
        if let Some(previous) = previous.filter(|image| !image.external) {
            if previous.file_path != meta.file_path {
                let _ = fs::remove_file(Path::new(&previous.file_path));
            }
        }

        Ok(meta)
    }

    /// Re-key downloaded rows so every picture appears once, labelled with the
    /// local date it was the Bing homepage image. Only the 8 days the API still
    /// serves can be checked; older rows are left as they are.
    pub async fn reconcile(&self, market: &str) -> AppResult<bool> {
        let list = BingClient::new(self.http).fetch(market).await?;
        self.reconcile_list(&list)
    }

    /// The reconciliation half that needs no network, so it can be tested with
    /// a fabricated list.
    fn reconcile_list(&self, list: &[BingImage]) -> AppResult<bool> {
        let rows = self.db.downloaded_rows()?;
        let mut changed = false;

        for image in list {
            let group: Vec<&UrlRow> = rows
                .iter()
                .filter(|row| strip_query(&row.url).ends_with(&image.url_base))
                .collect();
            if group.is_empty() {
                continue;
            }

            let chosen = group
                .iter()
                .find(|row| row.date == image.date)
                .copied()
                .unwrap_or(group[0]);
            for row in group.iter().filter(|row| row.id != chosen.id) {
                self.purge_downloaded(row)?;
                changed = true;
            }
            if chosen.date == image.date {
                continue;
            }

            match self.db.get_by_date(&image.date)? {
                Some(occupant) if occupant.id != chosen.id => {
                    if occupant.external {
                        // Never take a date owned by a scanned file.
                        continue;
                    }
                    self.purge_row(&occupant)?;
                }
                _ => {}
            }

            self.db.set_date(chosen.id, &image.date)?;
            self.retarget_thumb(chosen, &image.date)?;
            changed = true;
        }

        Ok(changed)
    }

    /// The downloaded row holding the same picture under another date.
    fn find_same_image(&self, image: &BingImage) -> AppResult<Option<UrlRow>> {
        Ok(self.db.downloaded_rows()?.into_iter().find(|row| {
            row.date != image.date && strip_query(&row.url).ends_with(&image.url_base)
        }))
    }

    fn purge_downloaded(&self, row: &UrlRow) -> AppResult<()> {
        if self.db.delete(row.id)?.is_some() {
            let _ = fs::remove_file(Path::new(&row.file_path));
            if !row.thumb_path.is_empty() {
                let _ = fs::remove_file(Path::new(&row.thumb_path));
            }
        }
        Ok(())
    }

    fn purge_row(&self, image: &ImageMeta) -> AppResult<()> {
        if self.db.delete(image.id)?.is_none() {
            return Ok(());
        }
        let _ = fs::remove_file(Path::new(&image.file_path));
        if !image.thumb_path.is_empty() {
            let _ = fs::remove_file(Path::new(&image.thumb_path));
        }
        Ok(())
    }

    /// Point a row's cached thumbnail at its (new) date; rebuild from the file
    /// that is already on disk.
    fn retarget_thumb(&self, row: &UrlRow, date: &str) -> AppResult<()> {
        let target = thumbnail::thumb_path_for(&self.thumbs_dir, date);
        if !row.thumb_path.is_empty() && Path::new(&row.thumb_path) != target {
            let _ = fs::remove_file(Path::new(&row.thumb_path));
        }
        let thumb = match thumbnail::generate(
            Path::new(&row.file_path),
            &target,
            thumbnail::THUMB_WIDTH,
        ) {
            Ok(()) => target.to_string_lossy().to_string(),
            Err(error) => {
                log::warn!("生成缩略图失败：{error}");
                String::new()
            }
        };
        self.db.set_thumbs(&[(row.id, thumb)])
    }

    /// Delete the local files and database row for an image. Scanned images are
    /// only un-indexed: their files stay where the user put them, and the path is
    /// tombstoned so the next scan does not bring them back.
    pub fn remove(&self, id: i64) -> AppResult<Option<ImageMeta>> {
        let Some(image) = self.db.get_by_id(id)? else {
            return Ok(None);
        };

        if image.external {
            if !self.db.forget(id, &image.file_path)? {
                return Ok(None);
            }
            if !image.thumb_path.is_empty() {
                let _ = fs::remove_file(Path::new(&image.thumb_path));
            }
            return Ok(Some(image));
        }

        if self.db.delete(id)?.is_none() {
            return Ok(None);
        }
        let _ = fs::remove_file(Path::new(&image.file_path));
        if !image.thumb_path.is_empty() {
            let _ = fs::remove_file(Path::new(&image.thumb_path));
        }
        Ok(Some(image))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::Db;
    use crate::infra::http::HttpClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
        let buffer = image::ImageBuffer::from_fn(width, height, |x, y| {
            image::Rgb([x as u8, y as u8, (x ^ y) as u8])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(buffer)
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
            .unwrap();
        bytes
    }

    fn bing(date: &str, url_base: &str) -> BingImage {
        BingImage {
            date: date.to_string(),
            url_base: url_base.to_string(),
            copyright: "© Test".to_string(),
            copyright_link: "https://example.com".to_string(),
            title: "Test".to_string(),
        }
    }

    fn insert_downloaded(
        db: &Db,
        date: &str,
        url: &str,
        file: &Path,
        thumb: &Path,
    ) {
        let bytes = jpeg_bytes(800, 600);
        std::fs::write(file, &bytes).unwrap();
        std::fs::write(thumb, b"thumb-cache").unwrap();
        let file_str = file.to_string_lossy().to_string();
        let thumb_str = thumb.to_string_lossy().to_string();
        db.upsert(&NewImage {
            date,
            title: "Test",
            copyright: "© Test",
            copyright_link: "https://example.com",
            url,
            file_path: &file_str,
            thumb_path: &thumb_str,
            width: 800,
            height: 600,
            bytes: bytes.len() as u64,
            created_at: "2026-11-20T00:00:00+08:00",
            external: false,
        })
        .unwrap();
    }

    #[test]
    fn reconcile_dedupes_and_moves_rows_to_their_real_dates() {
        let dir = tempfile::tempdir().unwrap();
        let images = dir.path().join("images");
        let thumbs = dir.path().join("thumbs");
        std::fs::create_dir_all(&images).unwrap();
        std::fs::create_dir_all(&thumbs).unwrap();
        let db = Db::open_in_memory().unwrap();
        let http = HttpClient::new(None).unwrap();
        let service = LibraryService::new(&http, &db, images.clone(), thumbs.clone());

        // RedNice filed under both 11-20 (older build) and 11-21.
        insert_downloaded(
            &db,
            "2026-11-20",
            "https://www.bing.com/th?id=OHR.RedNice_1920x1080.jpg",
            &images.join("rednice-old.jpg"),
            &thumbs.join("2026-11-20.jpg"),
        );
        insert_downloaded(
            &db,
            "2026-11-21",
            "https://www.bing.com/th?id=OHR.RedNice_1920x1080.jpg",
            &images.join("rednice-new.jpg"),
            &thumbs.join("2026-11-21.jpg"),
        );
        // Megaptera still carries the old (one day early) label.
        insert_downloaded(
            &db,
            "2026-11-19",
            "https://www.bing.com/th?id=OHR.Megaptera_1920x1080.jpg",
            &images.join("megaptera.jpg"),
            &thumbs.join("2026-11-19.jpg"),
        );

        let list = vec![
            bing("2026-11-21", "/th?id=OHR.RedNice"),
            bing("2026-11-20", "/th?id=OHR.Megaptera"),
        ];
        assert!(service.reconcile_list(&list).unwrap());

        assert!(db.get_by_date("2026-11-19").unwrap().is_none());
        assert_eq!(
            db.get_by_date("2026-11-20").unwrap().unwrap().file_path,
            images.join("megaptera.jpg").to_string_lossy()
        );
        assert_eq!(
            db.get_by_date("2026-11-21").unwrap().unwrap().file_path,
            images.join("rednice-new.jpg").to_string_lossy()
        );
        // The duplicate copy and its cache are gone; the moved row got a fresh
        // thumbnail for its new date.
        assert!(!images.join("rednice-old.jpg").exists());
        assert!(!thumbs.join("2026-11-19.jpg").exists());
        assert!(thumbs.join("2026-11-20.jpg").exists());
        assert!(thumbs.join("2026-11-21.jpg").exists());
        assert!(images.join("megaptera.jpg").exists());
    }

    #[tokio::test]
    async fn download_purges_a_same_picture_row_from_another_date() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/th"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(jpeg_bytes(800, 600)))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let images = dir.path().join("images");
        let thumbs = dir.path().join("thumbs");
        std::fs::create_dir_all(&images).unwrap();
        std::fs::create_dir_all(&thumbs).unwrap();
        let db = Db::open_in_memory().unwrap();
        let http = HttpClient::new(None).unwrap();
        let service = LibraryService::new(&http, &db, images.clone(), thumbs.clone());

        let old_file = images.join("old.jpg");
        let base = format!("{}/th?id=OHR.RedNice", server.uri());
        insert_downloaded(
            &db,
            "2026-11-20",
            &format!("{base}_1920x1080.jpg"),
            &old_file,
            &thumbs.join("2026-11-20.jpg"),
        );

        let meta = service
            .download_one(&bing("2026-11-21", &base), ImageSize::W1920x1080)
            .await
            .unwrap();

        assert_eq!(meta.date, "2026-11-21");
        assert!(db.get_by_date("2026-11-20").unwrap().is_none());
        assert!(!old_file.exists());
        assert_eq!(meta.thumb_path, thumbs.join("2026-11-21.jpg").to_string_lossy());
    }
}
