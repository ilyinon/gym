import { describe, expect, it } from "vitest";
import { validateExercise, validateRepetitions, validateSettings } from "./validation";
import type { UpdateSettingsInput } from "./types";

const settings: UpdateSettingsInput = {
  intervalMinutes: 15,
  activeStartMinutes: 540,
  activeEndMinutes: 1_140,
  activeDaysMask: 127,
  notificationsEnabled: true,
  popupEnabled: true,
  soundEnabled: true,
  alwaysOnTop: false,
  autostartEnabled: false,
};

describe("frontend validation", () => {
  it("validates numeric fields", () => {
    expect(validateRepetitions(10)).toBeNull();
    expect(validateRepetitions(1.5)).toContain("целым");
    expect(validateRepetitions(10_001)).toContain("10000");
    expect(validateSettings({ ...settings, intervalMinutes: 0 })).toContain("240");
  });

  it("validates exercises and schedules", () => {
    expect(validateExercise("  ", 10)).toContain("название");
    expect(validateExercise("Приседания", 20)).toBeNull();
    expect(validateSettings({ ...settings, activeDaysMask: 0 })).toContain("день");
    expect(validateSettings({ ...settings, activeStartMinutes: 1_140 })).toContain(
      "раньше",
    );
  });
});
