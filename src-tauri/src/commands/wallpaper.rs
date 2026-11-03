use std::path::PathBuf;

use serde_json::json;
use tauri::{Emitter, State};

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{ImageMeta, SetWallpaperResult, WallpaperStyle};
use crate::state::AppState;

#[tauri::command]
pub async fn get_today_image(
    state: State<'_, AppState>,
    force: bool,
) -> AppResult<ImageMeta> {
    let settings = state.settings();
    state
        .library()
        .latest(force, settings.image_size, &settings.market)
        .await
}

#[tauri::command]
pub async fn set_wallpaper(
    state: State<'_, AppState>,
    date: Option<String>,
    style: Option<WallpaperStyle>,
) -> AppResult<SetWallpaperResult> {
    let settings = state.settings();
    let style = style.unwrap_or(settings.wallpaper_style);
    let date = date.unwrap_or_else(AppState::now_string);
    let started_at = AppState::now_rfc3339();

    let image = match state.db.get_by_date(&date)? {
        Some(meta) if std::path::Path::new(&meta.file_path).is_file() => meta,
        _ => {
            state
                .library()
                .ensure(&date, false, settings.image_size, &settings.market)
                .await?
        }
    };

    let result = state.wallpaper().apply_meta(&image, style)?;

    state
        .app
        .emit(
            "wallpaper://changed",
            json!({ "date": &result.date, "path": &result.path }),
        )
        .map_err(|error| AppError::Other(error.to_string()))?;

    let _ = state.db.record_task(
        "manual_set",
        "success",
        &result.message,
        &started_at,
        &AppState::now_rfc3339(),
    );

    Ok(result)
}

#[tauri::command]
pub async fn set_wallpaper_from_path(
    state: State<'_, AppState>,
    path: String,
    style: WallpaperStyle,
) -> AppResult<()> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(AppError::InvalidInput(format!(
            "文件不存在：{}",
            path.display()
        )));
    }

    state.wallpaper().apply_path(&path, style)?;

    state
        .app
        .emit(
            "wallpaper://changed",
            json!({
                "date": AppState::now_string(),
                "path": path.to_string_lossy(),
            }),
        )
        .map_err(|error| AppError::Other(error.to_string()))?;

    Ok(())
}
