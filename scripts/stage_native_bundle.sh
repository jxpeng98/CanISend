#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 BINARY NEW_DESTINATION" >&2
  exit 2
fi

binary="$1"
destination="$2"

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

echo "native bundle: staged $destination"
