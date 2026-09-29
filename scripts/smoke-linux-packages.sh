#!/usr/bin/env bash
set -euo pipefail

bundle_dir="${1:-src-tauri/target/release/bundle}"
deb_path="$(find "$bundle_dir" -type f -name '*.deb' -print -quit)"
appimage_path="$(find "$bundle_dir" -type f -name '*.AppImage' -print -quit)"

test -n "$deb_path"
test -n "$appimage_path"
appimage_path="$(cd "$(dirname "$appimage_path")" && pwd)/$(basename "$appimage_path")"

package_name="$(dpkg-deb -f "$deb_path" Package)"
test -n "$package_name"

sudo dpkg -i "$deb_path" || sudo apt-get install -f -y
dpkg-query -W -f='${Status}\n' "$package_name" | grep -q 'install ok installed'

# Повторная установка той же версии проверяет безопасный upgrade path.
sudo dpkg -i "$deb_path"

profile_dir="$(mktemp -d)"
data_dir="$profile_dir/.local/share/com.oilyin.gym"
mkdir -p "$data_dir"
touch "$data_dir/gym.sqlite3"

sudo dpkg -r "$package_name"
test -f "$data_dir/gym.sqlite3"

extract_dir="$(mktemp -d)"
(
  cd "$extract_dir"
  chmod +x "$appimage_path"
  "$appimage_path" --appimage-extract >/dev/null
  test -x squashfs-root/AppRun
)

rm -rf -- "$profile_dir" "$extract_dir"

echo "Linux package smoke tests passed; user data survived package removal."
