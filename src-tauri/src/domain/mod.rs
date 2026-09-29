mod exercise;
mod history;
mod reminder;
mod settings;
mod workout;

pub(crate) use exercise::validate_exercise;
pub use exercise::{CreateExerciseInput, Exercise, UpdateExerciseInput};
pub use history::{HistoryDay, HistoryWorkout};
pub use reminder::{PauseRemindersInput, Reminder, ReminderStatus, SnoozeReminderInput};
pub(crate) use settings::validate_settings;
pub use settings::{Settings, UpdateSettingsInput};
pub use workout::{CompleteReminderInput, WorkoutEntry, WorkoutResultInput};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub settings: Settings,
    pub exercises: Vec<Exercise>,
    pub pending_reminder: Option<Reminder>,
    pub database_path: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SchedulerSnapshot {
    pub settings: Settings,
    pub pending_reminder: Option<Reminder>,
    pub enabled_exercise_count: usize,
}
