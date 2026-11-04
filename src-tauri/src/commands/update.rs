use serde::Serialize;
use tauri::{Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::UpdateInfo;
use crate::services::updater;
use crate::state::AppState;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
    percent: u16,
}

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
    let update = state
        .app
        .updater()
        .map_err(updater_error)?
        .check()
        .await
        .map_err(updater_error)?;

    // The signed manifest only carries stable releases, so a beta build that the
    // GitHub API surfaced has nothing here to install; keep the old behaviour of
    // opening the release page for that case.
    let Some(update) = update else {
        return open_release_page(&state);
    };

    let app = state.app.clone();
    let mut downloaded: u64 = 0;
    let mut last_percent = u16::MAX;
    update
        .download_and_install(
            move |chunk_length, total| {
                downloaded += chunk_length as u64;
                let percent = total.map_or(0, |total| {
                    ((downloaded as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as u16
                });
                if percent == last_percent {
                    return;
                }
                last_percent = percent;
                app.emit(
                    "update://progress",
                    UpdateProgress {
                        downloaded,
                        total,
                        percent,
                    },
                )
                .ok();
            },
            || {},
        )
        .await
        .map_err(updater_error)?;

    // Windows hands over to the installer process; the other platforms relaunch.
    #[cfg(not(windows))]
    state.app.restart();

    Ok(())
}

fn open_release_page(state: &AppState) -> AppResult<()> {
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

fn updater_error(error: impl std::fmt::Display) -> AppError {
    AppError::Update(format!("更新失败：{error}"))
}
