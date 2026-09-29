use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const MIN_INTERVAL_MINUTES: i64 = 1;
pub const MAX_INTERVAL_MINUTES: i64 = 240;
pub const MINUTES_PER_DAY: i64 = 24 * 60;
pub const ALL_DAYS_MASK: i64 = 0b0111_1111;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub interval_minutes: i64,
    pub active_start_minutes: i64,
    pub active_end_minutes: i64,
    pub active_days_mask: i64,
    pub notifications_enabled: bool,
    pub popup_enabled: bool,
    pub sound_enabled: bool,
    pub always_on_top: bool,
    pub autostart_enabled: bool,
    pub paused_until_utc: Option<i64>,
    pub pause_indefinitely: bool,
    pub next_due_at_utc: Option<i64>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub interval_minutes: i64,
    pub active_start_minutes: i64,
    pub active_end_minutes: i64,
    pub active_days_mask: i64,
    pub notifications_enabled: bool,
    pub popup_enabled: bool,
    pub sound_enabled: bool,
    pub always_on_top: bool,
    pub autostart_enabled: bool,
}

pub fn validate_settings(input: &UpdateSettingsInput) -> AppResult<()> {
    if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&input.interval_minutes) {
        return Err(AppError::Validation(format!(
            "Интервал должен быть от {MIN_INTERVAL_MINUTES} до {MAX_INTERVAL_MINUTES} минут"
        )));
    }
    if !(0..MINUTES_PER_DAY).contains(&input.active_start_minutes)
        || !(1..=MINUTES_PER_DAY).contains(&input.active_end_minutes)
        || input.active_start_minutes >= input.active_end_minutes
    {
        return Err(AppError::Validation(
            "Активное время должно находиться внутри одних суток, а начало — быть раньше окончания"
                .into(),
        ));
    }
    if input.active_days_mask <= 0 || input.active_days_mask & !ALL_DAYS_MASK != 0 {
        return Err(AppError::Validation(
            "Нужно выбрать хотя бы один корректный день недели".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> UpdateSettingsInput {
        UpdateSettingsInput {
            interval_minutes: 15,
            active_start_minutes: 9 * 60,
            active_end_minutes: 19 * 60,
            active_days_mask: ALL_DAYS_MASK,
            notifications_enabled: true,
            popup_enabled: true,
            sound_enabled: true,
            always_on_top: false,
            autostart_enabled: false,
        }
    }

    #[test]
    fn accepts_valid_settings() {
        assert!(validate_settings(&valid_input()).is_ok());
    }

    #[test]
    fn rejects_invalid_interval_schedule_and_days() {
        let mut input = valid_input();
        input.interval_minutes = 0;
        assert!(validate_settings(&input).is_err());

        input = valid_input();
        input.active_start_minutes = input.active_end_minutes;
        assert!(validate_settings(&input).is_err());

        input = valid_input();
        input.active_days_mask = 0;
        assert!(validate_settings(&input).is_err());
    }
}
