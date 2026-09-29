import type { AppSnapshot } from "./types";

export type DashboardState = "waiting" | "pending" | "paused";

export function dashboardState(snapshot: AppSnapshot): DashboardState {
  if (snapshot.pendingReminder) return "pending";
  if (
    snapshot.settings.pauseIndefinitely ||
    snapshot.settings.pausedUntilUtc !== null
  ) {
    return "paused";
  }
  return "waiting";
}

export function dashboardStateLabel(state: DashboardState): string {
  return {
    waiting: "Ожидание",
    pending: "Пора тренироваться",
    paused: "Пауза",
  }[state];
}

export function formatCountdown(targetUtc: number | null, nowUtc: number): string {
  if (targetUtc === null) return "—";
  const seconds = Math.max(0, Math.ceil((targetUtc - nowUtc) / 1_000));
  const hours = Math.floor(seconds / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  const remainder = seconds % 60;
  return hours > 0
    ? `${hours}:${minutes.toString().padStart(2, "0")}:${remainder
        .toString()
        .padStart(2, "0")}`
    : `${minutes}:${remainder.toString().padStart(2, "0")}`;
}
