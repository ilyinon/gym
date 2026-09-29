use tauri::State;

use crate::{domain::AppSnapshot, error::CommandResult, infrastructure::Database};

#[tauri::command]
pub fn get_app_snapshot(database: State<'_, Database>) -> CommandResult<AppSnapshot> {
    database.snapshot().map_err(Into::into)
}
