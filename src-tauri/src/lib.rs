mod app_info;
mod commands;
pub mod domain;
mod error;
mod infrastructure;
mod lifecycle;
mod logger;
mod notifications;
mod scheduler;
mod sound;
mod tray;
mod windows;

use std::fs;

use app_info::AppInfo;
use error::AppError;
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
                let fatal_startup = window
                    .app_handle()
                    .try_state::<AppInfo>()
                    .is_some_and(|info| info.fatal_error.is_some());
                if window.label() == windows::REMINDER_WINDOW_LABEL
                    || window.label() == windows::MAIN_WINDOW_LABEL
                {
                    if fatal_startup {
                        return;
                    }
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        logger::warn(format!("could not hide window on close: {error}"));
                    }
                }
            }
        })
        .setup(|app| {
            let app_data_directory = app.path().app_data_dir()?;
            let database_path = app_data_directory.join("gym.sqlite3");
            let log_path = app_data_directory.join("logs").join("gym.log");
            let database_result = fs::create_dir_all(&app_data_directory)
                .map_err(AppError::from)
                .and_then(|_| {
                    if let Err(error) = logger::init(&app_data_directory) {
                        eprintln!("could not initialize local logging: {error}");
                    }
                    Database::open(&database_path)
                });

            let database = match database_result {
                Ok(database) => database,
                Err(error) => {
                    let message = format!(
                        "Не удалось открыть локальную базу данных. Проверьте доступ к каталогу данных и перезапустите Gym. Техническая причина: {error}"
                    );
                    logger::error(&message);
                    app.manage(AppInfo {
                        version: env!("CARGO_PKG_VERSION").into(),
                        database_path: database_path.to_string_lossy().into_owned(),
                        log_path: log_path.to_string_lossy().into_owned(),
                        fatal_error: Some(message),
                    });
                    windows::show_main(app.handle());
                    return Ok(());
                }
            };

            app.manage(AppInfo {
                version: env!("CARGO_PKG_VERSION").into(),
                database_path: database_path.to_string_lossy().into_owned(),
                log_path: log_path.to_string_lossy().into_owned(),
                fatal_error: None,
            });
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
            commands::system::get_app_info,
            commands::system::preview_reminder_sound,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
