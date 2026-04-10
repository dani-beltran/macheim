use std::sync::Mutex;

use tracing::info;

use crate::error::{AppError, AppResult};
use crate::services::{game_detector, launcher};
use crate::AppState;

/// Launch Valheim with BepInEx (modded).
#[tauri::command]
pub async fn launch_modded(state: tauri::State<'_, Mutex<AppState>>) -> AppResult<()> {
    info!("Command: launch_modded");

    let game_path = {
        let state = state.lock().map_err(|e| {
            AppError::GameNotFound(format!("Failed to lock state: {}", e))
        })?;
        state
            .game_path
            .clone()
            .ok_or_else(|| AppError::GameNotFound("Game path not set".to_string()))?
    };

    let game_root = game_detector::get_valheim_root(&game_path);
    launcher::launch_modded(&game_path, &game_root)
}

/// Launch Valheim vanilla (via Steam, no mods).
#[tauri::command]
pub async fn launch_vanilla(_state: tauri::State<'_, Mutex<AppState>>) -> AppResult<()> {
    info!("Command: launch_vanilla");
    launcher::launch_vanilla()
}
