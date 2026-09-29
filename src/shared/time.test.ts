import { describe, expect, it } from "vitest";
import { minutesToTime, timeToMinutes } from "./time";

describe("time conversion", () => {
  it("converts minutes to an input time", () => {
    expect(minutesToTime(9 * 60 + 5)).toBe("09:05");
  });

  it("converts an input time to minutes", () => {
    expect(timeToMinutes("19:30")).toBe(19 * 60 + 30);
  });

  it("rejects an invalid time", () => {
    expect(() => timeToMinutes("25:00")).toThrow("Некорректное время");
  });
});
