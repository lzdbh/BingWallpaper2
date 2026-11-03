use std::path::Path;
use std::process::Command;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{PlatformCapabilities, WallpaperStyle};
use crate::domain::traits::WallpaperBackend;

pub struct MacOsBackend;

impl WallpaperBackend for MacOsBackend {
    fn set_wallpaper(&self, path: &Path, style: WallpaperStyle) -> AppResult<()> {
        if !path.is_file() {
            return Err(AppError::Io(format!("壁纸文件不存在：{}", path.display())));
        }
        // `System Events` only exposes a "fill" mode. Other modes are rejected
        // explicitly instead of silently falling back.
        if style != WallpaperStyle::Fill {
            return Err(AppError::UnsupportedStyle);
        }

        let escaped = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let script = format!(
            "tell application \"System Events\" to tell every desktop to set picture to POSIX file \"{escaped}\""
        );
        run("osascript", &["-e", &script])
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            os: "macos".to_string(),
            arch: std::env::consts::ARCH.to_string(),
            styles: vec![WallpaperStyle::Fill],
            multi_monitor: true,
            desktop_environment: Some("aqua".to_string()),
        }
    }
}

fn run(program: &str, args: &[&str]) -> AppResult<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| AppError::Io(format!("无法执行 {program}：{error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Io(format!(
            "{program} 执行失败：{}",
            stderr.trim()
        )));
    }
    Ok(())
}
