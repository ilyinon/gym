use tauri::State;

use crate::{domain::HistoryDay, error::CommandResult, infrastructure::Database};

#[tauri::command]
pub fn get_history(
    database: State<'_, Database>,
    date_from: i64,
    date_to: i64,
) -> CommandResult<HistoryDay> {
    database.history(date_from, date_to).map_err(Into::into)
}
