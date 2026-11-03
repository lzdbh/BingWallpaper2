use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;

use crate::domain::errors::{AppError, AppResult};
use crate::infra::fs;

/// Gallery cards are at most ~450 CSS pixels wide; 640 keeps a 2x display sharp
/// while staying an order of magnitude smaller than the original.
pub const THUMB_WIDTH: u32 = 640;
const THUMB_QUALITY: u8 = 80;

/// Deterministic thumbnail location for a library date, so a rescan never has
/// to trust a stale `thumb_path` column.
pub fn thumb_path_for(thumbs_dir: &Path, date: &str) -> PathBuf {
    thumbs_dir.join(format!("{date}.jpg"))
}

/// A thumbnail is reusable when it is at least as new as its source and
/// non-empty; anything else (missing, interrupted build, edited original) is
/// rebuilt.
pub fn is_fresh(source: &Path, thumbnail: &Path) -> bool {
    let (Ok(source_meta), Ok(thumb_meta)) = (source.metadata(), thumbnail.metadata()) else {
        return false;
    };
    if thumb_meta.len() < 512 {
        return false;
    }
    match (source_meta.modified(), thumb_meta.modified()) {
        (Ok(source_time), Ok(thumb_time)) => thumb_time >= source_time,
        _ => true,
    }
}

/// Generate a JPEG thumbnail that keeps the source aspect ratio.
pub fn generate(source: &Path, destination: &Path, max_width: u32) -> AppResult<()> {
    let image = image::ImageReader::open(source)?
        .with_guessed_format()?
        .decode()?;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return Err(AppError::Io("图片尺寸为 0".into()));
    }

    // The integer box filter is far cheaper than Lanczos over a full 4K frame
    // and stays clean at this 3x downscale.
    let thumbnail = if width > max_width {
        let target_height = ((height as u64 * max_width as u64) / width as u64).max(1) as u32;
        image.thumbnail_exact(max_width, target_height)
    } else {
        image
    };

    if let Some(parent) = destination.parent() {
        fs::ensure_dir(parent)?;
    }

    // Encode into memory first: a half-written file would otherwise be picked
    // up as a valid thumbnail by the next scan.
    let rgb = thumbnail.to_rgb8();
    let mut buffer = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut buffer, THUMB_QUALITY);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )?;
    }

    let tmp = destination.with_extension("jpg.tmp");
    std::fs::write(&tmp, &buffer)
        .map_err(|error| AppError::Io(format!("无法写入缩略图：{error}")))?;
    std::fs::rename(&tmp, destination).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        AppError::Io(format!("无法保存缩略图：{error}"))
    })?;
    Ok(())
}

/// Read image dimensions without decoding the whole bitmap into memory.
pub fn dimensions(path: &Path) -> (u32, u32) {
    image::image_dimensions(path).unwrap_or((0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_source(dir: &Path, name: &str, width: u32, height: u32) -> std::path::PathBuf {
        let path = dir.join(name);
        // A gradient rather than a solid colour, so the encoded files are
        // comfortably larger than the `is_fresh` size floor.
        let buffer = image::ImageBuffer::from_fn(width, height, |x, y| {
            image::Rgb([x as u8, y as u8, (x ^ y) as u8])
        });
        image::DynamicImage::ImageRgb8(buffer)
            .save(&path)
            .unwrap();
        path
    }

    #[test]
    fn thumb_path_is_date_scoped() {
        assert_eq!(
            thumb_path_for(Path::new("thumbs"), "2016-03-23"),
            PathBuf::from("thumbs/2016-03-23.jpg")
        );
    }

    #[test]
    fn generate_downscales_and_keeps_aspect() {
        let root = tempfile::tempdir().unwrap();
        let source = write_source(root.path(), "20160323.jpg", 1920, 1080);
        let destination = thumb_path_for(root.path(), "2016-03-23");

        generate(&source, &destination, THUMB_WIDTH).unwrap();

        let (width, height) = dimensions(&destination);
        assert_eq!(width, 640);
        assert_eq!(height, 360);
        assert!(!destination.with_extension("jpg.tmp").exists());
    }

    #[test]
    fn generate_leaves_a_small_source_untouched() {
        let root = tempfile::tempdir().unwrap();
        let source = write_source(root.path(), "small.jpg", 320, 200);
        let destination = root.path().join("small-thumb.jpg");

        generate(&source, &destination, THUMB_WIDTH).unwrap();

        assert_eq!(dimensions(&destination), (320, 200));
    }

    #[test]
    fn freshness_tracks_existence_size_and_mtime() {
        let root = tempfile::tempdir().unwrap();
        let source = write_source(root.path(), "20160323.jpg", 1920, 1080);
        let thumb = thumb_path_for(root.path(), "2016-03-23");

        // Missing thumbnail.
        assert!(!is_fresh(&source, &thumb));

        // Too small to be a real render (interrupted write).
        std::fs::write(&thumb, b"tiny").unwrap();
        assert!(!is_fresh(&source, &thumb));

        generate(&source, &thumb, THUMB_WIDTH).unwrap();
        assert!(is_fresh(&source, &thumb));

        // An edited original is newer than the cached thumbnail.
        let newer = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        filetime_set(&source, newer);
        assert!(!is_fresh(&source, &thumb));
    }

    fn filetime_set(path: &Path, when: std::time::SystemTime) {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap();
        file.set_modified(when).unwrap();
    }
}
