import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import "./App.css";
import { APP_NAME } from "./appMetadata";
import { errorMessage, gymApi } from "./shared/api";
import {
  dashboardState,
  dashboardStateLabel,
  formatCountdown,
} from "./shared/dashboard";
import {
  formatHistoryDay,
  historyStatusLabel,
  localDayRange,
  shiftLocalDay,
} from "./shared/history";
import { validateWorkoutResults } from "./shared/reminderForm";
import { formatTimestamp, minutesToTime, timeToMinutes } from "./shared/time";
import type {
  AppSnapshot,
  Exercise,
  HistoryDay,
  UpdateSettingsInput,
} from "./shared/types";
import { validateExercise, validateSettings } from "./shared/validation";

const DAYS = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"] as const;
const NOTIFICATION_EXPLAINED_KEY = "gym.notification-permission-explained";

function editableSettings(snapshot: AppSnapshot): UpdateSettingsInput {
  const settings = snapshot.settings;
  return {
    intervalMinutes: settings.intervalMinutes,
    activeStartMinutes: settings.activeStartMinutes,
    activeEndMinutes: settings.activeEndMinutes,
    activeDaysMask: settings.activeDaysMask,
    notificationsEnabled: settings.notificationsEnabled,
    popupEnabled: settings.popupEnabled,
    soundEnabled: settings.soundEnabled,
    alwaysOnTop: settings.alwaysOnTop,
    autostartEnabled: settings.autostartEnabled,
  };
}

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [settings, setSettings] = useState<UpdateSettingsInput | null>(null);
  const [newName, setNewName] = useState("");
  const [newTarget, setNewTarget] = useState(10);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [workoutResults, setWorkoutResults] = useState<Record<string, number>>({});
  const [notificationPermission, setNotificationPermission] = useState<
    "checking" | "prompt" | "granted" | "denied"
  >("checking");
  const [now, setNow] = useState(() => Date.now());
  const [historyDate, setHistoryDate] = useState(() => new Date());
  const [history, setHistory] = useState<HistoryDay | null>(null);
  const [todayHistory, setTodayHistory] = useState<HistoryDay | null>(null);

  const applySnapshot = useCallback((nextSnapshot: AppSnapshot) => {
    setSnapshot(nextSnapshot);
    setSettings(editableSettings(nextSnapshot));
    setWorkoutResults(
      nextSnapshot.pendingReminder
        ? Object.fromEntries(
            nextSnapshot.exercises
              .filter((exercise) => exercise.enabled)
              .map((exercise) => [exercise.id, exercise.targetReps]),
          )
        : {},
    );
  }, []);

  const loadSnapshot = useCallback(async () => {
    applySnapshot(await gymApi.getSnapshot());
  }, [applySnapshot]);

  const loadHistory = useCallback(async (date: Date) => {
    const { dateFrom, dateTo } = localDayRange(date);
    setHistory(await gymApi.getHistory(dateFrom, dateTo));
  }, []);

  const loadTodayHistory = useCallback(async () => {
    const { dateFrom, dateTo } = localDayRange(new Date());
    setTodayHistory(await gymApi.getHistory(dateFrom, dateTo));
  }, []);

  useEffect(() => {
    let active = true;
    void gymApi.getSnapshot().then(
      (nextSnapshot) => {
        if (!active) return;
        applySnapshot(nextSnapshot);
      },
      (reason: unknown) => {
        if (active) setError(errorMessage(reason));
      },
    );
    return () => {
      active = false;
    };
  }, [applySnapshot]);

  useEffect(() => {
    const { dateFrom, dateTo } = localDayRange(historyDate);
    void gymApi
      .getHistory(dateFrom, dateTo)
      .then(setHistory, (reason: unknown) => setError(errorMessage(reason)));
  }, [historyDate]);

  useEffect(() => {
    const { dateFrom, dateTo } = localDayRange(new Date());
    void gymApi
      .getHistory(dateFrom, dateTo)
      .then(setTodayHistory, (reason: unknown) => setError(errorMessage(reason)));
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    async function subscribe() {
      for (const eventName of ["gym://state-changed", "gym://reminder-due"]) {
        const unlisten = await listen(eventName, () => {
          void Promise.all([
            loadSnapshot(),
            loadHistory(historyDate),
            loadTodayHistory(),
          ]).catch((reason: unknown) => setError(errorMessage(reason)));
        });
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      }
    }

    void subscribe().catch((reason: unknown) => setError(errorMessage(reason)));
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [historyDate, loadHistory, loadSnapshot, loadTodayHistory]);

  useEffect(() => {
    if (!snapshot?.settings.notificationsEnabled) return;
    if (window.localStorage.getItem(NOTIFICATION_EXPLAINED_KEY) !== "yes") {
      setTimeout(() => setNotificationPermission("prompt"), 0);
      return;
    }
    void isPermissionGranted().then(
      (granted) => setNotificationPermission(granted ? "granted" : "prompt"),
      () => setNotificationPermission("prompt"),
    );
  }, [snapshot?.settings.notificationsEnabled]);

  async function execute(action: () => Promise<unknown>, successMessage: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await action();
      await Promise.all([loadSnapshot(), loadHistory(historyDate), loadTodayHistory()]);
      setNotice(successMessage);
    } catch (reason: unknown) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  function patchExercise(id: string, patch: Partial<Exercise>) {
    setSnapshot((current) =>
      current
        ? {
            ...current,
            exercises: current.exercises.map((exercise) =>
              exercise.id === id ? { ...exercise, ...patch } : exercise,
            ),
          }
        : current,
    );
  }

  function moveExercise(index: number, offset: -1 | 1) {
    if (!snapshot) return;
    const target = index + offset;
    if (target < 0 || target >= snapshot.exercises.length) return;
    const reordered = [...snapshot.exercises];
    [reordered[index], reordered[target]] = [reordered[target], reordered[index]];
    void execute(async () => {
      await gymApi.reorderExercises(reordered.map((exercise) => exercise.id));
    }, "Порядок упражнений сохранён");
  }

  const currentState = useMemo(
    () => (snapshot ? dashboardState(snapshot) : "waiting"),
    [snapshot],
  );
  const countdownTarget = snapshot?.pendingReminder
    ? snapshot.pendingReminder.dueAtUtc
    : (snapshot?.settings.nextDueAtUtc ?? null);
  const historyIsToday =
    localDayRange(historyDate).dateFrom === localDayRange(new Date()).dateFrom;

  if (!snapshot || !settings) {
    return (
      <main className="loading-screen">
        <div>
          <span className="eyebrow">{APP_NAME}</span>
          <h1>Загружаем локальные данные…</h1>
          {error && <p className="message error-message">{error}</p>}
        </div>
      </main>
    );
  }

  return (
    <main className="app-shell">
      <header className="page-header">
        <div>
          <span className="eyebrow">Локальный помощник</span>
          <h1>{APP_NAME}</h1>
          <p>
            Двигайтесь регулярно, отмечайте результат и наблюдайте прогресс по дням.
          </p>
        </div>
        <div className="next-due">
          <span>
            {snapshot.pendingReminder
              ? "Активное напоминание"
              : "Следующее напоминание"}
          </span>
          <strong>
            {currentState === "paused"
              ? "На паузе"
              : formatCountdown(countdownTarget, now)}
          </strong>
          {currentState !== "paused" && (
            <small>{formatTimestamp(countdownTarget)}</small>
          )}
        </div>
      </header>

      {(error || notice) && (
        <div
          className={`message ${error ? "error-message" : "success-message"}`}
          role={error ? "alert" : "status"}
        >
          {error ?? notice}
        </div>
      )}

      {snapshot.settings.notificationsEnabled &&
        (notificationPermission === "prompt" ||
          notificationPermission === "denied") && (
          <section className="permission-banner" aria-labelledby="permission-title">
            <div>
              <strong id="permission-title">Системные уведомления</strong>
              <p>
                {notificationPermission === "denied"
                  ? "Разрешение не выдано. Gym продолжит показывать собственное окно."
                  : "Gym использует уведомления, чтобы напомнить о тренировке, пока вы работаете в другом приложении."}
              </p>
            </div>
            <button
              type="button"
              onClick={() => {
                void requestPermission().then(
                  (permission) => {
                    window.localStorage.setItem(NOTIFICATION_EXPLAINED_KEY, "yes");
                    setNotificationPermission(
                      permission === "granted" ? "granted" : "denied",
                    );
                  },
                  (reason: unknown) => setError(errorMessage(reason)),
                );
              }}
            >
              Разрешить
            </button>
          </section>
        )}

      <div className="content-grid">
        <section className="panel scheduler-panel" aria-labelledby="scheduler-title">
          <div className="section-heading">
            <div>
              <span className="section-number">Сегодня</span>
              <h2 id="scheduler-title">{dashboardStateLabel(currentState)}</h2>
            </div>
            <div className="today-summary" aria-label="Итоги за сегодня">
              <span>
                <strong>{todayHistory?.completedCount ?? 0}</strong> тренировок
              </span>
              <span>
                <strong>{todayHistory?.totalReps ?? 0}</strong> повторений
              </span>
            </div>
          </div>

          {snapshot.pendingReminder ? (
            <div className="workout-form">
              <p className="status-copy">
                Напоминание на {formatTimestamp(snapshot.pendingReminder.dueAtUtc)}
                {snapshot.pendingReminder.snoozeCount > 0 &&
                  ` · отложено ${snapshot.pendingReminder.snoozeCount} раз`}
              </p>
              {snapshot.exercises
                .filter((exercise) => exercise.enabled)
                .map((exercise) => (
                  <label className="workout-entry" key={exercise.id}>
                    <span>{exercise.name}</span>
                    <input
                      type="number"
                      min="0"
                      max="10000"
                      value={workoutResults[exercise.id] ?? exercise.targetReps}
                      onChange={(event) =>
                        setWorkoutResults((current) => ({
                          ...current,
                          [exercise.id]: Number(event.currentTarget.value),
                        }))
                      }
                    />
                  </label>
                ))}
              <div className="scheduler-actions">
                <button
                  className="primary-button"
                  type="button"
                  disabled={busy}
                  onClick={() =>
                    void execute(async () => {
                      const validationError = validateWorkoutResults(
                        snapshot.exercises,
                        workoutResults,
                      );
                      if (validationError) throw new Error(validationError);
                      await gymApi.completeReminder({
                        reminderId: snapshot.pendingReminder!.id,
                        entries: snapshot.exercises
                          .filter((exercise) => exercise.enabled)
                          .map((exercise) => ({
                            exerciseId: exercise.id,
                            actualReps:
                              workoutResults[exercise.id] ?? exercise.targetReps,
                          })),
                      });
                    }, "Тренировка сохранена")
                  }
                >
                  Готово
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() =>
                    void execute(async () => {
                      await gymApi.snoozeReminder({
                        reminderId: snapshot.pendingReminder!.id,
                        minutes: 5,
                      });
                    }, "Напоминание отложено на 5 минут")
                  }
                >
                  Через 5 минут
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() =>
                    void execute(
                      async () => gymApi.skipReminder(snapshot.pendingReminder!.id),
                      "Тренировка пропущена",
                    )
                  }
                >
                  Пропустить
                </button>
              </div>
            </div>
          ) : (
            <div>
              <p className="status-copy">
                {snapshot.settings.pauseIndefinitely
                  ? "Напоминания остановлены до ручного возобновления."
                  : snapshot.settings.pausedUntilUtc
                    ? `Пауза до ${formatTimestamp(snapshot.settings.pausedUntilUtc)}.`
                    : `Ожидание до ${formatTimestamp(snapshot.settings.nextDueAtUtc)}.`}
              </p>
              <div className="scheduler-actions">
                <button
                  className="primary-button"
                  type="button"
                  disabled={
                    busy || !snapshot.exercises.some((exercise) => exercise.enabled)
                  }
                  onClick={() =>
                    void execute(
                      async () => gymApi.startWorkoutNow(),
                      "Тренировка начата",
                    )
                  }
                >
                  Тренировка сейчас
                </button>
                {snapshot.settings.pauseIndefinitely ||
                snapshot.settings.pausedUntilUtc ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void execute(
                        async () => gymApi.resumeReminders(),
                        "Напоминания возобновлены",
                      )
                    }
                  >
                    Возобновить
                  </button>
                ) : (
                  <>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() =>
                        void execute(
                          async () =>
                            gymApi.pauseReminders({
                              untilUtc: Date.now() + 60 * 60 * 1000,
                              indefinitely: false,
                            }),
                          "Пауза на час",
                        )
                      }
                    >
                      Пауза на час
                    </button>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() =>
                        void execute(
                          async () =>
                            gymApi.pauseReminders({
                              untilUtc: null,
                              indefinitely: true,
                            }),
                          "Бессрочная пауза",
                        )
                      }
                    >
                      Пауза
                    </button>
                  </>
                )}
              </div>
            </div>
          )}
        </section>

        <section className="panel" aria-labelledby="schedule-title">
          <div className="section-heading">
            <div>
              <span className="section-number">Расписание</span>
              <h2 id="schedule-title">Расписание</h2>
            </div>
            <span className="pill">Локально</span>
          </div>

          <div className="field-grid">
            <label>
              Интервал, минут
              <input
                type="number"
                min="1"
                max="240"
                value={settings.intervalMinutes}
                onChange={(event) =>
                  setSettings({
                    ...settings,
                    intervalMinutes: Number(event.currentTarget.value),
                  })
                }
              />
            </label>
            <label>
              Начало
              <input
                type="time"
                value={minutesToTime(settings.activeStartMinutes)}
                onChange={(event) =>
                  setSettings({
                    ...settings,
                    activeStartMinutes: timeToMinutes(event.currentTarget.value),
                  })
                }
              />
            </label>
            <label>
              Окончание
              <input
                type="time"
                value={minutesToTime(settings.activeEndMinutes)}
                onChange={(event) =>
                  setSettings({
                    ...settings,
                    activeEndMinutes: timeToMinutes(event.currentTarget.value),
                  })
                }
              />
            </label>
          </div>

          <fieldset className="days-fieldset">
            <legend>Активные дни</legend>
            <div className="day-picker">
              {DAYS.map((day, index) => {
                const bit = 1 << index;
                const active = (settings.activeDaysMask & bit) !== 0;
                return (
                  <button
                    key={day}
                    type="button"
                    className={active ? "day active" : "day"}
                    aria-pressed={active}
                    onClick={() =>
                      setSettings({
                        ...settings,
                        activeDaysMask: settings.activeDaysMask ^ bit,
                      })
                    }
                  >
                    {day}
                  </button>
                );
              })}
            </div>
          </fieldset>

          <div className="toggle-grid">
            {[
              ["notificationsEnabled", "Системные уведомления"],
              ["popupEnabled", "Открывать окно"],
              ["soundEnabled", "Звук"],
              ["alwaysOnTop", "Окно поверх остальных"],
              ["autostartEnabled", "Автозапуск"],
            ].map(([key, label]) => (
              <label className="toggle" key={key}>
                <input
                  type="checkbox"
                  checked={Boolean(settings[key as keyof UpdateSettingsInput])}
                  onChange={(event) =>
                    setSettings({
                      ...settings,
                      [key]: event.currentTarget.checked,
                    })
                  }
                />
                <span>{label}</span>
              </label>
            ))}
          </div>

          <button
            className="primary-button"
            type="button"
            disabled={busy}
            onClick={() =>
              void execute(async () => {
                const validationError = validateSettings(settings);
                if (validationError) throw new Error(validationError);
                await gymApi.updateSettings(settings);
              }, "Настройки сохранены")
            }
          >
            Сохранить настройки
          </button>
        </section>

        <section className="panel" aria-labelledby="exercises-title">
          <div className="section-heading">
            <div>
              <span className="section-number">Набор</span>
              <h2 id="exercises-title">Упражнения</h2>
            </div>
            <span className="count">{snapshot.exercises.length}</span>
          </div>

          <div className="exercise-list">
            {snapshot.exercises.length === 0 && (
              <div className="empty-state">
                <strong>Упражнений пока нет</strong>
                <p>Добавьте первое упражнение ниже, чтобы запустить тренировку.</p>
              </div>
            )}
            {snapshot.exercises.map((exercise, index) => (
              <article className="exercise-row" key={exercise.id}>
                <div className="reorder-buttons" aria-label="Изменить порядок">
                  <button
                    type="button"
                    aria-label="Поднять упражнение"
                    disabled={busy || index === 0}
                    onClick={() => moveExercise(index, -1)}
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    aria-label="Опустить упражнение"
                    disabled={busy || index === snapshot.exercises.length - 1}
                    onClick={() => moveExercise(index, 1)}
                  >
                    ↓
                  </button>
                </div>
                <label className="exercise-name">
                  <span>Название</span>
                  <input
                    value={exercise.name}
                    maxLength={80}
                    onChange={(event) =>
                      patchExercise(exercise.id, { name: event.currentTarget.value })
                    }
                  />
                </label>
                <label className="exercise-target">
                  <span>План</span>
                  <input
                    type="number"
                    min="0"
                    max="10000"
                    value={exercise.targetReps}
                    onChange={(event) =>
                      patchExercise(exercise.id, {
                        targetReps: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label className="enabled-checkbox" title="Упражнение активно">
                  <input
                    type="checkbox"
                    checked={exercise.enabled}
                    onChange={(event) =>
                      patchExercise(exercise.id, {
                        enabled: event.currentTarget.checked,
                      })
                    }
                  />
                  <span>Вкл.</span>
                </label>
                <div className="row-actions">
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void execute(async () => {
                        const validationError = validateExercise(
                          exercise.name,
                          exercise.targetReps,
                        );
                        if (validationError) throw new Error(validationError);
                        await gymApi.updateExercise({
                          id: exercise.id,
                          name: exercise.name,
                          targetReps: exercise.targetReps,
                          enabled: exercise.enabled,
                        });
                      }, `«${exercise.name.trim()}» сохранено`)
                    }
                  >
                    Сохранить
                  </button>
                  <button
                    className="danger-button"
                    type="button"
                    disabled={busy}
                    onClick={() => {
                      if (!window.confirm(`Удалить упражнение «${exercise.name}»?`)) {
                        return;
                      }
                      void execute(
                        async () => gymApi.deleteExercise(exercise.id),
                        "Упражнение удалено",
                      );
                    }}
                  >
                    Удалить
                  </button>
                </div>
              </article>
            ))}
          </div>

          <form
            className="add-exercise"
            onSubmit={(event) => {
              event.preventDefault();
              void execute(async () => {
                const validationError = validateExercise(newName, newTarget);
                if (validationError) throw new Error(validationError);
                await gymApi.createExercise({
                  name: newName,
                  targetReps: newTarget,
                  enabled: true,
                });
                setNewName("");
                setNewTarget(10);
              }, "Упражнение добавлено");
            }}
          >
            <label>
              Новое упражнение
              <input
                value={newName}
                maxLength={80}
                placeholder="Например, выпады"
                onChange={(event) => setNewName(event.currentTarget.value)}
              />
            </label>
            <label>
              Повторения
              <input
                type="number"
                min="0"
                max="10000"
                value={newTarget}
                onChange={(event) => setNewTarget(Number(event.currentTarget.value))}
              />
            </label>
            <button className="secondary-button" type="submit" disabled={busy}>
              Добавить
            </button>
          </form>
        </section>

        <section className="panel history-panel" aria-labelledby="history-title">
          <div className="history-heading">
            <div>
              <span className="section-number">История</span>
              <h2 id="history-title">{formatHistoryDay(historyDate)}</h2>
            </div>
            <div className="date-navigation" aria-label="Выбор дня истории">
              <button
                type="button"
                aria-label="Предыдущий день"
                onClick={() => setHistoryDate((date) => shiftLocalDay(date, -1))}
              >
                ←
              </button>
              <button
                type="button"
                disabled={historyIsToday}
                onClick={() => setHistoryDate(new Date())}
              >
                Сегодня
              </button>
              <button
                type="button"
                aria-label="Следующий день"
                disabled={historyIsToday}
                onClick={() => setHistoryDate((date) => shiftLocalDay(date, 1))}
              >
                →
              </button>
            </div>
          </div>

          <div className="history-totals">
            <div>
              <strong>{history?.completedCount ?? 0}</strong>
              <span>выполнено</span>
            </div>
            <div>
              <strong>{history?.skippedCount ?? 0}</strong>
              <span>пропущено</span>
            </div>
            <div>
              <strong>{history?.totalReps ?? 0}</strong>
              <span>повторений</span>
            </div>
          </div>

          {!history ? (
            <div className="empty-state" role="status">
              <strong>Загружаем историю…</strong>
            </div>
          ) : history.workouts.length === 0 ? (
            <div className="empty-state">
              <strong>В этот день тренировок не было</strong>
              <p>Перелистните дату или начните новую тренировку.</p>
            </div>
          ) : (
            <div className="history-list">
              {history.workouts.map((workout) => (
                <details className="history-workout" key={workout.reminderId}>
                  <summary>
                    <span className={`history-status ${workout.status}`}>
                      {historyStatusLabel(workout)}
                    </span>
                    <time dateTime={new Date(workout.resolvedAtUtc).toISOString()}>
                      {new Intl.DateTimeFormat("ru-RU", {
                        hour: "2-digit",
                        minute: "2-digit",
                      }).format(new Date(workout.resolvedAtUtc))}
                    </time>
                    <strong>{workout.totalReps} повторений</strong>
                  </summary>
                  {workout.entries.length === 0 ? (
                    <p className="history-empty-entry">Результаты не записывались.</p>
                  ) : (
                    <div className="history-entries">
                      {workout.entries.map((entry) => (
                        <div key={entry.id}>
                          <span>{entry.exerciseNameSnapshot}</span>
                          <span>
                            план {entry.plannedReps} · факт {entry.actualReps}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </details>
              ))}
            </div>
          )}
        </section>
      </div>

      <footer className="database-path">
        <span>Локальная база</span>
        <code>{snapshot.databasePath}</code>
      </footer>
    </main>
  );
}

export default App;
