use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

use crate::{infrastructure::Database, notifications, tray, windows};

use super::{
    clock::{Clock, SystemClock},
    schedule::{reconcile_local, SchedulerDecision},
};

const CLOCK_RECHECK_INTERVAL_MS: i64 = 30_000;
const ERROR_RETRY_INTERVAL_MS: i64 = 5_000;

#[derive(Clone)]
pub struct SchedulerHandle {
    wake_sender: watch::Sender<u64>,
    app_handle: AppHandle,
}

impl SchedulerHandle {
    pub fn wake(&self) {
        let next_revision = (*self.wake_sender.borrow()).wrapping_add(1);
        self.wake_sender.send_replace(next_revision);
        emit_state_changed(&self.app_handle);
    }
}

pub fn start_scheduler(app_handle: AppHandle, database: Database) -> SchedulerHandle {
    let (wake_sender, wake_receiver) = watch::channel(0_u64);
    tauri::async_runtime::spawn(run_scheduler(
        app_handle.clone(),
        database,
        wake_receiver,
        SystemClock,
    ));
    SchedulerHandle {
        wake_sender,
        app_handle,
    }
}

async fn run_scheduler<C>(
    app_handle: AppHandle,
    database: Database,
    mut wake_receiver: watch::Receiver<u64>,
    clock: C,
) where
    C: Clock + 'static,
{
    let mut last_presented_key: Option<(String, i64)> = None;

    loop {
        let now_utc_ms = clock.now_utc_ms();
        let scheduler_snapshot = match database.scheduler_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                eprintln!("scheduler could not load state: {error}");
                wait_for_wake_or_timeout(&mut wake_receiver, ERROR_RETRY_INTERVAL_MS).await;
                continue;
            }
        };
        tray::refresh(&app_handle, &scheduler_snapshot);

        let decision = reconcile_local(
            &scheduler_snapshot.settings,
            scheduler_snapshot.pending_reminder.as_ref(),
            scheduler_snapshot.enabled_exercise_count,
            now_utc_ms,
        );

        let next_wait = match decision {
            SchedulerDecision::WaitForChange => {
                last_presented_key = None;
                None
            }
            SchedulerDecision::WaitUntil(timestamp) => {
                if scheduler_snapshot.pending_reminder.is_none() {
                    last_presented_key = None;
                }
                Some(timestamp.saturating_sub(now_utc_ms))
            }
            SchedulerDecision::SetNextDue(next_due_at_utc) => {
                if let Err(error) = database.set_next_due(next_due_at_utc) {
                    eprintln!("scheduler could not persist next due time: {error}");
                    Some(ERROR_RETRY_INTERVAL_MS)
                } else {
                    emit_state_changed(&app_handle);
                    Some(0)
                }
            }
            SchedulerDecision::ResumeAfterPause { next_due_at_utc } => {
                if let Err(error) = database.resume_schedule(next_due_at_utc) {
                    eprintln!("scheduler could not resume expired pause: {error}");
                    Some(ERROR_RETRY_INTERVAL_MS)
                } else {
                    emit_state_changed(&app_handle);
                    Some(0)
                }
            }
            SchedulerDecision::CreateReminder {
                original_due_at_utc,
                due_at_utc,
            } => {
                match database.create_pending_reminder(original_due_at_utc, due_at_utc, now_utc_ms)
                {
                    Ok(_) => {
                        emit_state_changed(&app_handle);
                        Some(0)
                    }
                    Err(error) => {
                        eprintln!("scheduler could not create reminder: {error}");
                        Some(ERROR_RETRY_INTERVAL_MS)
                    }
                }
            }
            SchedulerDecision::PresentReminder(reminder) => {
                let presentation_key = (reminder.id.clone(), reminder.due_at_utc);
                if last_presented_key.as_ref() != Some(&presentation_key) {
                    match database.mark_reminder_presented(&reminder.id, now_utc_ms) {
                        Ok(presented) => {
                            if let Err(error) = app_handle.emit("gym://reminder-due", &presented) {
                                eprintln!("scheduler could not emit reminder event: {error}");
                            }
                            emit_state_changed(&app_handle);
                            let notification_delivered = notifications::send_workout_reminder(
                                &app_handle,
                                &scheduler_snapshot.settings,
                            );
                            let needs_fallback = scheduler_snapshot.settings.notifications_enabled
                                && !notification_delivered;
                            windows::present_reminder(
                                &app_handle,
                                &scheduler_snapshot.settings,
                                needs_fallback,
                            );
                            last_presented_key = Some(presentation_key);
                        }
                        Err(error) => {
                            eprintln!("scheduler could not mark reminder presented: {error}");
                        }
                    }
                }
                Some(CLOCK_RECHECK_INTERVAL_MS)
            }
        };

        match next_wait {
            Some(delay_ms) => {
                wait_for_wake_or_timeout(
                    &mut wake_receiver,
                    delay_ms.clamp(0, CLOCK_RECHECK_INTERVAL_MS),
                )
                .await;
            }
            None => {
                let _ = wake_receiver.changed().await;
            }
        }
    }
}

async fn wait_for_wake_or_timeout(receiver: &mut watch::Receiver<u64>, delay_ms: i64) {
    let _ = tokio::time::timeout(Duration::from_millis(delay_ms as u64), receiver.changed()).await;
}

fn emit_state_changed(app_handle: &AppHandle) {
    if let Err(error) = app_handle.emit("gym://state-changed", ()) {
        eprintln!("scheduler could not emit state event: {error}");
    }
    tray::refresh_from_database(app_handle);
}

#[cfg(test)]
mod tests {
    use super::super::clock::{Clock, FakeClock};
    use super::super::schedule::{reconcile_local, SchedulerDecision};
    use crate::domain::Settings;

    #[test]
    fn fake_clock_can_simulate_sleep_and_wake() {
        let clock = FakeClock::new(1_000);
        assert_eq!(clock.now_utc_ms(), 1_000);
        let settings = Settings {
            interval_minutes: 15,
            active_start_minutes: 0,
            active_end_minutes: 24 * 60,
            active_days_mask: 0b0111_1111,
            notifications_enabled: true,
            popup_enabled: true,
            sound_enabled: true,
            always_on_top: false,
            autostart_enabled: false,
            paused_until_utc: None,
            pause_indefinitely: false,
            next_due_at_utc: Some(500_000),
            created_at_utc: 0,
            updated_at_utc: 0,
        };

        assert!(matches!(
            reconcile_local(&settings, None, 3, clock.now_utc_ms()),
            SchedulerDecision::WaitUntil(500_000)
        ));

        clock.set(900_000);
        assert!(matches!(
            reconcile_local(&settings, None, 3, clock.now_utc_ms()),
            SchedulerDecision::CreateReminder { .. }
        ));
    }
}
