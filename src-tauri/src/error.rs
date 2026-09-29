use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;
pub type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("database state is unavailable")]
    StateUnavailable,
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("system integration error: {0}")]
    System(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        let code = match error {
            AppError::Validation(_) => "validation_error",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::StateUnavailable => "state_unavailable",
            AppError::Database(_) => "database_error",
            AppError::Io(_) => "filesystem_error",
            AppError::System(_) => "system_error",
        };

        Self {
            code,
            message: error.to_string(),
        }
    }
}
