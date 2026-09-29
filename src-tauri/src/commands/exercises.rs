use tauri::State;

use crate::{
    domain::{CreateExerciseInput, Exercise, UpdateExerciseInput},
    error::CommandResult,
    infrastructure::Database,
    scheduler::SchedulerHandle,
};

#[tauri::command]
pub fn list_exercises(database: State<'_, Database>) -> CommandResult<Vec<Exercise>> {
    database.exercises().map_err(Into::into)
}

#[tauri::command]
pub fn create_exercise(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    input: CreateExerciseInput,
) -> CommandResult<Exercise> {
    let exercise = database.create_exercise(input)?;
    scheduler.wake();
    Ok(exercise)
}

#[tauri::command]
pub fn update_exercise(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    input: UpdateExerciseInput,
) -> CommandResult<Exercise> {
    let exercise = database.update_exercise(input)?;
    scheduler.wake();
    Ok(exercise)
}

#[tauri::command]
pub fn reorder_exercises(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    ids: Vec<String>,
) -> CommandResult<Vec<Exercise>> {
    let exercises = database.reorder_exercises(&ids)?;
    scheduler.wake();
    Ok(exercises)
}

#[tauri::command]
pub fn delete_exercise(
    database: State<'_, Database>,
    scheduler: State<'_, SchedulerHandle>,
    id: String,
) -> CommandResult<()> {
    database.delete_exercise(&id)?;
    scheduler.wake();
    Ok(())
}
