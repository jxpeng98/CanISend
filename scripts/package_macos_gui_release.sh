#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 GUI_BINARY CLI_BINARY OUTPUT_DIRECTORY" >&2
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

gui_binary="$1"
cli_binary="$2"
output="$3"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
gui_binary="$(canisend_absolute_path "$gui_binary")"
cli_binary="$(canisend_absolute_path "$cli_binary")"
output="$(canisend_absolute_path "$output")"

for command in ditto jq unzip; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI package: required command is missing: $command" >&2
    exit 1
  fi
done
for binary in "$gui_binary" "$cli_binary"; do
  if [[ ! -f "$binary" || -L "$binary" ]]; then
    echo "macOS GUI package: binary must be a regular non-symlink file: $binary" >&2
    exit 1
  fi
done

version_json="$("$cli_binary" version --json)"
version="$(printf '%s' "$version_json" | jq -er '.data.version')"
archive_name="CanISend-$version-aarch64-apple-darwin.zip"
archive="$output/$archive_name"
mkdir -p "$output"
if [[ -e "$archive" || -L "$archive" ]]; then
  echo "macOS GUI package: output archive already exists: $archive" >&2
  exit 1
fi

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-package.XXXXXX")"
cleanup() {
  rm -rf "$fixture_root"
}
trap cleanup EXIT

stage="$fixture_root/stage"
app="$stage/CanISend.app"
manifest="$stage/CanISend.app.manifest.json"
mkdir -p "$stage"
"$script_dir/stage_macos_gui_app.sh" "$gui_binary" "$cli_binary" "$app"
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

mv "$temporary_archive" "$archive"
echo "macOS GUI package: created $archive"
