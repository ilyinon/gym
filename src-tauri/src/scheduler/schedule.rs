use chrono::{DateTime, Datelike, Days, Duration, Local, LocalResult, TimeZone, Timelike, Utc};

use crate::domain::{Reminder, Settings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerDecision {
    WaitForChange,
    WaitUntil(i64),
    SetNextDue(i64),
    ResumeAfterPause {
        next_due_at_utc: i64,
    },
    CreateReminder {
        original_due_at_utc: i64,
        due_at_utc: i64,
    },
    PresentReminder(Reminder),
}

pub fn reconcile_local(
    settings: &Settings,
    pending_reminder: Option<&Reminder>,
    enabled_exercise_count: usize,
    now_utc_ms: i64,
) -> SchedulerDecision {
    let now = DateTime::<Utc>::from_timestamp_millis(now_utc_ms)
        .expect("scheduler timestamp must be valid")
        .with_timezone(&Local);
    reconcile_at(settings, pending_reminder, enabled_exercise_count, now)
}

pub fn reconcile_at<Tz>(
    settings: &Settings,
    pending_reminder: Option<&Reminder>,
    enabled_exercise_count: usize,
    now: DateTime<Tz>,
) -> SchedulerDecision
where
    Tz: TimeZone + Clone,
{
    let now_utc_ms = now.timestamp_millis();

    if let Some(reminder) = pending_reminder {
        return if reminder.due_at_utc <= now_utc_ms {
            SchedulerDecision::PresentReminder(reminder.clone())
        } else {
            SchedulerDecision::WaitUntil(reminder.due_at_utc)
        };
    }

    if settings.pause_indefinitely {
        return SchedulerDecision::WaitForChange;
    }
    if let Some(paused_until) = settings.paused_until_utc {
        if paused_until > now_utc_ms {
            return SchedulerDecision::WaitUntil(paused_until);
        }
        return SchedulerDecision::ResumeAfterPause {
            next_due_at_utc: next_due_from(settings, now),
        };
    }
    if enabled_exercise_count == 0 {
        return SchedulerDecision::WaitForChange;
    }

    let Some(next_due_at_utc) = settings.next_due_at_utc else {
        return SchedulerDecision::SetNextDue(next_due_from(settings, now));
    };
    let normalized_due_at = normalize_timestamp(settings, next_due_at_utc, now.timezone());
    if normalized_due_at != next_due_at_utc {
        return SchedulerDecision::SetNextDue(normalized_due_at);
    }

    if next_due_at_utc <= now_utc_ms {
        SchedulerDecision::CreateReminder {
            original_due_at_utc: next_due_at_utc,
            due_at_utc: now_utc_ms,
        }
    } else {
        SchedulerDecision::WaitUntil(next_due_at_utc)
    }
}

pub fn next_due_from_utc(settings: &Settings, from_utc_ms: i64) -> i64 {
    let from = DateTime::<Utc>::from_timestamp_millis(from_utc_ms)
        .expect("scheduler timestamp must be valid")
        .with_timezone(&Local);
    next_due_from(settings, from)
}

pub fn next_due_from<Tz>(settings: &Settings, from: DateTime<Tz>) -> i64
where
    Tz: TimeZone + Clone,
{
    let proposed = from + Duration::minutes(settings.interval_minutes);
    normalize_at(settings, proposed).timestamp_millis()
}

pub fn is_active_at<Tz>(settings: &Settings, at: &DateTime<Tz>) -> bool
where
    Tz: TimeZone,
{
    let weekday_bit = 1_i64 << at.weekday().num_days_from_monday();
    let minute_of_day = i64::from(at.hour()) * 60 + i64::from(at.minute());
    settings.active_days_mask & weekday_bit != 0
        && minute_of_day >= settings.active_start_minutes
        && minute_of_day < settings.active_end_minutes
}

pub fn normalize_at<Tz>(settings: &Settings, at: DateTime<Tz>) -> DateTime<Tz>
where
    Tz: TimeZone + Clone,
{
    if is_active_at(settings, &at) {
        return at;
    }
    next_active_at(settings, at.clone()).unwrap_or(at)
}

pub fn next_active_at<Tz>(settings: &Settings, from: DateTime<Tz>) -> Option<DateTime<Tz>>
where
    Tz: TimeZone + Clone,
{
    if is_active_at(settings, &from) {
        return Some(from);
    }

    for day_offset in 0..=7 {
        let date = from.date_naive().checked_add_days(Days::new(day_offset))?;
        let weekday_bit = 1_i64 << date.weekday().num_days_from_monday();
        if settings.active_days_mask & weekday_bit == 0 {
            continue;
        }

        let hour = u32::try_from(settings.active_start_minutes / 60).ok()?;
        let minute = u32::try_from(settings.active_start_minutes % 60).ok()?;
        let local_start = date.and_hms_opt(hour, minute, 0)?;
        let candidate = resolve_local_time(&from.timezone(), local_start)?;
        if candidate > from {
            return Some(candidate);
        }
    }
    None
}

fn normalize_timestamp<Tz>(settings: &Settings, timestamp: i64, timezone: Tz) -> i64
where
    Tz: TimeZone + Clone,
{
    let at = DateTime::<Utc>::from_timestamp_millis(timestamp)
        .expect("scheduler timestamp must be valid")
        .with_timezone(&timezone);
    normalize_at(settings, at).timestamp_millis()
}

fn resolve_local_time<Tz>(timezone: &Tz, start: chrono::NaiveDateTime) -> Option<DateTime<Tz>>
where
    Tz: TimeZone,
{
    for offset_minutes in 0..=180 {
        let candidate = start + Duration::minutes(offset_minutes);
        match timezone.from_local_datetime(&candidate) {
            LocalResult::Single(value) => return Some(value),
            LocalResult::Ambiguous(first, second) => return Some(first.min(second)),
            LocalResult::None => continue,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};

    use super::*;

    fn settings() -> Settings {
        Settings {
            interval_minutes: 15,
            active_start_minutes: 9 * 60,
            active_end_minutes: 19 * 60,
            active_days_mask: 0b0001_1111,
            notifications_enabled: true,
            popup_enabled: true,
            sound_enabled: true,
            always_on_top: false,
            autostart_enabled: false,
            paused_until_utc: None,
            pause_indefinitely: false,
            next_due_at_utc: None,
            created_at_utc: 0,
            updated_at_utc: 0,
        }
    }

    fn monday(hour: u32, minute: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 28, hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn recognizes_active_period_boundaries() {
        let settings = settings();
        assert!(!is_active_at(&settings, &monday(8, 59)));
        assert!(is_active_at(&settings, &monday(9, 0)));
        assert!(is_active_at(&settings, &monday(18, 59)));
        assert!(!is_active_at(&settings, &monday(19, 0)));
    }

    #[test]
    fn moves_evening_due_time_to_next_active_day() {
        let settings = settings();
        assert_eq!(
            normalize_at(&settings, monday(20, 0)),
            FixedOffset::east_opt(3 * 3600)
                .unwrap()
                .with_ymd_and_hms(2026, 9, 29, 9, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn skips_disabled_weekend() {
        let settings = settings();
        let friday_evening = FixedOffset::east_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 2, 20, 0, 0)
            .unwrap();
        let monday_morning = FixedOffset::east_opt(3 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 5, 9, 0, 0)
            .unwrap();
        assert_eq!(normalize_at(&settings, friday_evening), monday_morning);
    }

    #[test]
    fn interval_is_counted_from_resolution_time() {
        assert_eq!(
            next_due_from(&settings(), monday(10, 0)),
            monday(10, 15).timestamp_millis()
        );
    }

    #[test]
    fn overdue_state_creates_only_one_reminder_decision() {
        let mut settings = settings();
        settings.next_due_at_utc = Some(monday(10, 0).timestamp_millis());
        let decision = reconcile_at(&settings, None, 3, monday(11, 0));
        assert!(matches!(decision, SchedulerDecision::CreateReminder { .. }));
    }

    #[test]
    fn pending_reminder_is_presented_after_restart() {
        let settings = settings();
        let reminder = Reminder {
            id: "pending".into(),
            status: crate::domain::ReminderStatus::Pending,
            original_due_at_utc: monday(10, 0).timestamp_millis(),
            due_at_utc: monday(10, 0).timestamp_millis(),
            first_presented_at_utc: Some(monday(10, 0).timestamp_millis()),
            last_presented_at_utc: Some(monday(10, 0).timestamp_millis()),
            resolved_at_utc: None,
            snooze_count: 0,
            created_at_utc: monday(10, 0).timestamp_millis(),
        };
        assert!(matches!(
            reconcile_at(&settings, Some(&reminder), 3, monday(10, 30)),
            SchedulerDecision::PresentReminder(_)
        ));
    }

    #[test]
    fn expired_pause_restarts_interval() {
        let mut settings = settings();
        settings.paused_until_utc = Some(monday(9, 0).timestamp_millis());
        assert!(matches!(
            reconcile_at(&settings, None, 3, monday(10, 0)),
            SchedulerDecision::ResumeAfterPause { .. }
        ));
    }

    #[test]
    fn indefinite_pause_waits_for_an_explicit_change() {
        let mut settings = settings();
        settings.pause_indefinitely = true;
        assert_eq!(
            reconcile_at(&settings, None, 3, monday(10, 0)),
            SchedulerDecision::WaitForChange
        );
    }

    #[test]
    fn timezone_changes_active_window_interpretation() {
        let mut settings = settings();
        settings.active_days_mask = 0b0111_1111;
        let utc_due = DateTime::parse_from_rfc3339("2026-09-28T06:30:00Z")
            .unwrap()
            .timestamp_millis();
        let plus_three = FixedOffset::east_opt(3 * 3600).unwrap();
        let plus_one = FixedOffset::east_opt(3600).unwrap();

        assert_eq!(normalize_timestamp(&settings, utc_due, plus_three), utc_due);
        assert_eq!(
            normalize_timestamp(&settings, utc_due, plus_one),
            DateTime::parse_from_rfc3339("2026-09-28T08:00:00Z")
                .unwrap()
                .timestamp_millis()
        );
    }
}
