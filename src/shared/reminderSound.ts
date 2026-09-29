export interface ReminderAudio {
  currentTime: number;
  preload: string;
  volume: number;
  load(): void;
  play(): Promise<void>;
}

export const REMINDER_SOUND_URL = "/sounds/workout-reminder.wav";

export function prepareReminderAudio(
  createAudio: (source: string) => ReminderAudio = (source) => new Audio(source),
): ReminderAudio {
  const audio = createAudio(REMINDER_SOUND_URL);
  audio.preload = "auto";
  audio.volume = 0.9;
  audio.load();
  return audio;
}

export function playReminderAudio(audio: ReminderAudio): void {
  audio.currentTime = 0;
  void audio.play().catch(() => undefined);
}
