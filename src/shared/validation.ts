import type { UpdateSettingsInput } from "./types";

export function validateRepetitions(value: number): string | null {
  if (!Number.isInteger(value) || value < 0 || value > 10_000) {
    return "Количество повторений должно быть целым числом от 0 до 10000";
  }
  return null;
}

export function validateExercise(name: string, targetReps: number): string | null {
  const normalized = name.trim();
  if (!normalized) return "Введите название упражнения";
  if ([...normalized].length > 80) {
    return "Название упражнения не может быть длиннее 80 символов";
  }
  return validateRepetitions(targetReps);
}

export function validateSettings(input: UpdateSettingsInput): string | null {
  if (
    !Number.isInteger(input.intervalMinutes) ||
    input.intervalMinutes < 1 ||
    input.intervalMinutes > 240
  ) {
    return "Интервал должен быть целым числом от 1 до 240 минут";
  }
  if (input.activeStartMinutes >= input.activeEndMinutes) {
    return "Начало активного времени должно быть раньше окончания";
  }
  if (input.activeDaysMask === 0) return "Выберите хотя бы один активный день";
  return null;
}
