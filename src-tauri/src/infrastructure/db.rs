use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::{
    domain::{
        AppSnapshot, CompleteReminderInput, CreateExerciseInput, Exercise, HistoryDay,
        HistoryWorkout, PauseRemindersInput, Reminder, ReminderStatus, SchedulerSnapshot, Settings,
        SnoozeReminderInput, UpdateExerciseInput, UpdateSettingsInput, WorkoutEntry,
    },
    error::{AppError, AppResult},
};

use super::migrations;

const DEFAULT_INTERVAL_MINUTES: i64 = 15;
const DEFAULT_ACTIVE_START_MINUTES: i64 = 9 * 60;
const DEFAULT_ACTIVE_END_MINUTES: i64 = 19 * 60;
const DEFAULT_ACTIVE_DAYS_MASK: i64 = 0b0111_1111;
const MAX_HISTORY_RANGE_MS: i64 = 370 * 24 * 60 * 60 * 1_000;

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
    path: Arc<PathBuf>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        let connection = Connection::open(&path)?;
        Self::initialize(connection, path)
    }

    #[cfg(test)]
    fn in_memory() -> AppResult<Self> {
        let connection = Connection::open_in_memory()?;
        Self::initialize(connection, PathBuf::from(":memory:"))
    }

    fn initialize(mut connection: Connection, path: PathBuf) -> AppResult<Self> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        if path != Path::new(":memory:") {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "NORMAL")?;
        }
        migrations::migrate(&mut connection)?;
        seed_defaults(&mut connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path: Arc::new(path),
        })
    }

    fn connection(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| AppError::StateUnavailable)
    }

    pub fn snapshot(&self) -> AppResult<AppSnapshot> {
        let connection = self.connection()?;
        Ok(AppSnapshot {
            settings: query_settings(&connection)?,
            exercises: query_exercises(&connection)?,
            pending_reminder: query_pending_reminder(&connection)?,
            database_path: self.path.to_string_lossy().into_owned(),
        })
    }

    pub fn settings(&self) -> AppResult<Settings> {
        let connection = self.connection()?;
        query_settings(&connection)
    }

    pub fn update_settings(&self, input: UpdateSettingsInput) -> AppResult<Settings> {
        crate::domain::validate_settings(&input)?;

        let now = now_utc_ms();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = query_settings(&transaction)?;
        let next_due_at_utc = if current.pause_indefinitely || current.paused_until_utc.is_some() {
            None
        } else if current.interval_minutes != input.interval_minutes {
            Some(now + input.interval_minutes * 60_000)
        } else {
            current.next_due_at_utc
        };

        transaction.execute(
            "UPDATE app_settings SET
                interval_minutes = ?1,
                active_start_minutes = ?2,
                active_end_minutes = ?3,
                active_days_mask = ?4,
                notifications_enabled = ?5,
                popup_enabled = ?6,
                sound_enabled = ?7,
                always_on_top = ?8,
                autostart_enabled = ?9,
                next_due_at_utc = ?10,
                updated_at_utc = ?11
             WHERE id = 1",
            params![
                input.interval_minutes,
                input.active_start_minutes,
                input.active_end_minutes,
                input.active_days_mask,
                bool_to_int(input.notifications_enabled),
                bool_to_int(input.popup_enabled),
                bool_to_int(input.sound_enabled),
                bool_to_int(input.always_on_top),
                bool_to_int(input.autostart_enabled),
                next_due_at_utc,
                now,
            ],
        )?;
        transaction.commit()?;

        query_settings(&connection)
    }

    pub fn exercises(&self) -> AppResult<Vec<Exercise>> {
        let connection = self.connection()?;
        query_exercises(&connection)
    }

    pub fn create_exercise(&self, input: CreateExerciseInput) -> AppResult<Exercise> {
        let name = crate::domain::validate_exercise(&input.name, input.target_reps)?;
        let now = now_utc_ms();
        let id = Uuid::new_v4().to_string();
        let connection = self.connection()?;
        let position: i64 = connection.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM exercises",
            [],
            |row| row.get(0),
        )?;

        connection.execute(
            "INSERT INTO exercises
                (id, name, target_reps, position, enabled, created_at_utc, updated_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                id,
                name,
                input.target_reps,
                position,
                bool_to_int(input.enabled),
                now,
            ],
        )?;

        query_exercise_by_id(&connection, &id)
    }

    pub fn update_exercise(&self, input: UpdateExerciseInput) -> AppResult<Exercise> {
        let name = crate::domain::validate_exercise(&input.name, input.target_reps)?;
        let now = now_utc_ms();
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE exercises
             SET name = ?1, target_reps = ?2, enabled = ?3, updated_at_utc = ?4
             WHERE id = ?5",
            params![
                name,
                input.target_reps,
                bool_to_int(input.enabled),
                now,
                input.id,
            ],
        )?;

        if changed == 0 {
            return Err(AppError::NotFound("Упражнение не найдено".into()));
        }

        query_exercise_by_id(&connection, &input.id)
    }

    pub fn delete_exercise(&self, id: &str) -> AppResult<()> {
        let connection = self.connection()?;
        let changed = connection.execute("DELETE FROM exercises WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(AppError::NotFound("Упражнение не найдено".into()));
        }
        Ok(())
    }

    pub fn reorder_exercises(&self, ids: &[String]) -> AppResult<Vec<Exercise>> {
        let mut connection = self.connection()?;
        let existing_ids = query_exercises(&connection)?
            .into_iter()
            .map(|exercise| exercise.id)
            .collect::<Vec<_>>();
        let requested_set = ids.iter().cloned().collect::<HashSet<_>>();
        let existing_set = existing_ids.iter().cloned().collect::<HashSet<_>>();

        if ids.len() != existing_ids.len()
            || requested_set.len() != ids.len()
            || requested_set != existing_set
        {
            return Err(AppError::Validation(
                "Для сортировки нужно передать каждое упражнение ровно один раз".into(),
            ));
        }

        let transaction = connection.transaction()?;
        let now = now_utc_ms();
        for (position, id) in ids.iter().enumerate() {
            transaction.execute(
                "UPDATE exercises SET position = ?1, updated_at_utc = ?2 WHERE id = ?3",
                params![position as i64, now, id],
            )?;
        }
        transaction.commit()?;

        query_exercises(&connection)
    }

    pub(crate) fn scheduler_snapshot(&self) -> AppResult<SchedulerSnapshot> {
        let connection = self.connection()?;
        let enabled_exercise_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM exercises WHERE enabled = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(SchedulerSnapshot {
            settings: query_settings(&connection)?,
            pending_reminder: query_pending_reminder(&connection)?,
            enabled_exercise_count: enabled_exercise_count as usize,
        })
    }

    pub(crate) fn set_next_due(&self, next_due_at_utc: i64) -> AppResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE app_settings
             SET next_due_at_utc = ?1, updated_at_utc = ?2
             WHERE id = 1",
            params![next_due_at_utc, now_utc_ms()],
        )?;
        Ok(())
    }

    pub(crate) fn resume_schedule(&self, next_due_at_utc: i64) -> AppResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE app_settings
             SET paused_until_utc = NULL, pause_indefinitely = 0,
                 next_due_at_utc = ?1, updated_at_utc = ?2
             WHERE id = 1",
            params![next_due_at_utc, now_utc_ms()],
        )?;
        Ok(())
    }

    pub(crate) fn create_pending_reminder(
        &self,
        original_due_at_utc: i64,
        due_at_utc: i64,
        created_at_utc: i64,
    ) -> AppResult<Reminder> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        if let Some(existing) = query_pending_reminder(&transaction)? {
            return Ok(existing);
        }
        let enabled_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM exercises WHERE enabled = 1",
            [],
            |row| row.get(0),
        )?;
        if enabled_count == 0 {
            return Err(AppError::Validation(
                "Нужно включить хотя бы одно упражнение".into(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO reminders
                (id, status, original_due_at_utc, due_at_utc, created_at_utc)
             VALUES (?1, 'pending', ?2, ?3, ?4)",
            params![id, original_due_at_utc, due_at_utc, created_at_utc],
        )?;
        transaction.execute(
            "UPDATE app_settings SET next_due_at_utc = NULL, updated_at_utc = ?1 WHERE id = 1",
            [created_at_utc],
        )?;
        let reminder = query_reminder_by_id(&transaction, &id)?;
        transaction.commit()?;
        Ok(reminder)
    }

    pub(crate) fn mark_reminder_presented(
        &self,
        id: &str,
        presented_at_utc: i64,
    ) -> AppResult<Reminder> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE reminders
             SET first_presented_at_utc = COALESCE(first_presented_at_utc, ?1),
                 last_presented_at_utc = ?1
             WHERE id = ?2 AND status = 'pending'",
            params![presented_at_utc, id],
        )?;
        if changed == 0 {
            return Err(AppError::Conflict(
                "Напоминание уже не ожидает выполнения".into(),
            ));
        }
        query_reminder_by_id(&connection, id)
    }

    pub fn start_reminder_now(&self, now_utc_ms: i64) -> AppResult<Reminder> {
        self.create_pending_reminder(now_utc_ms, now_utc_ms, now_utc_ms)
    }

    pub fn snooze_reminder(
        &self,
        input: SnoozeReminderInput,
        now_utc_ms: i64,
    ) -> AppResult<Reminder> {
        if !(1..=60).contains(&input.minutes) {
            return Err(AppError::Validation(
                "Отложить напоминание можно на срок от 1 до 60 минут".into(),
            ));
        }
        let connection = self.connection()?;
        let due_at_utc = now_utc_ms + input.minutes * 60_000;
        let changed = connection.execute(
            "UPDATE reminders
             SET due_at_utc = ?1, snooze_count = snooze_count + 1
             WHERE id = ?2 AND status = 'pending'",
            params![due_at_utc, input.reminder_id],
        )?;
        if changed == 0 {
            return Err(AppError::Conflict(
                "Можно отложить только ожидающее напоминание".into(),
            ));
        }
        query_reminder_by_id(&connection, &input.reminder_id)
    }

    pub fn complete_reminder(
        &self,
        input: CompleteReminderInput,
        resolved_at_utc: i64,
        next_due_at_utc: i64,
    ) -> AppResult<Reminder> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let reminder = query_reminder_by_id(&transaction, &input.reminder_id)?;
        match reminder.status {
            ReminderStatus::Completed => return Ok(reminder),
            ReminderStatus::Skipped => {
                return Err(AppError::Conflict(
                    "Пропущенное напоминание нельзя завершить".into(),
                ));
            }
            ReminderStatus::Pending => {}
        }

        let enabled_exercises = query_enabled_exercises(&transaction)?;
        let input_by_exercise = input
            .entries
            .iter()
            .map(|entry| (entry.exercise_id.as_str(), entry.actual_reps))
            .collect::<HashMap<_, _>>();
        if enabled_exercises.is_empty()
            || input_by_exercise.len() != input.entries.len()
            || input_by_exercise.len() != enabled_exercises.len()
            || enabled_exercises
                .iter()
                .any(|exercise| !input_by_exercise.contains_key(exercise.id.as_str()))
        {
            return Err(AppError::Validation(
                "Нужно передать результат каждого активного упражнения ровно один раз".into(),
            ));
        }
        if input
            .entries
            .iter()
            .any(|entry| !(0..=10_000).contains(&entry.actual_reps))
        {
            return Err(AppError::Validation(
                "Фактическое количество повторений должно быть от 0 до 10000".into(),
            ));
        }

        for exercise in enabled_exercises {
            let actual_reps = input_by_exercise[exercise.id.as_str()];
            transaction.execute(
                "INSERT INTO workout_entries
                    (id, reminder_id, exercise_id, exercise_name_snapshot,
                     planned_reps, actual_reps, position_snapshot)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    Uuid::new_v4().to_string(),
                    &input.reminder_id,
                    &exercise.id,
                    &exercise.name,
                    exercise.target_reps,
                    actual_reps,
                    exercise.position,
                ],
            )?;
        }
        transaction.execute(
            "UPDATE reminders SET status = 'completed', resolved_at_utc = ?1 WHERE id = ?2",
            params![resolved_at_utc, &input.reminder_id],
        )?;
        transaction.execute(
            "UPDATE app_settings SET next_due_at_utc = ?1, updated_at_utc = ?2 WHERE id = 1",
            params![next_due_at_utc, resolved_at_utc],
        )?;
        let completed = query_reminder_by_id(&transaction, &input.reminder_id)?;
        transaction.commit()?;
        Ok(completed)
    }

    pub fn skip_reminder(
        &self,
        reminder_id: &str,
        resolved_at_utc: i64,
        next_due_at_utc: i64,
    ) -> AppResult<Reminder> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let reminder = query_reminder_by_id(&transaction, reminder_id)?;
        match reminder.status {
            ReminderStatus::Skipped => return Ok(reminder),
            ReminderStatus::Completed => {
                return Err(AppError::Conflict(
                    "Завершённое напоминание нельзя пропустить".into(),
                ));
            }
            ReminderStatus::Pending => {}
        }
        transaction.execute(
            "UPDATE reminders SET status = 'skipped', resolved_at_utc = ?1 WHERE id = ?2",
            params![resolved_at_utc, reminder_id],
        )?;
        transaction.execute(
            "UPDATE app_settings SET next_due_at_utc = ?1, updated_at_utc = ?2 WHERE id = 1",
            params![next_due_at_utc, resolved_at_utc],
        )?;
        let skipped = query_reminder_by_id(&transaction, reminder_id)?;
        transaction.commit()?;
        Ok(skipped)
    }

    pub fn history(&self, date_from: i64, date_to: i64) -> AppResult<HistoryDay> {
        if date_from >= date_to || date_to - date_from > MAX_HISTORY_RANGE_MS {
            return Err(AppError::Validation(
                "Период истории должен быть положительным и не длиннее 370 дней".into(),
            ));
        }

        let connection = self.connection()?;
        let resolved = {
            let mut statement = connection.prepare(
                "SELECT id, status, due_at_utc, resolved_at_utc
                 FROM reminders
                 WHERE status IN ('completed', 'skipped')
                   AND resolved_at_utc >= ?1 AND resolved_at_utc < ?2
                 ORDER BY resolved_at_utc DESC, id DESC",
            )?;
            let rows = statement
                .query_map(params![date_from, date_to], |row| {
                    let status: String = row.get(1)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        ReminderStatus::parse(&status)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };

        let mut workouts = Vec::with_capacity(resolved.len());
        for (reminder_id, status, due_at_utc, resolved_at_utc) in resolved {
            let entries = query_workout_entries(&connection, &reminder_id)?;
            let total_reps = entries.iter().map(|entry| entry.actual_reps).sum();
            workouts.push(HistoryWorkout {
                reminder_id,
                status,
                due_at_utc,
                resolved_at_utc,
                entries,
                total_reps,
            });
        }

        Ok(HistoryDay {
            date_from_utc: date_from,
            date_to_utc: date_to,
            completed_count: workouts
                .iter()
                .filter(|workout| workout.status == ReminderStatus::Completed)
                .count() as i64,
            skipped_count: workouts
                .iter()
                .filter(|workout| workout.status == ReminderStatus::Skipped)
                .count() as i64,
            total_reps: workouts.iter().map(|workout| workout.total_reps).sum(),
            workouts,
        })
    }

    pub fn pause_reminders(
        &self,
        input: PauseRemindersInput,
        now_utc_ms: i64,
    ) -> AppResult<Settings> {
        if input.indefinitely && input.until_utc.is_some() {
            return Err(AppError::Validation(
                "Для бессрочной паузы не нужно указывать время окончания".into(),
            ));
        }
        if !input.indefinitely && input.until_utc.is_none_or(|until| until <= now_utc_ms) {
            return Err(AppError::Validation(
                "Время окончания паузы должно быть в будущем".into(),
            ));
        }

        let connection = self.connection()?;
        connection.execute(
            "UPDATE app_settings
             SET paused_until_utc = ?1, pause_indefinitely = ?2,
                 next_due_at_utc = NULL, updated_at_utc = ?3
             WHERE id = 1",
            params![input.until_utc, bool_to_int(input.indefinitely), now_utc_ms,],
        )?;
        query_settings(&connection)
    }

    pub fn resume_reminders(&self, now_utc_ms: i64, next_due_at_utc: i64) -> AppResult<Settings> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE app_settings
             SET paused_until_utc = NULL, pause_indefinitely = 0,
                 next_due_at_utc = ?1, updated_at_utc = ?2
             WHERE id = 1",
            params![next_due_at_utc, now_utc_ms],
        )?;
        query_settings(&connection)
    }
}

fn seed_defaults(connection: &mut Connection) -> AppResult<()> {
    let now = now_utc_ms();
    let transaction = connection.transaction()?;
    let settings_created = transaction.execute(
        "INSERT INTO app_settings (
            id, interval_minutes, active_start_minutes, active_end_minutes,
            active_days_mask, notifications_enabled, popup_enabled, sound_enabled,
            always_on_top, autostart_enabled, paused_until_utc, pause_indefinitely,
            next_due_at_utc, created_at_utc, updated_at_utc
         ) VALUES (1, ?1, ?2, ?3, ?4, 1, 1, 1, 0, 0, NULL, 0, ?5, ?6, ?6)
         ON CONFLICT(id) DO NOTHING",
        params![
            DEFAULT_INTERVAL_MINUTES,
            DEFAULT_ACTIVE_START_MINUTES,
            DEFAULT_ACTIVE_END_MINUTES,
            DEFAULT_ACTIVE_DAYS_MASK,
            now + DEFAULT_INTERVAL_MINUTES * 60_000,
            now,
        ],
    )?;

    if settings_created == 1 {
        for (position, (name, target_reps)) in
            [("Приседания", 20_i64), ("Отжимания", 10), ("Пресс", 20)]
                .into_iter()
                .enumerate()
        {
            transaction.execute(
                "INSERT INTO exercises
                    (id, name, target_reps, position, enabled, created_at_utc, updated_at_utc)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    name,
                    target_reps,
                    position as i64,
                    now,
                ],
            )?;
        }
    }

    transaction.commit()?;
    Ok(())
}

fn query_settings(connection: &Connection) -> AppResult<Settings> {
    connection
        .query_row(
            "SELECT
                interval_minutes, active_start_minutes, active_end_minutes,
                active_days_mask, notifications_enabled, popup_enabled, sound_enabled,
                always_on_top, autostart_enabled, paused_until_utc,
                pause_indefinitely, next_due_at_utc, created_at_utc, updated_at_utc
             FROM app_settings WHERE id = 1",
            [],
            map_settings,
        )
        .map_err(AppError::from)
}

fn map_settings(row: &Row<'_>) -> rusqlite::Result<Settings> {
    Ok(Settings {
        interval_minutes: row.get(0)?,
        active_start_minutes: row.get(1)?,
        active_end_minutes: row.get(2)?,
        active_days_mask: row.get(3)?,
        notifications_enabled: row.get::<_, i64>(4)? != 0,
        popup_enabled: row.get::<_, i64>(5)? != 0,
        sound_enabled: row.get::<_, i64>(6)? != 0,
        always_on_top: row.get::<_, i64>(7)? != 0,
        autostart_enabled: row.get::<_, i64>(8)? != 0,
        paused_until_utc: row.get(9)?,
        pause_indefinitely: row.get::<_, i64>(10)? != 0,
        next_due_at_utc: row.get(11)?,
        created_at_utc: row.get(12)?,
        updated_at_utc: row.get(13)?,
    })
}

fn query_exercises(connection: &Connection) -> AppResult<Vec<Exercise>> {
    let mut statement = connection.prepare(
        "SELECT id, name, target_reps, position, enabled, created_at_utc, updated_at_utc
         FROM exercises ORDER BY position, created_at_utc, id",
    )?;
    let exercises = statement
        .query_map([], map_exercise)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(exercises)
}

fn query_enabled_exercises(connection: &Connection) -> AppResult<Vec<Exercise>> {
    let mut statement = connection.prepare(
        "SELECT id, name, target_reps, position, enabled, created_at_utc, updated_at_utc
         FROM exercises WHERE enabled = 1 ORDER BY position, created_at_utc, id",
    )?;
    let exercises = statement
        .query_map([], map_exercise)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(exercises)
}

fn query_exercise_by_id(connection: &Connection, id: &str) -> AppResult<Exercise> {
    connection
        .query_row(
            "SELECT id, name, target_reps, position, enabled, created_at_utc, updated_at_utc
             FROM exercises WHERE id = ?1",
            [id],
            map_exercise,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("Упражнение не найдено".into()))
}

fn map_exercise(row: &Row<'_>) -> rusqlite::Result<Exercise> {
    Ok(Exercise {
        id: row.get(0)?,
        name: row.get(1)?,
        target_reps: row.get(2)?,
        position: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at_utc: row.get(5)?,
        updated_at_utc: row.get(6)?,
    })
}

fn query_pending_reminder(connection: &Connection) -> AppResult<Option<Reminder>> {
    connection
        .query_row(
            "SELECT id, status, original_due_at_utc, due_at_utc,
                    first_presented_at_utc, last_presented_at_utc, resolved_at_utc,
                    snooze_count, created_at_utc
             FROM reminders WHERE status = 'pending' LIMIT 1",
            [],
            map_reminder,
        )
        .optional()
        .map_err(AppError::from)
}

fn query_reminder_by_id(connection: &Connection, id: &str) -> AppResult<Reminder> {
    connection
        .query_row(
            "SELECT id, status, original_due_at_utc, due_at_utc,
                    first_presented_at_utc, last_presented_at_utc, resolved_at_utc,
                    snooze_count, created_at_utc
             FROM reminders WHERE id = ?1",
            [id],
            map_reminder,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("Напоминание не найдено".into()))
}

fn map_reminder(row: &Row<'_>) -> rusqlite::Result<Reminder> {
    let status: String = row.get(1)?;
    let status = ReminderStatus::parse(&status).map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(Reminder {
        id: row.get(0)?,
        status,
        original_due_at_utc: row.get(2)?,
        due_at_utc: row.get(3)?,
        first_presented_at_utc: row.get(4)?,
        last_presented_at_utc: row.get(5)?,
        resolved_at_utc: row.get(6)?,
        snooze_count: row.get(7)?,
        created_at_utc: row.get(8)?,
    })
}

fn query_workout_entries(
    connection: &Connection,
    reminder_id: &str,
) -> AppResult<Vec<WorkoutEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, reminder_id, exercise_id, exercise_name_snapshot,
                planned_reps, actual_reps, position_snapshot
         FROM workout_entries WHERE reminder_id = ?1
         ORDER BY position_snapshot, id",
    )?;
    let entries = statement
        .query_map([reminder_id], |row| {
            Ok(WorkoutEntry {
                id: row.get(0)?,
                reminder_id: row.get(1)?,
                exercise_id: row.get(2)?,
                exercise_name_snapshot: row.get(3)?,
                planned_reps: row.get(4)?,
                actual_reps: row.get(5)?,
                position_snapshot: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn now_utc_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::WorkoutResultInput;

    fn new_exercise(name: &str, target_reps: i64) -> CreateExerciseInput {
        CreateExerciseInput {
            name: name.into(),
            target_reps,
            enabled: true,
        }
    }

    #[test]
    fn creates_schema_and_default_data_idempotently() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("gym.sqlite3");

        let database = Database::open(&path).unwrap();
        let snapshot = database.snapshot().unwrap();
        assert_eq!(snapshot.settings.interval_minutes, 15);
        assert_eq!(
            snapshot
                .exercises
                .iter()
                .map(|exercise| exercise.name.as_str())
                .collect::<Vec<_>>(),
            ["Приседания", "Отжимания", "Пресс"]
        );
        drop(database);

        let reopened = Database::open(&path).unwrap();
        assert_eq!(reopened.exercises().unwrap().len(), 3);
        let version: i64 = reopened
            .connection()
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, migrations::CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn supports_exercise_crud_and_atomic_reordering() {
        let database = Database::in_memory().unwrap();
        let created = database.create_exercise(new_exercise("Планка", 1)).unwrap();
        assert_eq!(created.position, 3);

        let updated = database
            .update_exercise(UpdateExerciseInput {
                id: created.id.clone(),
                name: "  Планка на локтях ".into(),
                target_reps: 2,
                enabled: false,
            })
            .unwrap();
        assert_eq!(updated.name, "Планка на локтях");
        assert!(!updated.enabled);

        let before = database.exercises().unwrap();
        assert!(database.reorder_exercises(&[before[0].id.clone()]).is_err());
        assert_eq!(database.exercises().unwrap(), before);

        let reversed_ids = before
            .iter()
            .rev()
            .map(|exercise| exercise.id.clone())
            .collect::<Vec<_>>();
        let reordered = database.reorder_exercises(&reversed_ids).unwrap();
        assert_eq!(
            reordered
                .iter()
                .map(|exercise| exercise.id.as_str())
                .collect::<Vec<_>>(),
            reversed_ids.iter().map(String::as_str).collect::<Vec<_>>()
        );

        database.delete_exercise(&created.id).unwrap();
        assert_eq!(database.exercises().unwrap().len(), 3);
    }

    #[test]
    fn changing_interval_recalculates_next_due_time() {
        let database = Database::in_memory().unwrap();
        let previous = database.settings().unwrap();
        let updated = database
            .update_settings(UpdateSettingsInput {
                interval_minutes: 30,
                active_start_minutes: previous.active_start_minutes,
                active_end_minutes: previous.active_end_minutes,
                active_days_mask: previous.active_days_mask,
                notifications_enabled: previous.notifications_enabled,
                popup_enabled: previous.popup_enabled,
                sound_enabled: previous.sound_enabled,
                always_on_top: previous.always_on_top,
                autostart_enabled: previous.autostart_enabled,
            })
            .unwrap();

        assert_eq!(updated.interval_minutes, 30);
        assert!(updated.next_due_at_utc > previous.next_due_at_utc);
    }

    #[test]
    fn workout_snapshot_survives_exercise_rename_and_delete() {
        let database = Database::in_memory().unwrap();
        let exercise = database.exercises().unwrap().into_iter().next().unwrap();
        let now = now_utc_ms();
        {
            let connection = database.connection().unwrap();
            connection
                .execute(
                    "INSERT INTO reminders
                        (id, status, original_due_at_utc, due_at_utc, resolved_at_utc, created_at_utc)
                     VALUES ('reminder-1', 'completed', ?1, ?1, ?1, ?1)",
                    [now],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO workout_entries
                        (id, reminder_id, exercise_id, exercise_name_snapshot,
                         planned_reps, actual_reps, position_snapshot)
                     VALUES ('entry-1', 'reminder-1', ?1, ?2, 20, 18, 0)",
                    params![&exercise.id, &exercise.name],
                )
                .unwrap();
        }

        database.delete_exercise(&exercise.id).unwrap();
        let connection = database.connection().unwrap();
        let (exercise_id, snapshot_name): (Option<String>, String) = connection
            .query_row(
                "SELECT exercise_id, exercise_name_snapshot FROM workout_entries WHERE id = 'entry-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(exercise_id, None);
        assert_eq!(snapshot_name, "Приседания");
    }

    #[test]
    fn database_rejects_a_second_pending_reminder() {
        let database = Database::in_memory().unwrap();
        let connection = database.connection().unwrap();
        let now = now_utc_ms();
        connection
            .execute(
                "INSERT INTO reminders
                    (id, status, original_due_at_utc, due_at_utc, created_at_utc)
                 VALUES ('pending-1', 'pending', ?1, ?1, ?1)",
                [now],
            )
            .unwrap();

        let second = connection.execute(
            "INSERT INTO reminders
                (id, status, original_due_at_utc, due_at_utc, created_at_utc)
             VALUES ('pending-2', 'pending', ?1, ?1, ?1)",
            [now],
        );
        assert!(second.is_err());
    }

    #[test]
    fn failed_workout_transaction_rolls_back_reminder() {
        let database = Database::in_memory().unwrap();
        let exercise = database.exercises().unwrap().into_iter().next().unwrap();
        let mut connection = database.connection().unwrap();
        let now = now_utc_ms();

        let result = (|| -> rusqlite::Result<()> {
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO reminders
                    (id, status, original_due_at_utc, due_at_utc, resolved_at_utc, created_at_utc)
                 VALUES ('rollback-reminder', 'completed', ?1, ?1, ?1, ?1)",
                [now],
            )?;
            transaction.execute(
                "INSERT INTO workout_entries
                    (id, reminder_id, exercise_id, exercise_name_snapshot,
                     planned_reps, actual_reps, position_snapshot)
                 VALUES ('invalid-entry', 'rollback-reminder', ?1, ?2, 20, 10001, 0)",
                params![exercise.id, exercise.name],
            )?;
            transaction.commit()
        })();

        assert!(result.is_err());
        let reminder_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM reminders WHERE id = 'rollback-reminder'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reminder_count, 0);
    }

    fn completion_input(database: &Database, reminder_id: &str) -> CompleteReminderInput {
        CompleteReminderInput {
            reminder_id: reminder_id.into(),
            entries: database
                .exercises()
                .unwrap()
                .into_iter()
                .filter(|exercise| exercise.enabled)
                .map(|exercise| WorkoutResultInput {
                    exercise_id: exercise.id,
                    actual_reps: exercise.target_reps,
                })
                .collect(),
        }
    }

    #[test]
    fn start_now_is_idempotent_while_a_reminder_is_pending() {
        let database = Database::in_memory().unwrap();
        let first = database.start_reminder_now(1_000).unwrap();
        let second = database.start_reminder_now(2_000).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(database.snapshot().unwrap().pending_reminder, Some(first));
        assert_eq!(database.settings().unwrap().next_due_at_utc, None);
    }

    #[test]
    fn completing_a_reminder_is_transactional_and_idempotent() {
        let database = Database::in_memory().unwrap();
        let pending = database.start_reminder_now(1_000).unwrap();
        let input = completion_input(&database, &pending.id);

        let completed = database
            .complete_reminder(input.clone(), 2_000, 902_000)
            .unwrap();
        let repeated = database.complete_reminder(input, 3_000, 1_803_000).unwrap();

        assert_eq!(completed.status, ReminderStatus::Completed);
        assert_eq!(completed, repeated);
        assert_eq!(database.settings().unwrap().next_due_at_utc, Some(902_000));
        let connection = database.connection().unwrap();
        let entry_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM workout_entries WHERE reminder_id = ?1",
                [&pending.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(entry_count, 3);
    }

    #[test]
    fn history_returns_daily_totals_and_snapshot_entries() {
        let database = Database::in_memory().unwrap();
        let completed_reminder = database.start_reminder_now(1_000).unwrap();
        let input = completion_input(&database, &completed_reminder.id);
        let expected_total: i64 = input.entries.iter().map(|entry| entry.actual_reps).sum();
        database.complete_reminder(input, 2_000, 902_000).unwrap();

        let skipped_reminder = database.start_reminder_now(3_000).unwrap();
        database
            .skip_reminder(&skipped_reminder.id, 4_000, 904_000)
            .unwrap();

        let history = database.history(0, 5_000).unwrap();
        assert_eq!(history.completed_count, 1);
        assert_eq!(history.skipped_count, 1);
        assert_eq!(history.total_reps, expected_total);
        assert_eq!(history.workouts.len(), 2);
        assert_eq!(history.workouts[0].status, ReminderStatus::Skipped);
        assert!(history.workouts[0].entries.is_empty());
        assert_eq!(history.workouts[1].entries.len(), 3);
        assert_eq!(
            history.workouts[1].entries[0].exercise_name_snapshot,
            "Приседания"
        );
    }

    #[test]
    fn history_rejects_invalid_or_excessive_ranges() {
        let database = Database::in_memory().unwrap();
        assert!(database.history(1_000, 1_000).is_err());
        assert!(database.history(0, MAX_HISTORY_RANGE_MS + 1).is_err());
    }

    #[test]
    fn snooze_and_skip_keep_a_single_reminder_and_skip_is_idempotent() {
        let database = Database::in_memory().unwrap();
        let pending = database.start_reminder_now(1_000).unwrap();
        let snoozed = database
            .snooze_reminder(
                SnoozeReminderInput {
                    reminder_id: pending.id.clone(),
                    minutes: 5,
                },
                2_000,
            )
            .unwrap();
        assert_eq!(snoozed.due_at_utc, 302_000);
        assert_eq!(snoozed.snooze_count, 1);

        let skipped = database.skip_reminder(&pending.id, 4_000, 904_000).unwrap();
        let repeated = database
            .skip_reminder(&pending.id, 5_000, 1_805_000)
            .unwrap();
        assert_eq!(skipped, repeated);
        assert_eq!(skipped.status, ReminderStatus::Skipped);
        assert!(database.snapshot().unwrap().pending_reminder.is_none());
        assert_eq!(database.settings().unwrap().next_due_at_utc, Some(904_000));
    }

    #[test]
    fn pause_and_resume_persist_scheduler_state() {
        let database = Database::in_memory().unwrap();
        let paused = database
            .pause_reminders(
                PauseRemindersInput {
                    until_utc: Some(61_000),
                    indefinitely: false,
                },
                1_000,
            )
            .unwrap();
        assert_eq!(paused.paused_until_utc, Some(61_000));
        assert_eq!(paused.next_due_at_utc, None);

        let resumed = database.resume_reminders(2_000, 902_000).unwrap();
        assert_eq!(resumed.paused_until_utc, None);
        assert!(!resumed.pause_indefinitely);
        assert_eq!(resumed.next_due_at_utc, Some(902_000));
    }

    #[test]
    fn scheduler_snapshot_counts_only_enabled_exercises() {
        let database = Database::in_memory().unwrap();
        let exercise = database.exercises().unwrap().remove(0);
        database
            .update_exercise(UpdateExerciseInput {
                id: exercise.id,
                name: exercise.name,
                target_reps: exercise.target_reps,
                enabled: false,
            })
            .unwrap();

        assert_eq!(
            database
                .scheduler_snapshot()
                .unwrap()
                .enabled_exercise_count,
            2
        );
    }
}
