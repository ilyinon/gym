use tauri::{AppHandle, State};

use crate::{app_info::AppInfo, sound};

#[tauri::command]
pub fn get_app_info(app_info: State<'_, AppInfo>) -> AppInfo {
    app_info.inner().clone()
}

#[tauri::command]
pub fn preview_reminder_sound(app_handle: AppHandle) -> Result<(), String> {
    sound::play_workout_reminder(&app_handle)
}
