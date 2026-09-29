use tauri::{AppHandle, State};

use crate::{
    domain::{CompleteReminderInput, PauseRemindersInput, Reminder, Settings, SnoozeReminderInput},
    error::CommandResult,
    infrastructure::Database,
    scheduler::{current_time_utc_ms, next_due_from_utc, SchedulerHandle},
    windows,
};

#[tauri::command]
pub fn start_workout_now(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
) -> CommandResult<Reminder> {
    let reminder = database.start_reminder_now(current_time_utc_ms())?;
    scheduler.wake();
    Ok(reminder)
}

#[tauri::command]
pub fn complete_reminder(
    app_handle: AppHandle,
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    input: CompleteReminderInput,
) -> CommandResult<Reminder> {
    let now = current_time_utc_ms();
    let settings = database.settings()?;
    let reminder = database.complete_reminder(input, now, next_due_from_utc(&settings, now))?;
    windows::hide_reminder(&app_handle);
    scheduler.wake();
    Ok(reminder)
}

#[tauri::command]
pub fn snooze_reminder(
    app_handle: AppHandle,
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    input: SnoozeReminderInput,
) -> CommandResult<Reminder> {
    let reminder = database.snooze_reminder(input, current_time_utc_ms())?;
    windows::hide_reminder(&app_handle);
    scheduler.wake();
    Ok(reminder)
}

#[tauri::command]
pub fn skip_reminder(
    app_handle: AppHandle,
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    reminder_id: String,
) -> CommandResult<Reminder> {
    let now = current_time_utc_ms();
    let settings = database.settings()?;
    let reminder = database.skip_reminder(&reminder_id, now, next_due_from_utc(&settings, now))?;
    windows::hide_reminder(&app_handle);
    scheduler.wake();
    Ok(reminder)
}

#[tauri::command]
pub fn hide_reminder_window(app_handle: AppHandle) {
    windows::hide_reminder(&app_handle);
}

#[tauri::command]
pub fn pause_reminders(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    input: PauseRemindersInput,
) -> CommandResult<Settings> {
    let settings = database.pause_reminders(input, current_time_utc_ms())?;
    scheduler.wake();
    Ok(settings)
}

#[tauri::command]
pub fn resume_reminders(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
) -> CommandResult<Settings> {
    let now = current_time_utc_ms();
    let current = database.settings()?;
    let settings = database.resume_reminders(now, next_due_from_utc(&current, now))?;
    scheduler.wake();
    Ok(settings)
}
