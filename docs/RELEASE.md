# Выпуск Gym 0.1.0

## Локальные команды

Перед выпуском:

```bash
npm ci
npm run check
npm run release:verify
```

Fedora 44 x86_64:

```bash
npm run release:fedora
scripts/smoke-fedora-packages.sh
npm run release:collect:fedora
```

macOS с установленными Rust-целями `aarch64-apple-darwin` и
`x86_64-apple-darwin`:

```bash
npm run release:macos
npm run release:collect:macos
```

Локальная macOS-сборка по умолчанию получает ad-hoc подпись. Она предназначена для
тестирования, а не публичного распространения.

## GitHub release

Workflow `.github/workflows/release.yml` запускается вручную либо тегом `vX.Y.Z`.
Ручной запуск может создать ad-hoc macOS build. Для тега workflow требует полноценную
Developer ID подпись и notarization.

Необходимые repository secrets:

- `APPLE_CERTIFICATE` — base64 экспортированного Developer ID Application `.p12`;
- `APPLE_CERTIFICATE_PASSWORD`;
- `APPLE_SIGNING_IDENTITY`;
- `KEYCHAIN_PASSWORD`;
- `APPLE_ID`;
- `APPLE_PASSWORD` — app-specific password;
- `APPLE_TEAM_ID`.

Workflow создаёт:

- Fedora 44 x86_64 `.rpm` и `.AppImage`;
- universal macOS `.dmg` с arm64 и x86_64;
- общий `SHA256SUMS.txt`;
- GitHub Release только после успешных Fedora и macOS jobs.

## Автоматические проверки

- совпадение версий в `package.json`, `Cargo.toml` и `tauri.conf.json`;
- соответствие git-тега версии;
- наличие датированной записи в `CHANGELOG.md`;
- полный `npm run check`;
- создание и распаковка AppImage;
- чистая установка, повторная установка и удаление `.rpm`;
- сохранение пользовательской базы после удаления пакета;
- восстановление резервной копии SQLite в Rust-тесте;
- `codesign`, Gatekeeper assessment и stapler validation подписанной macOS-сборки;
- SHA-256 для каждого публикуемого пакета.

Версия 0.1.0 создаёт схему базы с нуля. Предыдущего публичного формата базы ещё нет;
повторное открытие и идемпотентность текущей миграции покрыты Rust-тестами. Начиная с
0.2.0 в репозитории должна храниться fixture-база предыдущего релиза.

## Ручная матрица перед тегом

- [ ] Fedora 44 GNOME Wayland: чистая установка и первый запуск.
- [ ] Fedora 44 GNOME X11: tray, уведомление, popup и автозапуск.
- [ ] KDE Plasma: tray, уведомление и best-effort always-on-top.
- [ ] macOS arm64: установка DMG, Gatekeeper, уведомления и автозапуск.
- [ ] macOS x86_64: установка DMG и основной сценарий.
- [ ] Scaling 100%, 125% и 150% на Linux и macOS.
- [ ] VoiceOver на macOS и Orca на Linux.
- [ ] Обновление поверх предыдущей сборки без потери базы.
- [ ] Восстановление копии `gym.sqlite3` в чистом профиле.

После выполнения матрицы создаётся тег, совпадающий с manifest-версией:

```bash
git tag v0.1.0
git push origin v0.1.0
```
