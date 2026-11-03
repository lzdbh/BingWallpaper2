use std::fs::File;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;

use crate::domain::errors::{AppError, AppResult};

/// Generate a JPEG thumbnail that keeps the source aspect ratio.
pub fn generate(source: &Path, destination: &Path, max_width: u32) -> AppResult<()> {
    let image = image::open(source)?;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return Err(AppError::Io("图片尺寸为 0".into()));
    }

    let thumbnail = if width > max_width {
        let target_height = ((height as u64 * max_width as u64) / width as u64).max(1) as u32;
        image.resize_exact(max_width, target_height, FilterType::Lanczos3)
    } else {
        image
    };

    if let Some(parent) = destination.parent() {
        crate::infra::fs::ensure_dir(parent)?;
    }

    let mut file = File::create(destination).map_err(|error| {
        AppError::Io(format!("无法创建缩略图 {}：{error}", destination.display()))
    })?;
    let rgb = thumbnail.to_rgb8();
    let mut encoder = JpegEncoder::new_with_quality(&mut file, 82);
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(())
}

/// Read image dimensions without decoding the whole bitmap into memory.
pub fn dimensions(path: &Path) -> (u32, u32) {
    image::image_dimensions(path).unwrap_or((0, 0))
}
