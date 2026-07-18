#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 BINARY NEW_DESTINATION [TARGET_TRIPLE]" >&2
  exit 2
fi

binary="$1"
destination="$2"
target="${3:-development}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
destination="$(canisend_absolute_path "$destination")"

if [[ ! -f "$binary" || -L "$binary" ]]; then
  echo "native bundle: binary must be a regular non-symlink file: $binary" >&2
  exit 1
fi
if [[ -e "$destination" ]]; then
  echo "native bundle: destination must not exist: $destination" >&2
  exit 1
fi

registry_root="${CARGO_HOME:-$HOME/.cargo}/registry/src"
typst_notice="$(find "$registry_root" -path '*/typst-assets-0.15.1/NOTICE' -print -quit)"
if [[ -z "$typst_notice" ]]; then
  echo "native bundle: typst-assets 0.15.1 NOTICE was not found under $registry_root" >&2
  exit 1
fi
typst_license="$(dirname "$typst_notice")/LICENSE"
if [[ ! -f "$typst_license" ]]; then
  echo "native bundle: typst-assets 0.15.1 LICENSE is missing" >&2
  exit 1
fi

mkdir -p "$destination"
cp "$binary" "$destination/$(basename "$binary")"
cp LICENSE "$destination/LICENSE"
cp THIRD_PARTY_NOTICES.md "$destination/THIRD_PARTY_NOTICES.md"
cp "$typst_license" "$destination/TYPST-ASSETS-LICENSE"
cp "$typst_notice" "$destination/TYPST-ASSETS-NOTICE"
cp release/KNOWN_LIMITATIONS.md "$destination/KNOWN_LIMITATIONS.md"
cp release/ISSUE_COLLECTION.md "$destination/FEEDBACK.md"
cp docs/guides/installation.md "$destination/INSTALL.md"
cp docs/guides/privacy-and-consent.md "$destination/PRIVACY.md"
cp SECURITY.md "$destination/SECURITY.md"
printf '%s\n' "$target" > "$destination/TARGET"
"$destination/$(basename "$binary")" version --json > "$destination/RELEASE.json"

if ! grep -q '"operation":"product.version"' "$destination/RELEASE.json" \
  || ! grep -q '"version":"' "$destination/RELEASE.json"; then
  echo "native bundle: staged binary did not report a product version" >&2
  exit 1
fi
if find "$destination" -type l -print -quit | grep -q .; then
  echo "native bundle: symlinks are not allowed" >&2
  exit 1
fi

echo "native bundle: staged $destination"
