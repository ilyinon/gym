use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutEntry {
    pub id: String,
    pub reminder_id: String,
    pub exercise_id: Option<String>,
    pub exercise_name_snapshot: String,
    pub planned_reps: i64,
    pub actual_reps: i64,
    pub position_snapshot: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutResultInput {
    pub exercise_id: String,
    pub actual_reps: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteReminderInput {
    pub reminder_id: String,
    pub entries: Vec<WorkoutResultInput>,
}
