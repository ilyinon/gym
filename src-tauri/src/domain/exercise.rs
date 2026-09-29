use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const MAX_EXERCISE_NAME_CHARS: usize = 80;
pub const MAX_REPETITIONS: i64 = 10_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub target_reps: i64,
    pub position: i64,
    pub enabled: bool,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExerciseInput {
    pub name: String,
    pub target_reps: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExerciseInput {
    pub id: String,
    pub name: String,
    pub target_reps: i64,
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub fn validate_exercise(name: &str, target_reps: i64) -> AppResult<String> {
    let normalized_name = name.trim();
    let name_length = normalized_name.chars().count();

    if name_length == 0 {
        return Err(AppError::Validation(
            "Название упражнения не может быть пустым".into(),
        ));
    }
    if name_length > MAX_EXERCISE_NAME_CHARS {
        return Err(AppError::Validation(format!(
            "Название упражнения не может быть длиннее {MAX_EXERCISE_NAME_CHARS} символов"
        )));
    }
    if !(0..=MAX_REPETITIONS).contains(&target_reps) {
        return Err(AppError::Validation(format!(
            "Количество повторений должно быть от 0 до {MAX_REPETITIONS}"
        )));
    }

    Ok(normalized_name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_valid_name() {
        assert_eq!(
            validate_exercise("  Приседания  ", 20).unwrap(),
            "Приседания"
        );
    }

    #[test]
    fn rejects_empty_name_and_invalid_repetitions() {
        assert!(validate_exercise("   ", 20).is_err());
        assert!(validate_exercise("Пресс", -1).is_err());
        assert!(validate_exercise("Пресс", MAX_REPETITIONS + 1).is_err());
    }
}
