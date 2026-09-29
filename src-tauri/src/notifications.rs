use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::domain::Settings;

pub fn send_workout_reminder(app_handle: &AppHandle, settings: &Settings) -> bool {
    if !settings.notifications_enabled {
        return false;
    }

    let builder = app_handle
        .notification()
        .builder()
        .title("Пора размяться")
        .body("Откройте Gym и отметьте выполненные упражнения.");

    #[cfg(target_os = "macos")]
    let builder = if settings.sound_enabled {
        builder.sound("Ping")
    } else {
        builder
    };
    #[cfg(target_os = "linux")]
    let builder = if settings.sound_enabled {
        builder.sound("message-new-instant")
    } else {
        builder
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let builder = builder;

    match builder.show() {
        Ok(()) => true,
        Err(error) => {
            eprintln!("could not show system notification: {error}");
            false
        }
    }
}
