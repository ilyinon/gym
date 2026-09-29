use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Pending,
    Completed,
    Skipped,
}

impl ReminderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Skipped => "skipped",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "completed" => Ok(Self::Completed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(AppError::Database(rusqlite::Error::InvalidQuery)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub status: ReminderStatus,
    pub original_due_at_utc: i64,
    pub due_at_utc: i64,
    pub first_presented_at_utc: Option<i64>,
    pub last_presented_at_utc: Option<i64>,
    pub resolved_at_utc: Option<i64>,
    pub snooze_count: i64,
    pub created_at_utc: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnoozeReminderInput {
    pub reminder_id: String,
    pub minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseRemindersInput {
    pub until_utc: Option<i64>,
    pub indefinitely: bool,
}
