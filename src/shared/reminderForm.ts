import type { CompleteReminderInput, Exercise } from "./types";

export type WorkoutResults = Record<string, number>;

export function initialWorkoutResults(exercises: Exercise[]): WorkoutResults {
  return Object.fromEntries(
    exercises
      .filter((exercise) => exercise.enabled)
      .map((exercise) => [exercise.id, exercise.targetReps]),
  );
}

export function validateWorkoutResults(
  exercises: Exercise[],
  results: WorkoutResults,
): string | null {
  const activeExercises = exercises.filter((exercise) => exercise.enabled);
  if (activeExercises.length === 0) return "Нет активных упражнений";

  for (const exercise of activeExercises) {
    const value = results[exercise.id];
    if (!Number.isInteger(value) || value < 0 || value > 10_000) {
      return `Для «${exercise.name}» укажите целое число от 0 до 10000`;
    }
  }
  return null;
}

export function completeReminderInput(
  reminderId: string,
  exercises: Exercise[],
  results: WorkoutResults,
): CompleteReminderInput {
  return {
    reminderId,
    entries: exercises
      .filter((exercise) => exercise.enabled)
      .map((exercise) => ({
        exerciseId: exercise.id,
        actualReps: results[exercise.id],
      })),
  };
}
