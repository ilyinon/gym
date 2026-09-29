export interface Settings {
  intervalMinutes: number;
  activeStartMinutes: number;
  activeEndMinutes: number;
  activeDaysMask: number;
  notificationsEnabled: boolean;
  popupEnabled: boolean;
  soundEnabled: boolean;
  alwaysOnTop: boolean;
  autostartEnabled: boolean;
  pausedUntilUtc: number | null;
  pauseIndefinitely: boolean;
  nextDueAtUtc: number | null;
  createdAtUtc: number;
  updatedAtUtc: number;
}

export type UpdateSettingsInput = Pick<
  Settings,
  | "intervalMinutes"
  | "activeStartMinutes"
  | "activeEndMinutes"
  | "activeDaysMask"
  | "notificationsEnabled"
  | "popupEnabled"
  | "soundEnabled"
  | "alwaysOnTop"
  | "autostartEnabled"
>;

export interface Exercise {
  id: string;
  name: string;
  targetReps: number;
  position: number;
  enabled: boolean;
  createdAtUtc: number;
  updatedAtUtc: number;
}

export interface CreateExerciseInput {
  name: string;
  targetReps: number;
  enabled: boolean;
}

export type UpdateExerciseInput = Pick<
  Exercise,
  "id" | "name" | "targetReps" | "enabled"
>;

export type ReminderStatus = "pending" | "completed" | "skipped";

export interface Reminder {
  id: string;
  status: ReminderStatus;
  originalDueAtUtc: number;
  dueAtUtc: number;
  firstPresentedAtUtc: number | null;
  lastPresentedAtUtc: number | null;
  resolvedAtUtc: number | null;
  snoozeCount: number;
  createdAtUtc: number;
}

export interface WorkoutResultInput {
  exerciseId: string;
  actualReps: number;
}

export interface CompleteReminderInput {
  reminderId: string;
  entries: WorkoutResultInput[];
}

export interface SnoozeReminderInput {
  reminderId: string;
  minutes: number;
}

export interface PauseRemindersInput {
  untilUtc: number | null;
  indefinitely: boolean;
}

export interface AppSnapshot {
  settings: Settings;
  exercises: Exercise[];
  pendingReminder: Reminder | null;
  databasePath: string;
}

export interface AppInfo {
  version: string;
  databasePath: string;
  logPath: string;
  fatalError: string | null;
}

export interface WorkoutEntry {
  id: string;
  reminderId: string;
  exerciseId: string | null;
  exerciseNameSnapshot: string;
  plannedReps: number;
  actualReps: number;
  positionSnapshot: number;
}

export interface HistoryWorkout {
  reminderId: string;
  status: "completed" | "skipped";
  dueAtUtc: number;
  resolvedAtUtc: number;
  entries: WorkoutEntry[];
  totalReps: number;
}

export interface HistoryDay {
  dateFromUtc: number;
  dateToUtc: number;
  completedCount: number;
  skippedCount: number;
  totalReps: number;
  workouts: HistoryWorkout[];
}
