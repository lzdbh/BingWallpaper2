use serde::{Deserialize, Serialize};

/// Resolution requested from Bing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageSize {
    #[serde(rename = "uhd")]
    Uhd,
    #[serde(rename = "1920x1200")]
    W1920x1200,
    #[default]
    #[serde(rename = "1920x1080")]
    W1920x1080,
    #[serde(rename = "1366x768")]
    W1366x768,
}

impl ImageSize {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Uhd => "_UHD.jpg",
            Self::W1920x1200 => "_1920x1200.jpg",
            Self::W1920x1080 => "_1920x1080.jpg",
            Self::W1366x768 => "_1366x768.jpg",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Uhd => "UHD 4K",
            Self::W1920x1200 => "1920 × 1200",
            Self::W1920x1080 => "1920 × 1080",
            Self::W1366x768 => "1366 × 768",
        }
    }
}

/// Desktop background fit mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WallpaperStyle {
    #[default]
    Fill,
    Fit,
    Stretch,
    Tile,
    Center,
    Span,
}

impl WallpaperStyle {
    pub const ALL: [Self; 6] = [
        Self::Fill,
        Self::Fit,
        Self::Stretch,
        Self::Tile,
        Self::Center,
        Self::Span,
    ];
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFilter {
    #[default]
    All,
    Favorites,
    Recent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanPhase {
    #[default]
    Index,
    Thumbnail,
}

/// Progress of the background library scan (indexing files, then building thumbs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub current: u32,
    pub total: u32,
    pub percent: u32,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFinished {
    pub imported: u32,
    pub relinked: u32,
    pub thumbnails: u32,
    pub failed: u32,
    /// Set instead of the counters when the pass aborted; the gallery shows it
    /// as a warning and stops the progress bar.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMeta {
    pub id: i64,
    pub date: String,
    pub title: String,
    pub copyright: String,
    pub copyright_link: String,
    pub file_path: String,
    pub thumb_path: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub favorite: bool,
    /// The file lives in a user-selected directory instead of the app-managed
    /// image dir, so cleanup and delete must never touch it on disk.
    pub external: bool,
    /// Derived when the row is read: `file_path` still points at a real file.
    /// A row can only go stale if the picture was removed outside the app.
    pub downloaded: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub image_size: ImageSize,
    pub wallpaper_style: WallpaperStyle,
    pub image_dir: Option<String>,
    pub locale: String,
    pub market: String,
    pub theme: ThemeMode,
    pub close_to_tray: bool,
    pub auto_start: bool,
    pub auto_set: bool,
    pub auto_set_time: String,
    pub catch_up: bool,
    pub notifications: bool,
    pub auto_check_update: bool,
    pub update_channel: UpdateChannel,
    pub retention_days: u32,
    pub proxy: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            image_size: ImageSize::default(),
            wallpaper_style: WallpaperStyle::default(),
            image_dir: None,
            locale: "zh-CN".to_string(),
            market: "zh-CN".to_string(),
            theme: ThemeMode::default(),
            close_to_tray: true,
            auto_start: false,
            auto_set: true,
            auto_set_time: "00:05".to_string(),
            catch_up: true,
            notifications: true,
            auto_check_update: true,
            update_channel: UpdateChannel::default(),
            retention_days: 30,
            proxy: None,
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.locale = if self.locale.trim().is_empty() {
            "zh-CN".to_string()
        } else {
            self.locale
        };
        self.market = if self.market.trim().is_empty() {
            "zh-CN".to_string()
        } else {
            self.market
        };
        self.auto_set_time = normalize_time(&self.auto_set_time).unwrap_or_else(|| "00:05".into());
        self.retention_days = self.retention_days.clamp(1, 365);
        self.image_dir = self
            .image_dir
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self.proxy = self
            .proxy
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        self
    }

    /// Replace a style the current platform cannot apply with `fill`, returning
    /// whether the stored value had to change. Builds before the UI gated styles
    /// could persist a mode that the backend rejects, which then failed every
    /// scheduled apply.
    pub fn clamp_style_to(&mut self, supported: &[WallpaperStyle]) -> bool {
        if supported.contains(&self.wallpaper_style) {
            return false;
        }
        self.wallpaper_style = WallpaperStyle::default();
        true
    }
}

pub fn normalize_time(value: &str) -> Option<String> {
    let mut parts = value.trim().split(':');
    let hour: u32 = parts.next()?.parse().ok()?;
    let minute: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    Some(format!("{hour:02}:{minute:02}"))
}

/// Patch payload accepted by `update_settings`.
///
/// `image_dir` and `proxy` are `Option<Option<String>>` so that an explicit
/// `null` clears the value while an omitted field keeps the current one.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    #[serde(default)]
    pub image_size: Option<ImageSize>,
    #[serde(default)]
    pub wallpaper_style: Option<WallpaperStyle>,
    #[serde(default, deserialize_with = "double_option")]
    pub image_dir: Option<Option<String>>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub market: Option<String>,
    #[serde(default)]
    pub theme: Option<ThemeMode>,
    #[serde(default)]
    pub close_to_tray: Option<bool>,
    #[serde(default)]
    pub auto_start: Option<bool>,
    #[serde(default)]
    pub auto_set: Option<bool>,
    #[serde(default)]
    pub auto_set_time: Option<String>,
    #[serde(default)]
    pub catch_up: Option<bool>,
    #[serde(default)]
    pub notifications: Option<bool>,
    #[serde(default)]
    pub auto_check_update: Option<bool>,
    #[serde(default)]
    pub update_channel: Option<UpdateChannel>,
    #[serde(default)]
    pub retention_days: Option<u32>,
    #[serde(default, deserialize_with = "double_option")]
    pub proxy: Option<Option<String>>,
}

fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

impl SettingsPatch {
    pub fn apply_to(self, settings: &mut Settings) {
        if let Some(value) = self.image_size {
            settings.image_size = value;
        }
        if let Some(value) = self.wallpaper_style {
            settings.wallpaper_style = value;
        }
        if let Some(value) = self.image_dir {
            settings.image_dir = value;
        }
        if let Some(value) = self.locale {
            settings.locale = value;
        }
        if let Some(value) = self.market {
            settings.market = value;
        }
        if let Some(value) = self.theme {
            settings.theme = value;
        }
        if let Some(value) = self.close_to_tray {
            settings.close_to_tray = value;
        }
        if let Some(value) = self.auto_start {
            settings.auto_start = value;
        }
        if let Some(value) = self.auto_set {
            settings.auto_set = value;
        }
        if let Some(value) = self.auto_set_time {
            settings.auto_set_time = value;
        }
        if let Some(value) = self.catch_up {
            settings.catch_up = value;
        }
        if let Some(value) = self.notifications {
            settings.notifications = value;
        }
        if let Some(value) = self.auto_check_update {
            settings.auto_check_update = value;
        }
        if let Some(value) = self.update_channel {
            settings.update_channel = value;
        }
        if let Some(value) = self.retention_days {
            settings.retention_days = value;
        }
        if let Some(value) = self.proxy {
            settings.proxy = value;
        }
        let normalized = settings.clone().normalized();
        *settings = normalized;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStats {
    pub count: u32,
    pub favorite_count: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub os: String,
    pub arch: String,
    pub styles: Vec<WallpaperStyle>,
    pub multi_monitor: bool,
    pub desktop_environment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapState {
    pub settings: Settings,
    pub capabilities: PlatformCapabilities,
    pub today: Option<ImageMeta>,
    pub version: String,
    pub image_dir: String,
    pub storage: StorageStats,
    pub is_first_run: bool,
    /// A gallery-directory scan is still running when the window opens.
    pub scanning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWallpaperResult {
    pub success: bool,
    pub message: String,
    pub path: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub task_id: String,
    pub current: u32,
    pub total: u32,
    pub percent: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFinished {
    pub task_id: String,
    pub success: u32,
    pub failed: u32,
    pub skipped: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: String,
    pub url: String,
    pub published_at: String,
    pub mandatory: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearCacheResult {
    pub removed: u32,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoEvent {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_time_pads_and_validates() {
        assert_eq!(normalize_time("9:5").as_deref(), Some("09:05"));
        assert_eq!(normalize_time("23:59").as_deref(), Some("23:59"));
        assert_eq!(normalize_time(" 07:30 ").as_deref(), Some("07:30"));
        assert_eq!(normalize_time("24:00"), None);
        assert_eq!(normalize_time("12:60"), None);
        assert_eq!(normalize_time("12:30:45"), None);
        assert_eq!(normalize_time("abc"), None);
        assert_eq!(normalize_time(""), None);
    }

    #[test]
    fn default_settings_are_sane() {
        let settings = Settings::default();
        assert_eq!(settings.image_size, ImageSize::W1920x1080);
        assert_eq!(settings.wallpaper_style, WallpaperStyle::Fill);
        assert_eq!(settings.locale, "zh-CN");
        assert_eq!(settings.market, "zh-CN");
        assert_eq!(settings.theme, ThemeMode::System);
        assert_eq!(settings.update_channel, UpdateChannel::Stable);
        assert!(settings.auto_set);
        assert!(settings.close_to_tray);
        assert!(settings.catch_up);
        assert_eq!(settings.retention_days, 30);
    }

    #[test]
    fn normalized_fills_defaults_and_clamps() {
        let settings = Settings {
            locale: "  ".into(),
            market: "".into(),
            retention_days: 0,
            auto_set_time: "bad".into(),
            image_dir: Some("   ".into()),
            proxy: Some("".into()),
            ..Settings::default()
        };
        let normalized = settings.normalized();
        assert_eq!(normalized.locale, "zh-CN");
        assert_eq!(normalized.market, "zh-CN");
        assert_eq!(normalized.retention_days, 1);
        assert_eq!(normalized.auto_set_time, "00:05");
        assert_eq!(normalized.image_dir, None);
        assert_eq!(normalized.proxy, None);
    }

    #[test]
    fn normalized_clamps_high_retention_and_trims_paths() {
        let settings = Settings {
            retention_days: 9999,
            image_dir: Some("  D:\\pics  ".into()),
            proxy: Some("  http://127.0.0.1:7890  ".into()),
            ..Settings::default()
        };
        let normalized = settings.normalized();
        assert_eq!(normalized.retention_days, 365);
        assert_eq!(normalized.image_dir.as_deref(), Some("D:\\pics"));
        assert_eq!(normalized.proxy.as_deref(), Some("http://127.0.0.1:7890"));
    }

    #[test]
    fn patch_keeps_untouched_fields() {
        let mut settings = Settings::default();
        let before = settings.image_size;
        SettingsPatch::default().apply_to(&mut settings);
        assert_eq!(settings.image_size, before);
    }

    #[test]
    fn patch_updates_provided_fields() {
        let mut settings = Settings::default();
        SettingsPatch {
            image_size: Some(ImageSize::Uhd),
            wallpaper_style: Some(WallpaperStyle::Center),
            retention_days: Some(7),
            ..Default::default()
        }
        .apply_to(&mut settings);
        assert_eq!(settings.image_size, ImageSize::Uhd);
        assert_eq!(settings.wallpaper_style, WallpaperStyle::Center);
        assert_eq!(settings.retention_days, 7);
    }

    #[test]
    fn patch_double_option_clears_versus_keeps() {
        let mut settings = Settings {
            image_dir: Some("D:\\a".into()),
            ..Settings::default()
        };

        // `image_dir` absent from the patch keeps the current value.
        SettingsPatch {
            proxy: Some(Some("http://p".into())),
            ..Default::default()
        }
        .apply_to(&mut settings);
        assert_eq!(settings.image_dir.as_deref(), Some("D:\\a"));
        assert_eq!(settings.proxy.as_deref(), Some("http://p"));

        // An explicit `None` clears the value.
        SettingsPatch {
            image_dir: Some(None),
            ..Default::default()
        }
        .apply_to(&mut settings);
        assert_eq!(settings.image_dir, None);
    }
}
