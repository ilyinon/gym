# Gym

Локальное desktop-приложение для регулярных напоминаний о коротких тренировках и учёта выполненных упражнений.

Целевые платформы: macOS и Fedora Linux. Приложение работает в системном трее, не требует аккаунта или интернета и хранит данные только на компьютере пользователя.

## Статус

Шаг 7 реализован: GitHub Actions собирает Fedora 44 x86_64 `.rpm`/`.AppImage` и universal macOS `.dmg`, проверяет manifest-версии, changelog, установку пакетов, подпись/notarization и SHA-256. Tag-driven workflow публикует релиз только после успешных Fedora и macOS jobs.

## Стек

- Tauri 2;
- React + TypeScript + Vite;
- Rust для фоновой логики и системной интеграции;
- SQLite для настроек и истории;
- npm с зафиксированным `package-lock.json`.

## Требования для разработки

- актуальная LTS-версия Node.js;
- Rust stable;
- системные зависимости Tauri для целевой платформы;
- Xcode Command Line Tools на macOS.

## Быстрый старт

```bash
npm ci
npm run tauri dev
```

Frontend без desktop-оболочки:

```bash
npm run dev
```

## Проверки

```bash
npm run check
```

Команда последовательно проверяет форматирование, ESLint, TypeScript, frontend- и Rust-тесты, затем запускает Clippy с запретом предупреждений.

Отдельные команды:

```bash
npm run format
npm run lint
npm run typecheck
npm test
npm run tauri build -- --no-bundle
```

## Документация

- [Спецификация продукта и архитектуры](docs/SPEC.md)
- [План реализации](docs/IMPLEMENTATION_PLAN.md)
- [Architecture Decision Records](docs/adr/README.md)
- [Уведомления о сторонних материалах](THIRD_PARTY_NOTICES.md)

## Основной пользовательский цикл

1. Пользователь задаёт интервал и упражнения.
2. Приложение ждёт в системном трее.
3. В назначенное время проигрывает голосовое напоминание, показывает системное уведомление и компактное окно.
4. Пользователь вводит фактическое количество повторений.
5. Результат сохраняется, новый интервал начинается от момента завершения тренировки.
