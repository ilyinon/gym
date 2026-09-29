import { describe, expect, it } from "vitest";
import { dashboardState, dashboardStateLabel, formatCountdown } from "./dashboard";
import type { AppSnapshot } from "./types";

const snapshot = {
  settings: { pauseIndefinitely: false, pausedUntilUtc: null },
  pendingReminder: null,
} as AppSnapshot;

describe("dashboard presentation", () => {
  it("shows waiting, pending and paused states", () => {
    expect(dashboardState(snapshot)).toBe("waiting");
    expect(dashboardState({ ...snapshot, pendingReminder: {} as never })).toBe(
      "pending",
    );
    expect(
      dashboardState({
        ...snapshot,
        settings: { ...snapshot.settings, pauseIndefinitely: true },
      }),
    ).toBe("paused");
    expect(dashboardStateLabel("pending")).toBe("Пора тренироваться");
  });

  it("formats a live countdown without negative values", () => {
    expect(formatCountdown(3_661_000, 0)).toBe("1:01:01");
    expect(formatCountdown(59_000, 0)).toBe("0:59");
    expect(formatCountdown(1_000, 2_000)).toBe("0:00");
  });
});
