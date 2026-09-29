use chrono::{DateTime, Local, LocalResult, TimeZone, Utc};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager,
};

use crate::{
    domain::{PauseRemindersInput, SchedulerSnapshot},
    infrastructure::Database,
    logger,
    scheduler::{current_time_utc_ms, next_due_from_utc, SchedulerHandle},
    windows,
};

const STATUS_ID: &str = "status";
const START_ID: &str = "start-workout";
const PAUSE_HOUR_ID: &str = "pause-hour";
const PAUSE_TOMORROW_ID: &str = "pause-tomorrow";
const RESUME_ID: &str = "resume";
const OPEN_ID: &str = "open";
const SETTINGS_ID: &str = "settings";
const EXIT_ID: &str = "exit";

struct TrayState {
    status: MenuItem<tauri::Wry>,
    pause_hour: MenuItem<tauri::Wry>,
    pause_tomorrow: MenuItem<tauri::Wry>,
    resume: MenuItem<tauri::Wry>,
}

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, STATUS_ID, "Gym запускается…", false, None::<&str>)?;
    let start = MenuItem::with_id(
        app,
        START_ID,
        "Записать тренировку сейчас",
        true,
        None::<&str>,
    )?;
    let pause_hour = MenuItem::with_id(app, PAUSE_HOUR_ID, "Пауза на час", true, None::<&str>)?;
    let pause_tomorrow = MenuItem::with_id(
        app,
        PAUSE_TOMORROW_ID,
        "Пауза до завтра",
        true,
        None::<&str>,
    )?;
    let resume = MenuItem::with_id(app, RESUME_ID, "Возобновить", false, None::<&str>)?;
    let open = MenuItem::with_id(app, OPEN_ID, "Открыть Gym", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS_ID, "Настройки", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, EXIT_ID, "Выход", true, None::<&str>)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &separator_one,
            &start,
            &pause_hour,
            &pause_tomorrow,
            &resume,
            &separator_two,
            &open,
            &settings,
            &exit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("gym-tray")
        .menu(&menu)
        .tooltip("Gym")
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event);
    if let Some(icon) = app.default_window_icon().cloned() {
        // macOS template icons adapt to light/dark menu bars. Linux keeps the
        // original colored application icon because template icons are macOS-only.
        builder = builder
            .icon(icon)
            .icon_as_template(cfg!(target_os = "macos"));
    }
    builder.build(app)?;

    app.manage(TrayState {
        status,
        pause_hour,
        pause_tomorrow,
        resume,
    });
    Ok(())
}

fn handle_menu_event(app_handle: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        START_ID => start_workout(app_handle),
        PAUSE_HOUR_ID => pause_until(
            app_handle,
            current_time_utc_ms().saturating_add(60 * 60 * 1_000),
        ),
        PAUSE_TOMORROW_ID => {
            let now = current_time_utc_ms();
            if let Some(until) = next_local_midnight(now) {
                pause_until(app_handle, until);
            }
        }
        RESUME_ID => resume(app_handle),
        OPEN_ID | SETTINGS_ID => windows::show_main(app_handle),
        EXIT_ID => app_handle.exit(0),
        _ => {}
    }
}

fn start_workout(app_handle: &AppHandle) {
    let database = app_handle.state::<Database>();
    match database.start_reminder_now(current_time_utc_ms()) {
        Ok(_) => match database.settings() {
            Ok(settings) => {
                windows::present_reminder(app_handle, &settings, true);
                app_handle.state::<SchedulerHandle>().wake();
            }
            Err(error) => logger::warn(format!("tray could not load settings: {error}")),
        },
        Err(error) => logger::warn(format!("tray could not start workout: {error}")),
    }
}

fn pause_until(app_handle: &AppHandle, until_utc: i64) {
    let database = app_handle.state::<Database>();
    if let Err(error) = database.pause_reminders(
        PauseRemindersInput {
            until_utc: Some(until_utc),
            indefinitely: false,
        },
        current_time_utc_ms(),
    ) {
        logger::warn(format!("tray could not pause reminders: {error}"));
        return;
    }
    app_handle.state::<SchedulerHandle>().wake();
}

fn resume(app_handle: &AppHandle) {
    let now = current_time_utc_ms();
    let database = app_handle.state::<Database>();
    let result = database
        .settings()
        .and_then(|settings| database.resume_reminders(now, next_due_from_utc(&settings, now)));
    if let Err(error) = result {
        logger::warn(format!("tray could not resume reminders: {error}"));
        return;
    }
    app_handle.state::<SchedulerHandle>().wake();
}

pub(crate) fn refresh_from_database(app_handle: &AppHandle) {
    let Some(database) = app_handle.try_state::<Database>() else {
        return;
    };
    if let Ok(snapshot) = database.scheduler_snapshot() {
        refresh(app_handle, &snapshot);
    }
}

pub(crate) fn refresh(app_handle: &AppHandle, snapshot: &SchedulerSnapshot) {
    let Some(tray) = app_handle.try_state::<TrayState>() else {
        return;
    };
    let paused = snapshot.settings.pause_indefinitely
        || snapshot
            .settings
            .paused_until_utc
            .is_some_and(|until| until > current_time_utc_ms());

    if let Err(error) = tray
        .status
        .set_text(status_text(snapshot, current_time_utc_ms()))
    {
        logger::warn(format!("could not update tray status: {error}"));
    }
    for result in [
        tray.pause_hour.set_enabled(!paused),
        tray.pause_tomorrow.set_enabled(!paused),
        tray.resume.set_enabled(paused),
    ] {
        if let Err(error) = result {
            logger::warn(format!("could not update tray action: {error}"));
        }
    }
}

fn status_text(snapshot: &SchedulerSnapshot, now_utc_ms: i64) -> String {
    if snapshot.pending_reminder.is_some() {
        return "Пора тренироваться".into();
    }
    if snapshot.settings.pause_indefinitely {
        return "Пауза до ручного возобновления".into();
    }
    if let Some(until) = snapshot.settings.paused_until_utc {
        let local = DateTime::<Utc>::from_timestamp_millis(until)
            .expect("stored timestamp must be valid")
            .with_timezone(&Local);
        return format!("Пауза до {}", local.format("%d.%m %H:%M"));
    }
    match snapshot.settings.next_due_at_utc {
        Some(next) => {
            let remaining_ms = next.saturating_sub(now_utc_ms).max(0);
            let remaining_minutes = remaining_ms.saturating_add(59_999) / 60_000;
            format!("Следующая тренировка через {remaining_minutes} мин")
        }
        None if snapshot.enabled_exercise_count == 0 => "Нет активных упражнений".into(),
        None => "Рассчитываем следующую тренировку…".into(),
    }
}

fn next_local_midnight(now_utc_ms: i64) -> Option<i64> {
    let now = DateTime::<Utc>::from_timestamp_millis(now_utc_ms)?.with_timezone(&Local);
    let next_date = now.date_naive().succ_opt()?;
    let midnight = next_date.and_hms_opt(0, 0, 0)?;
    match Local.from_local_datetime(&midnight) {
        LocalResult::Single(value) => Some(value.timestamp_millis()),
        LocalResult::Ambiguous(first, second) => Some(first.min(second).timestamp_millis()),
        LocalResult::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Reminder, ReminderStatus, Settings};

    fn snapshot() -> SchedulerSnapshot {
        SchedulerSnapshot {
            settings: Settings {
                interval_minutes: 15,
                active_start_minutes: 540,
                active_end_minutes: 1_140,
                active_days_mask: 127,
                notifications_enabled: true,
                popup_enabled: true,
                sound_enabled: true,
                always_on_top: false,
                autostart_enabled: false,
                paused_until_utc: None,
                pause_indefinitely: false,
                next_due_at_utc: Some(901_000),
                created_at_utc: 0,
                updated_at_utc: 0,
            },
            pending_reminder: None,
            enabled_exercise_count: 3,
        }
    }

    #[test]
    fn status_shows_rounded_remaining_minutes() {
        assert_eq!(
            status_text(&snapshot(), 1_000),
            "Следующая тренировка через 15 мин"
        );
    }

    #[test]
    fn pending_state_has_priority_in_status() {
        let mut snapshot = snapshot();
        snapshot.settings.pause_indefinitely = true;
        snapshot.pending_reminder = Some(Reminder {
            id: "pending".into(),
            status: ReminderStatus::Pending,
            original_due_at_utc: 1_000,
            due_at_utc: 1_000,
            first_presented_at_utc: None,
            last_presented_at_utc: None,
            resolved_at_utc: None,
            snooze_count: 0,
            created_at_utc: 1_000,
        });
        assert_eq!(status_text(&snapshot, 1_000), "Пора тренироваться");
    }
}
