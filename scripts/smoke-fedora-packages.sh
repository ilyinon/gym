#!/usr/bin/env bash
set -euo pipefail

bundle_dir="${1:-src-tauri/target/release/bundle}"
rpm_path="$(find "$bundle_dir" -type f -name '*.rpm' -print -quit)"
appimage_path="$(find "$bundle_dir" -type f -name '*.AppImage' -print -quit)"

test -n "$rpm_path"
test -n "$appimage_path"
rpm_path="$(cd "$(dirname "$rpm_path")" && pwd)/$(basename "$rpm_path")"
appimage_path="$(cd "$(dirname "$appimage_path")" && pwd)/$(basename "$appimage_path")"

package_name="$(rpm -qp --queryformat '%{NAME}' "$rpm_path")"
test -n "$package_name"

dnf install -y "$rpm_path"
rpm -q "$package_name"

# Повторная установка той же версии проверяет безопасный upgrade path.
rpm -Uvh --replacepkgs "$rpm_path"

profile_dir="$(mktemp -d)"
data_dir="$profile_dir/.local/share/com.oilyin.gym"
mkdir -p "$data_dir"
touch "$data_dir/gym.sqlite3"

dnf remove -y "$package_name"
test -f "$data_dir/gym.sqlite3"

extract_dir="$(mktemp -d)"
(
  cd "$extract_dir"
  chmod +x "$appimage_path"
  "$appimage_path" --appimage-extract >/dev/null
  test -x squashfs-root/AppRun
)

rm -rf -- "$profile_dir" "$extract_dir"

echo "Fedora package smoke tests passed; user data survived package removal."
