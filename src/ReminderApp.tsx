import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage, gymApi } from "./shared/api";
import {
  completeReminderInput,
  initialWorkoutResults,
  validateWorkoutResults,
  type WorkoutResults,
} from "./shared/reminderForm";
import {
  playReminderAudio,
  prepareReminderAudio,
  type ReminderAudio,
} from "./shared/reminderSound";
import type { AppSnapshot } from "./shared/types";
import "./ReminderApp.css";

interface ReminderView {
  snapshot: AppSnapshot;
  reminderId: string;
  results: WorkoutResults;
}

function ReminderApp() {
  const [view, setView] = useState<ReminderView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const firstInput = useRef<HTMLInputElement>(null);
  const reminderAudio = useRef<ReminderAudio | null>(null);

  const applySnapshot = useCallback((snapshot: AppSnapshot) => {
    if (!snapshot.pendingReminder) {
      setView(null);
      void gymApi.hideReminderWindow();
      return;
    }

    setView((current) => ({
      snapshot,
      reminderId: snapshot.pendingReminder!.id,
      results:
        current?.reminderId === snapshot.pendingReminder!.id
          ? current.results
          : initialWorkoutResults(snapshot.exercises),
    }));
  }, []);

  const loadReminder = useCallback(async () => {
    applySnapshot(await gymApi.getSnapshot());
  }, [applySnapshot]);

  useEffect(() => {
    reminderAudio.current = prepareReminderAudio();
    return () => {
      reminderAudio.current = null;
    };
  }, []);

  useEffect(() => {
    void gymApi
      .getSnapshot()
      .then(applySnapshot, (reason: unknown) => setError(errorMessage(reason)));

    let disposed = false;
    const unlisteners: Array<() => void> = [];
    async function subscribe() {
      for (const eventName of ["gym://reminder-due", "gym://state-changed"]) {
        const unlisten = await listen(eventName, () => {
          void loadReminder()
            .then(() => {
              if (eventName === "gym://reminder-due") {
                window.requestAnimationFrame(() => {
                  firstInput.current?.focus();
                  firstInput.current?.select();
                });
              }
            })
            .catch((reason: unknown) => setError(errorMessage(reason)));
        });
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      }
      const unlistenSound = await listen("gym://play-reminder-sound", () => {
        if (reminderAudio.current) playReminderAudio(reminderAudio.current);
      });
      if (disposed) unlistenSound();
      else unlisteners.push(unlistenSound);
    }
    void subscribe().catch((reason: unknown) => setError(errorMessage(reason)));

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [applySnapshot, loadReminder]);

  useEffect(() => {
    firstInput.current?.focus();
    firstInput.current?.select();
  }, [view?.reminderId]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void gymApi.hideReminderWindow();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  async function execute(action: () => Promise<unknown>) {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await action();
      setBusy(false);
    } catch (reason: unknown) {
      setError(errorMessage(reason));
      setBusy(false);
    }
  }

  if (!view) {
    return (
      <main className="reminder-shell reminder-loading" aria-live="polite">
        <p>Загружаем тренировку…</p>
        {error && <p className="reminder-error">{error}</p>}
      </main>
    );
  }

  const activeExercises = view.snapshot.exercises.filter(
    (exercise) => exercise.enabled,
  );

  return (
    <main className="reminder-shell" aria-busy={busy}>
      <header className="reminder-header">
        <span className="reminder-kicker">Gym</span>
        <h1>Пора размяться</h1>
        <p>Следующий интервал начнётся после сохранения.</p>
      </header>

      <form
        className="reminder-form"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          const validationError = validateWorkoutResults(
            view.snapshot.exercises,
            view.results,
          );
          if (validationError) {
            setError(validationError);
            return;
          }
          void execute(() =>
            gymApi.completeReminder(
              completeReminderInput(
                view.reminderId,
                view.snapshot.exercises,
                view.results,
              ),
            ),
          );
        }}
      >
        <div className="reminder-exercises">
          {activeExercises.map((exercise, index) => (
            <label className="reminder-exercise" key={exercise.id}>
              <span className="exercise-copy">
                <strong>{exercise.name}</strong>
                <small>план {exercise.targetReps}</small>
              </span>
              <input
                ref={index === 0 ? firstInput : undefined}
                aria-label={`Фактический результат: ${exercise.name}`}
                type="number"
                inputMode="numeric"
                step="1"
                min="0"
                max="10000"
                value={view.results[exercise.id] ?? ""}
                disabled={busy}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  setView((current) => {
                    if (!current) return current;
                    const results = { ...current.results };
                    if (value === "") delete results[exercise.id];
                    else results[exercise.id] = Number(value);
                    return { ...current, results };
                  });
                }}
              />
            </label>
          ))}
        </div>

        {error && (
          <p className="reminder-error" role="alert" aria-live="assertive">
            {error}
          </p>
        )}

        <button className="reminder-primary" type="submit" disabled={busy}>
          {busy ? "Сохраняем…" : "Готово"}
        </button>
        <div className="reminder-secondary-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void execute(() =>
                gymApi.snoozeReminder({ reminderId: view.reminderId, minutes: 5 }),
              )
            }
          >
            Через 5 минут
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void execute(() => gymApi.skipReminder(view.reminderId))}
          >
            Пропустить
          </button>
        </div>
        <p className="keyboard-hint">Enter — сохранить · Esc — скрыть</p>
      </form>
    </main>
  );
}

export default ReminderApp;
