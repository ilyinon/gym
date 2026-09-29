import { describe, expect, it } from "vitest";
import { historyStatusLabel, localDayRange, shiftLocalDay } from "./history";
import type { HistoryWorkout } from "./types";

describe("history helpers", () => {
  it("builds adjacent local day ranges", () => {
    const day = new Date(2026, 8, 29, 12);
    const next = shiftLocalDay(day, 1);
    expect(localDayRange(day).dateTo).toBe(localDayRange(next).dateFrom);
  });

  it("keeps the stored workout status labels", () => {
    const workout = { status: "completed" } as HistoryWorkout;
    expect(historyStatusLabel(workout)).toBe("Выполнено");
    expect(historyStatusLabel({ ...workout, status: "skipped" })).toBe("Пропущено");
  });
});
