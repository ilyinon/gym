import { describe, expect, it, vi } from "vitest";
import {
  playReminderAudio,
  prepareReminderAudio,
  REMINDER_SOUND_URL,
  type ReminderAudio,
} from "./reminderSound";

function fakeAudio(): ReminderAudio {
  return {
    currentTime: 10,
    preload: "none",
    volume: 0,
    load: vi.fn(),
    play: vi.fn().mockResolvedValue(undefined),
  };
}

describe("reminder sound", () => {
  it("preloads the bundled audio at a comfortable volume", () => {
    const audio = fakeAudio();
    const createAudio = vi.fn(() => audio);

    expect(prepareReminderAudio(createAudio)).toBe(audio);
    expect(createAudio).toHaveBeenCalledWith(REMINDER_SOUND_URL);
    expect(audio.preload).toBe("auto");
    expect(audio.volume).toBe(0.9);
    expect(audio.load).toHaveBeenCalledOnce();
  });

  it("restarts the phrase when a reminder arrives", () => {
    const audio = fakeAudio();

    playReminderAudio(audio);

    expect(audio.currentTime).toBe(0);
    expect(audio.play).toHaveBeenCalledOnce();
  });
});
