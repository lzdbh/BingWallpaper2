use std::path::Path;

use super::errors::AppResult;
use super::models::{PlatformCapabilities, WallpaperStyle};

/// Platform abstraction for changing the desktop background.
pub trait WallpaperBackend: Send + Sync {
    fn set_wallpaper(&self, path: &Path, style: WallpaperStyle) -> AppResult<()>;

    fn capabilities(&self) -> PlatformCapabilities;
}
