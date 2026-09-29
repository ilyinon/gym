use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::{domain::Settings, logger};

pub fn send_workout_reminder(app_handle: &AppHandle, settings: &Settings) -> bool {
    if !settings.notifications_enabled {
        return false;
    }

    let builder = app_handle
        .notification()
        .builder()
        .title("Пора размяться")
        .body("Откройте Gym и отметьте выполненные упражнения.");

    match builder.show() {
        Ok(()) => true,
        Err(error) => {
            logger::warn(format!("could not show system notification: {error}"));
            false
        }
    }
}
