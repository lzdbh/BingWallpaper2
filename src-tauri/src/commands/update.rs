use tauri::{Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::UpdateInfo;
use crate::services::updater;
use crate::state::AppState;

#[tauri::command]
pub async fn check_update(state: State<'_, AppState>) -> AppResult<Option<UpdateInfo>> {
    let settings = state.settings();
    let info = updater::check(&state.http, &state.version, settings.update_channel).await?;

    if let Ok(mut guard) = state.last_update.lock() {
        *guard = info.clone();
    }

    if let Some(info) = &info {
        state.app.emit("update://available", info).ok();
    }

    Ok(info)
}

#[tauri::command]
pub async fn install_update(state: State<'_, AppState>) -> AppResult<()> {
    let info = state
        .last_update
        .lock()
        .ok()
        .and_then(|guard| guard.clone());

    let Some(info) = info else {
        return Err(AppError::Update("当前没有可安装的更新".into()));
    };

    if info.url.is_empty() {
        return Err(AppError::Update("更新缺少下载地址".into()));
    }

    state
        .app
        .opener()
        .open_url(info.url, None::<&str>)
        .map_err(|error| AppError::Io(format!("无法打开下载页面：{error}")))
}
