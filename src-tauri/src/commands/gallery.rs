use tauri::{Emitter, State};

use crate::domain::errors::AppResult;
use crate::domain::models::{ImageFilter, ImageMeta, Page};
use crate::state::AppState;

#[tauri::command]
pub async fn list_images(
    state: State<'_, AppState>,
    page: u32,
    page_size: u32,
    filter: ImageFilter,
    query: Option<String>,
) -> AppResult<Page<ImageMeta>> {
    state.db.list(page, page_size, filter, query.as_deref())
}

#[tauri::command]
pub async fn toggle_favorite(
    state: State<'_, AppState>,
    id: i64,
    favorite: bool,
) -> AppResult<ImageMeta> {
    let image = state
        .db
        .set_favorite(id, favorite)?
        .ok_or_else(|| crate::domain::AppError::InvalidInput(format!("图片 {id} 不存在")))?;
    state.app.emit("library://changed", ()).ok();
    Ok(image)
}

#[tauri::command]
pub async fn delete_image(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.library().remove(id)?;
    state.app.emit("library://changed", ()).ok();
    Ok(())
}

/// Re-index the configured gallery directory. Returns false when a pass is
/// already running.
#[tauri::command]
pub async fn rescan_library(state: State<'_, AppState>) -> AppResult<bool> {
    Ok(state.spawn_library_scan())
}
