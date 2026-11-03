use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{Settings, SettingsPatch, WallpaperStyle};
use crate::infra::fs;

/// JSON-backed settings store with atomic writes.
pub struct SettingsStore {
    path: PathBuf,
    settings: RwLock<Settings>,
    first_run: bool,
    supported_styles: Vec<WallpaperStyle>,
}

impl SettingsStore {
    pub fn load(path: PathBuf, supported_styles: Vec<WallpaperStyle>) -> Self {
        let mut first_run = false;
        let mut settings = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(settings) => settings.normalized(),
                Err(error) => {
                    log::warn!("settings.json 无法解析，使用默认设置：{error}");
                    let backup = path.with_extension("json.broken");
                    let _ = std::fs::rename(&path, &backup);
                    first_run = true;
                    Settings::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                first_run = true;
                Settings::default()
            }
            Err(error) => {
                log::warn!("读取 settings.json 失败，使用默认设置：{error}");
                Settings::default()
            }
        };

        let style_clamped = settings.clamp_style_to(&supported_styles);
        if style_clamped {
            log::warn!("settings.json 记录的壁纸契合度不受当前平台支持，已改用填充");
        }

        let store = Self {
            path,
            settings: RwLock::new(settings.clone()),
            first_run,
            supported_styles,
        };
        if first_run || style_clamped {
            if let Err(error) = store.persist(&settings) {
                log::warn!("写入默认设置失败：{error}");
            }
        }
        store
    }

    pub fn is_first_run(&self) -> bool {
        self.first_run
    }

    pub fn get(&self) -> Settings {
        self.settings
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn update(&self, patch: SettingsPatch) -> AppResult<Settings> {
        let mut next = self.get();
        patch.apply_to(&mut next);
        next.clamp_style_to(&self.supported_styles);
        self.persist(&next)?;
        if let Ok(mut guard) = self.settings.write() {
            *guard = next.clone();
        }
        Ok(next)
    }

    pub fn set(&self, settings: Settings) -> AppResult<Settings> {
        let mut normalized = settings.normalized();
        normalized.clamp_style_to(&self.supported_styles);
        self.persist(&normalized)?;
        if let Ok(mut guard) = self.settings.write() {
            *guard = normalized.clone();
        }
        Ok(normalized)
    }

    fn persist(&self, settings: &Settings) -> AppResult<()> {
        let json = serde_json::to_vec_pretty(settings)?;
        fs::atomic_write(&self.path, &json)
    }
}

pub fn default_image_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("images")
}

pub fn resolve_image_dir(data_dir: &Path, settings: &Settings) -> PathBuf {
    settings
        .image_dir
        .as_ref()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| default_image_dir(data_dir))
}

pub fn validate_dir(path: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err(AppError::InvalidInput("目录不能为空".into()));
    }
    fs::ensure_dir(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_image_dir_is_images_subfolder() {
        let data = Path::new("data");
        assert_eq!(default_image_dir(data), data.join("images"));
    }

    #[test]
    fn resolve_image_dir_prefers_setting_then_falls_back() {
        let data = Path::new("data");

        let unset = Settings {
            image_dir: None,
            ..Settings::default()
        };
        assert_eq!(resolve_image_dir(data, &unset), data.join("images"));

        let custom = Settings {
            image_dir: Some("D:\\custom".to_string()),
            ..Settings::default()
        };
        assert_eq!(resolve_image_dir(data, &custom), PathBuf::from("D:\\custom"));

        // An empty value falls back to the default directory.
        let empty = Settings {
            image_dir: Some(String::new()),
            ..Settings::default()
        };
        assert_eq!(resolve_image_dir(data, &empty), data.join("images"));
    }

    #[test]
    fn validate_dir_rejects_empty() {
        let error = validate_dir("").unwrap_err();
        assert!(matches!(error, AppError::InvalidInput(_)));
    }

    #[test]
    fn validate_dir_creates_and_returns_path() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("wallpapers");
        let resolved = validate_dir(target.to_str().unwrap()).unwrap();
        assert_eq!(resolved, target);
        assert!(target.is_dir());
    }

    fn write_settings(path: &Path, style: WallpaperStyle) {
        let stored = Settings {
            wallpaper_style: style,
            ..Settings::default()
        };
        std::fs::write(path, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();
    }

    #[test]
    fn load_clamps_an_unsupported_style_and_rewrites_the_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        write_settings(&path, WallpaperStyle::Center);

        let store = SettingsStore::load(path.clone(), vec![WallpaperStyle::Fill]);

        assert_eq!(store.get().wallpaper_style, WallpaperStyle::Fill);
        let on_disk: Settings = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(on_disk.wallpaper_style, WallpaperStyle::Fill);
    }

    #[test]
    fn load_keeps_a_style_the_platform_supports() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        write_settings(&path, WallpaperStyle::Center);

        let store = SettingsStore::load(path, WallpaperStyle::ALL.to_vec());

        assert_eq!(store.get().wallpaper_style, WallpaperStyle::Center);
    }

    #[test]
    fn update_clamps_a_style_the_platform_cannot_apply() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        let store = SettingsStore::load(path, vec![WallpaperStyle::Fill]);

        let patch = SettingsPatch {
            wallpaper_style: Some(WallpaperStyle::Span),
            ..SettingsPatch::default()
        };
        let next = store.update(patch).unwrap();

        assert_eq!(next.wallpaper_style, WallpaperStyle::Fill);
    }
}
