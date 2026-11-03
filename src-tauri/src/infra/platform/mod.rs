use crate::domain::traits::WallpaperBackend;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub fn create_backend() -> Box<dyn WallpaperBackend> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsBackend)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsBackend)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBackend::detect())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Box::new(UnsupportedBackend)
    }
}

#[allow(dead_code)]
struct UnsupportedBackend;

#[allow(dead_code)]
impl WallpaperBackend for UnsupportedBackend {
    fn set_wallpaper(
        &self,
        _path: &std::path::Path,
        _style: crate::domain::models::WallpaperStyle,
    ) -> crate::domain::errors::AppResult<()> {
        Err(crate::domain::errors::AppError::UnsupportedStyle)
    }

    fn capabilities(&self) -> crate::domain::models::PlatformCapabilities {
        crate::domain::models::PlatformCapabilities {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            styles: Vec::new(),
            multi_monitor: false,
            desktop_environment: None,
        }
    }
}
