mod commands;
pub mod domain;
mod error;
mod infrastructure;
mod lifecycle;
mod notifications;
mod scheduler;
mod tray;
mod windows;

use std::fs;

use infrastructure::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            windows::show_main(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg(lifecycle::AUTOSTART_ARGUMENT)
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == windows::REMINDER_WINDOW_LABEL
                    || window.label() == windows::MAIN_WINDOW_LABEL
                {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("could not hide window on close: {error}");
                    }
                }
            }
        })
        .setup(|app| {
            let app_data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_directory)?;
            let database = Database::open(app_data_directory.join("gym.sqlite3"))?;
            let settings = database.settings()?;
            app.manage(database.clone());
            tray::setup(app)?;
            lifecycle::reconcile_autostart(app.handle(), settings.autostart_enabled);
            if !lifecycle::launched_from_autostart() {
                windows::show_main(app.handle());
            }
            let scheduler = scheduler::start_scheduler(app.handle().clone(), database);
            app.manage(scheduler);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot::get_app_snapshot,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::exercises::list_exercises,
            commands::exercises::create_exercise,
            commands::exercises::update_exercise,
            commands::exercises::reorder_exercises,
            commands::exercises::delete_exercise,
            commands::history::get_history,
            commands::reminders::start_workout_now,
            commands::reminders::complete_reminder,
            commands::reminders::snooze_reminder,
            commands::reminders::skip_reminder,
            commands::reminders::pause_reminders,
            commands::reminders::resume_reminders,
            commands::reminders::hide_reminder_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
