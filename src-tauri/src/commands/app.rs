use std::path::Path;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{
    BootstrapState, ClearCacheResult, PlatformCapabilities, StorageStats,
};
use crate::state::AppState;

#[tauri::command]
pub async fn get_bootstrap_state(state: State<'_, AppState>) -> AppResult<BootstrapState> {
    let settings = state.settings();
    let capabilities = state.backend.capabilities();

    let today = match state.db.get_by_date(&AppState::now_string())? {
        Some(meta) if Path::new(&meta.file_path).is_file() => Some(meta),
        _ => match state
            .library()
            .latest(false, settings.image_size, &settings.market)
            .await
        {
            Ok(meta) => Some(meta),
            Err(error) => {
                log::warn!("启动时获取今日壁纸失败：{error}");
                state.db.latest().ok().flatten()
            }
        },
    };

    let storage: StorageStats = state.db.stats().unwrap_or_default();

    Ok(BootstrapState {
        settings,
        capabilities,
        today,
        version: state.version.clone(),
        image_dir: state.images_dir().to_string_lossy().to_string(),
        storage,
        is_first_run: state.settings.is_first_run(),
        scanning: state.is_scanning(),
    })
}

#[tauri::command]
pub async fn get_platform_capabilities(
    state: State<'_, AppState>,
) -> AppResult<PlatformCapabilities> {
    Ok(state.backend.capabilities())
}

#[tauri::command]
pub async fn open_image_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let dir = state.images_dir();
    crate::infra::fs::ensure_dir(&dir)?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|error| AppError::Io(format!("无法打开目录：{error}")))
}

const ALLOWED_HOSTS: [&str; 6] = [
    "github.com",
    "www.github.com",
    "bing.com",
    "www.bing.com",
    "gnu.org",
    "www.gnu.org",
];

#[tauri::command]
pub async fn open_external(app: AppHandle, url: String) -> AppResult<()> {
    let parsed = url::Url::parse(&url)
        .map_err(|error| AppError::InvalidInput(format!("链接格式无效：{error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::InvalidInput("只允许打开 http/https 链接".into()));
    }
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    let allowed = ALLOWED_HOSTS
        .iter()
        .any(|item| host == *item || host.ends_with(&format!(".{item}")));
    if !allowed {
        return Err(AppError::InvalidInput(format!(
            "不允许打开该链接：{host}"
        )));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| AppError::Io(format!("无法打开链接：{error}")))
}

pub fn reveal_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> AppResult<()> {
    reveal_main_window(&app);
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> AppResult<()> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> AppResult<ClearCacheResult> {
    let settings = state.settings();
    if settings.retention_days == 0 {
        return Ok(ClearCacheResult::default());
    }
    let cutoff = (chrono::Local::now() - chrono::Duration::days(settings.retention_days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let candidates = state.db.older_than(&cutoff)?;
    let mut removed = 0u32;
    let mut bytes_freed = 0u64;

    for image in candidates {
        let bytes = image.bytes;
        match state.library().remove(image.id) {
            Ok(Some(_)) => {
                removed += 1;
                bytes_freed += bytes;
            }
            Ok(None) => {}
            Err(error) => log::warn!("清理图片 {} 失败：{error}", image.id),
        }
    }

    state.app.emit("library://changed", ()).ok();

    Ok(ClearCacheResult {
        removed,
        bytes_freed,
    })
}
