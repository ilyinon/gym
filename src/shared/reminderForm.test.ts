import { describe, expect, it } from "vitest";
import type { Exercise } from "./types";
import {
  completeReminderInput,
  initialWorkoutResults,
  validateWorkoutResults,
} from "./reminderForm";

const exercises: Exercise[] = [
  {
    id: "squats",
    name: "Приседания",
    targetReps: 20,
    position: 0,
    enabled: true,
    createdAtUtc: 0,
    updatedAtUtc: 0,
  },
  {
    id: "disabled",
    name: "Планка",
    targetReps: 1,
    position: 1,
    enabled: false,
    createdAtUtc: 0,
    updatedAtUtc: 0,
  },
];

describe("reminder form", () => {
  it("prefills only enabled exercises with their plan", () => {
    expect(initialWorkoutResults(exercises)).toEqual({ squats: 20 });
  });

  it("rejects fractions and values outside the allowed range", () => {
    expect(validateWorkoutResults(exercises, { squats: 2.5 })).toContain("целое");
    expect(validateWorkoutResults(exercises, { squats: 10_001 })).toContain("10000");
    expect(validateWorkoutResults(exercises, { squats: 0 })).toBeNull();
  });

  it("builds one result for every enabled exercise", () => {
    expect(completeReminderInput("reminder", exercises, { squats: 18 })).toEqual({
      reminderId: "reminder",
      entries: [{ exerciseId: "squats", actualReps: 18 }],
    });
  });
});
