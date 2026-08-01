#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 UNIFIED_HOST_BINARY OUTPUT_DIRECTORY" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI package: this script must run on macOS" >&2
  exit 1
fi
if [[ "$(uname -m)" != "arm64" ]]; then
  echo "macOS GUI package: the Alpha desktop archive must be built natively on Apple Silicon" >&2
  exit 1
fi

host_binary="$1"
output="$2"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
host_binary="$(canisend_absolute_path "$host_binary")"
output="$(canisend_absolute_path "$output")"

for command in ditto hdiutil jq unzip; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI package: required command is missing: $command" >&2
    exit 1
  fi
done
if [[ ! -f "$host_binary" || -L "$host_binary" ]]; then
  echo "macOS GUI package: host must be a regular non-symlink file: $host_binary" >&2
  exit 1
fi

version_json="$("$host_binary" version --json)"
version="$(printf '%s' "$version_json" | jq -er '.data.version')"
archive_name="CanISend-$version-aarch64-apple-darwin.zip"
archive="$output/$archive_name"
dmg_name="CanISend-$version-aarch64-apple-darwin.dmg"
dmg="$output/$dmg_name"
mkdir -p "$output"
for destination in "$archive" "$dmg"; do
  if [[ -e "$destination" || -L "$destination" ]]; then
    echo "macOS GUI package: output already exists: $destination" >&2
    exit 1
  fi
done

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-package.XXXXXX")"
cleanup() {
  rm -rf "$fixture_root"
}
trap cleanup EXIT

stage="$fixture_root/stage"
app="$stage/CanISend.app"
manifest="$stage/CanISend.app.manifest.json"
mkdir -p "$stage"
"$script_dir/stage_macos_gui_app.sh" "$host_binary" "$app"
test -f "$manifest"

# Resource forks and extended attributes are intentionally excluded. All executable
# signatures and app integrity data are regular files or Mach-O bytes, and excluding
# AppleDouble entries keeps the frozen archive top level exact and portable.
temporary_archive="$fixture_root/$archive_name"
ditto -c -k --norsrc --noextattr "$stage" "$temporary_archive"
if [[ ! -s "$temporary_archive" || -L "$temporary_archive" ]]; then
  echo "macOS GUI package: archive was not created as a regular non-empty file" >&2
  exit 1
fi

entries="$(unzip -Z1 "$temporary_archive")"
if [[ "$(printf '%s\n' "$entries" | sed -n '/^CanISend\.app\/$/p' | wc -l | tr -d ' ')" != "1" ]] \
  || [[ "$(printf '%s\n' "$entries" | sed -n '/^CanISend\.app\.manifest\.json$/p' | wc -l | tr -d ' ')" != "1" ]] \
  || printf '%s\n' "$entries" | grep -Eq '(^|/)(__MACOSX|\._[^/]*)(/|$)'; then
  echo "macOS GUI package: archive top-level contract is invalid" >&2
  exit 1
fi

dmg_source="$fixture_root/dmg-source"
mkdir -p "$dmg_source"
ditto --norsrc --noextattr "$app" "$dmg_source/CanISend.app"
cp "$manifest" "$dmg_source/CanISend.app.manifest.json"
ln -s /Applications "$dmg_source/Applications"

temporary_dmg="$fixture_root/$dmg_name"
hdiutil create \
  -quiet \
  -format UDZO \
  -fs HFS+ \
  -volname CanISend \
  -srcfolder "$dmg_source" \
  "$temporary_dmg"
if [[ ! -s "$temporary_dmg" || -L "$temporary_dmg" ]]; then
  echo "macOS GUI package: DMG was not created as a regular non-empty file" >&2
  exit 1
fi
hdiutil verify "$temporary_dmg" >/dev/null

mv "$temporary_archive" "$archive"
mv "$temporary_dmg" "$dmg"
echo "macOS GUI package: created $archive"
echo "macOS GUI package: created $dmg"
