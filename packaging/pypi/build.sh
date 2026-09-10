#!/usr/bin/env bash
# Build a native wheel with the same notices and source shipped by the npm channel.
set -euo pipefail
repo="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$repo"
output="${1:?usage: build.sh NEW_OUTPUT_DIRECTORY}"
mkdir -p "$(dirname "$output")"
mkdir "$output"
output="$(CDPATH= cd -- "$output" && pwd)"
data="$repo/packaging/pypi/canisend.data"
mkdir "$data" # Refuse to overwrite an existing staging directory.
trap 'rm -rf "$data"' EXIT
notices="$data/data/share/canisend"
mkdir -p "$notices"
cargo fetch --locked
typst_notice="$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -path '*/typst-assets-0.15.1/NOTICE' -print -quit)"
test -n "$typst_notice"
cp LICENSE THIRD_PARTY_NOTICES.md "$notices/"
cp "$(dirname "$typst_notice")/LICENSE" "$notices/TYPST-ASSETS-LICENSE"
cp "$typst_notice" "$notices/TYPST-ASSETS-NOTICE"
git archive --format=tar.gz HEAD > "$notices/SOURCE.tar.gz"
cd packaging/pypi
maturin build --release --locked --out "$output"
