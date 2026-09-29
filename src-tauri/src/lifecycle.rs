use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::{
    error::{AppError, AppResult},
    logger,
};

pub const AUTOSTART_ARGUMENT: &str = "--hidden";

pub fn launched_from_autostart() -> bool {
    std::env::args().any(|argument| argument == AUTOSTART_ARGUMENT)
}

pub fn set_autostart(app_handle: &AppHandle, enabled: bool) -> AppResult<()> {
    let manager = app_handle.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|error| AppError::System(error.to_string()))
}

pub fn reconcile_autostart(app_handle: &AppHandle, enabled: bool) {
    let manager = app_handle.autolaunch();
    let current = match manager.is_enabled() {
        Ok(value) => value,
        Err(error) => {
            logger::warn(format!("could not read autostart state: {error}"));
            return;
        }
    };
    if current != enabled {
        if let Err(error) = set_autostart(app_handle, enabled) {
            logger::warn(format!("could not reconcile autostart state: {error}"));
        }
    }
}
