use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const CURRENT_SCHEMA_VERSION: i64 = 1;

const MIGRATION_0001: &str = r#"
CREATE TABLE app_settings (
    id                      INTEGER PRIMARY KEY CHECK (id = 1),
    interval_minutes        INTEGER NOT NULL CHECK (interval_minutes BETWEEN 1 AND 240),
    active_start_minutes    INTEGER NOT NULL CHECK (active_start_minutes BETWEEN 0 AND 1439),
    active_end_minutes      INTEGER NOT NULL CHECK (active_end_minutes BETWEEN 1 AND 1440),
    active_days_mask        INTEGER NOT NULL CHECK (active_days_mask BETWEEN 1 AND 127),
    notifications_enabled   INTEGER NOT NULL CHECK (notifications_enabled IN (0, 1)),
    popup_enabled           INTEGER NOT NULL CHECK (popup_enabled IN (0, 1)),
    sound_enabled           INTEGER NOT NULL CHECK (sound_enabled IN (0, 1)),
    always_on_top           INTEGER NOT NULL CHECK (always_on_top IN (0, 1)),
    autostart_enabled       INTEGER NOT NULL CHECK (autostart_enabled IN (0, 1)),
    paused_until_utc        INTEGER,
    pause_indefinitely      INTEGER NOT NULL DEFAULT 0 CHECK (pause_indefinitely IN (0, 1)),
    next_due_at_utc         INTEGER,
    created_at_utc          INTEGER NOT NULL,
    updated_at_utc          INTEGER NOT NULL,
    CHECK (active_start_minutes < active_end_minutes)
);

CREATE TABLE exercises (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 80),
    target_reps     INTEGER NOT NULL CHECK (target_reps BETWEEN 0 AND 10000),
    position        INTEGER NOT NULL,
    enabled         INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    created_at_utc  INTEGER NOT NULL,
    updated_at_utc  INTEGER NOT NULL
);

CREATE INDEX exercises_position_idx ON exercises(position, created_at_utc);

CREATE TABLE reminders (
    id                      TEXT PRIMARY KEY,
    status                  TEXT NOT NULL CHECK (status IN ('pending', 'completed', 'skipped')),
    original_due_at_utc     INTEGER NOT NULL,
    due_at_utc              INTEGER NOT NULL,
    first_presented_at_utc  INTEGER,
    last_presented_at_utc   INTEGER,
    resolved_at_utc         INTEGER,
    snooze_count            INTEGER NOT NULL DEFAULT 0 CHECK (snooze_count >= 0),
    created_at_utc          INTEGER NOT NULL
);

CREATE UNIQUE INDEX only_one_pending_reminder_idx
    ON reminders(status)
    WHERE status = 'pending';

CREATE INDEX reminders_due_at_idx ON reminders(due_at_utc);

CREATE TABLE workout_entries (
    id                      TEXT PRIMARY KEY,
    reminder_id             TEXT NOT NULL REFERENCES reminders(id) ON DELETE CASCADE,
    exercise_id             TEXT REFERENCES exercises(id) ON DELETE SET NULL,
    exercise_name_snapshot  TEXT NOT NULL,
    planned_reps            INTEGER NOT NULL CHECK (planned_reps BETWEEN 0 AND 10000),
    actual_reps             INTEGER NOT NULL CHECK (actual_reps BETWEEN 0 AND 10000),
    position_snapshot       INTEGER NOT NULL
);

CREATE INDEX workout_entries_reminder_idx
    ON workout_entries(reminder_id, position_snapshot);
"#;

pub fn migrate(connection: &mut Connection) -> AppResult<()> {
    let current_version: i64 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current_version > CURRENT_SCHEMA_VERSION {
        return Err(AppError::Conflict(format!(
            "Версия базы {current_version} новее поддерживаемой {CURRENT_SCHEMA_VERSION}"
        )));
    }

    if current_version < 1 {
        let transaction = connection.transaction()?;
        transaction.execute_batch(MIGRATION_0001)?;
        transaction.pragma_update(None, "user_version", 1_i64)?;
        transaction.commit()?;
    }

    Ok(())
}
