use tauri::{Emitter, State};

use crate::domain::errors::AppResult;
use crate::domain::models::{Settings, SettingsPatch};
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.settings())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> AppResult<Settings> {
    let previous = state.settings();
    let next = state.settings.update(patch)?;
    state.apply_settings_side_effects(&previous, &next)?;
    state
        .app
        .emit("settings://changed", &next)
        .map_err(|error| crate::domain::AppError::Other(error.to_string()))?;
    Ok(next)
}
