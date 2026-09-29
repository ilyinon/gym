import type { HistoryWorkout } from "./types";

export function localDayRange(value: Date): { dateFrom: number; dateTo: number } {
  const start = new Date(value.getFullYear(), value.getMonth(), value.getDate());
  const end = new Date(value.getFullYear(), value.getMonth(), value.getDate() + 1);
  return { dateFrom: start.getTime(), dateTo: end.getTime() };
}

export function shiftLocalDay(value: Date, offset: number): Date {
  return new Date(value.getFullYear(), value.getMonth(), value.getDate() + offset);
}

export function historyStatusLabel(workout: HistoryWorkout): string {
  return workout.status === "completed" ? "Выполнено" : "Пропущено";
}

export function formatHistoryDay(value: Date): string {
  return new Intl.DateTimeFormat("ru-RU", {
    weekday: "long",
    day: "numeric",
    month: "long",
  }).format(value);
}
