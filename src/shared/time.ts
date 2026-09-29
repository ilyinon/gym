export function minutesToTime(minutes: number): string {
  const safeMinutes = Math.min(24 * 60, Math.max(0, Math.trunc(minutes)));
  const hours = Math.floor(safeMinutes / 60);
  const remainder = safeMinutes % 60;
  return `${hours.toString().padStart(2, "0")}:${remainder
    .toString()
    .padStart(2, "0")}`;
}

export function timeToMinutes(value: string): number {
  const match = /^(\d{2}):(\d{2})$/.exec(value);
  if (!match) {
    throw new Error("Некорректное время");
  }
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 23 || minutes > 59) {
    throw new Error("Некорректное время");
  }
  return hours * 60 + minutes;
}

export function formatTimestamp(timestamp: number | null): string {
  if (timestamp === null) {
    return "не назначено";
  }
  return new Intl.DateTimeFormat("ru-RU", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}
