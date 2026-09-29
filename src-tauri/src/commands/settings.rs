use tauri::{AppHandle, State};

use crate::{
    domain::{Settings, UpdateSettingsInput},
    error::CommandResult,
    infrastructure::Database,
    lifecycle,
    scheduler::SchedulerHandle,
};

#[tauri::command]
pub fn get_settings(database: State<'_, Database>) -> CommandResult<Settings> {
    database.settings().map_err(Into::into)
}

#[tauri::command]
pub fn update_settings(
    app_handle: AppHandle,
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    input: UpdateSettingsInput,
) -> CommandResult<Settings> {
    let previous = database.settings()?;
    let autostart_changed = previous.autostart_enabled != input.autostart_enabled;
    if autostart_changed {
        lifecycle::set_autostart(&app_handle, input.autostart_enabled)?;
    }
    let settings = match database.update_settings(input) {
        Ok(settings) => settings,
        Err(error) => {
            if autostart_changed {
                let _ = lifecycle::set_autostart(&app_handle, previous.autostart_enabled);
            }
            return Err(error.into());
        }
    };
    scheduler.wake();
    Ok(settings)
}
