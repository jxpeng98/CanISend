#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 BINARY TARGET_TRIPLE OUTPUT_DIRECTORY" >&2
  exit 2
fi

binary="$1"
target="$2"
output="$3"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
output="$(canisend_absolute_path "$output")"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin|x86_64-unknown-linux-gnu|x86_64-unknown-linux-musl)
    archive_extension="tar.gz"
    ;;
  x86_64-pc-windows-msvc)
    archive_extension="zip"
    ;;
  *)
    echo "native package: unsupported target: $target" >&2
    exit 2
    ;;
esac

if [[ ! -f "$binary" || -L "$binary" ]]; then
  echo "native package: binary must be a regular non-symlink file: $binary" >&2
  exit 1
fi

version_json="$("$binary" version --json)"
version="$(printf '%s' "$version_json" | sed -E 's/^.*"version":"([^"]+)".*$/\1/')"
if [[ -z "$version" || "$version" == "$version_json" ]]; then
  echo "native package: could not read product version from binary" >&2
  exit 1
fi

mkdir -p "$output"
bundle_name="canisend-$version-$target"
bundle="$output/$bundle_name"
archive_name="$bundle_name.$archive_extension"
archive="$output/$archive_name"
if [[ -e "$bundle" || -e "$archive" ]]; then
  echo "native package: output already exists for $target" >&2
  exit 1
fi

repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
cd "$repo_root"
"$script_dir/stage_native_bundle.sh" "$binary" "$bundle" "$target"

if [[ "$archive_extension" == "zip" ]]; then
  if ! command -v 7z >/dev/null 2>&1; then
    echo "native package: 7z is required to create the Windows archive" >&2
    exit 1
  fi
  (
    cd "$output"
    7z a -tzip "$archive_name" "$bundle_name" >/dev/null
  )
else
  (
    cd "$output"
    tar -czf "$archive_name" "$bundle_name"
  )
fi

if [[ ! -s "$archive" || -L "$archive" ]]; then
  echo "native package: archive was not created as a regular non-empty file" >&2
  exit 1
fi

echo "native package: created $archive"
