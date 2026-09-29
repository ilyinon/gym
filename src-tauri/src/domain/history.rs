use serde::Serialize;

use super::{ReminderStatus, WorkoutEntry};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryWorkout {
    pub reminder_id: String,
    pub status: ReminderStatus,
    pub due_at_utc: i64,
    pub resolved_at_utc: i64,
    pub entries: Vec<WorkoutEntry>,
    pub total_reps: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    pub date_from_utc: i64,
    pub date_to_utc: i64,
    pub completed_count: i64,
    pub skipped_count: i64,
    pub total_reps: i64,
    pub workouts: Vec<HistoryWorkout>,
}
