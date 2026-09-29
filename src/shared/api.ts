import { invoke } from "@tauri-apps/api/core";
import type {
  AppSnapshot,
  CompleteReminderInput,
  CreateExerciseInput,
  Exercise,
  HistoryDay,
  PauseRemindersInput,
  Reminder,
  Settings,
  SnoozeReminderInput,
  UpdateExerciseInput,
  UpdateSettingsInput,
} from "./types";

export const gymApi = {
  getSnapshot: () => invoke<AppSnapshot>("get_app_snapshot"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (input: UpdateSettingsInput) =>
    invoke<Settings>("update_settings", { input }),
  listExercises: () => invoke<Exercise[]>("list_exercises"),
  createExercise: (input: CreateExerciseInput) =>
    invoke<Exercise>("create_exercise", { input }),
  updateExercise: (input: UpdateExerciseInput) =>
    invoke<Exercise>("update_exercise", { input }),
  reorderExercises: (ids: string[]) => invoke<Exercise[]>("reorder_exercises", { ids }),
  deleteExercise: (id: string) => invoke<void>("delete_exercise", { id }),
  startWorkoutNow: () => invoke<Reminder>("start_workout_now"),
  completeReminder: (input: CompleteReminderInput) =>
    invoke<Reminder>("complete_reminder", { input }),
  snoozeReminder: (input: SnoozeReminderInput) =>
    invoke<Reminder>("snooze_reminder", { input }),
  skipReminder: (reminderId: string) =>
    invoke<Reminder>("skip_reminder", { reminderId }),
  pauseReminders: (input: PauseRemindersInput) =>
    invoke<Settings>("pause_reminders", { input }),
  resumeReminders: () => invoke<Settings>("resume_reminders"),
  getHistory: (dateFrom: number, dateTo: number) =>
    invoke<HistoryDay>("get_history", { dateFrom, dateTo }),
  hideReminderWindow: () => invoke<void>("hide_reminder_window"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  return "Неизвестная ошибка приложения";
}
