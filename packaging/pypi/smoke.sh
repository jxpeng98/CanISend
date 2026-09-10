#!/usr/bin/env bash
# Exercise the installed wheel, including its native payload, source and resources.
set -euo pipefail
repo="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$repo"
wheel_dir="${1:?usage: smoke.sh WHEEL_DIRECTORY NEW_VENV}"
venv="${2:?usage: smoke.sh WHEEL_DIRECTORY NEW_VENV}"
test ! -e "$venv"
python3 -m venv "$venv"
"$venv/bin/pip" install --no-index --no-deps "$wheel_dir"/*.whl
binary="$venv/bin/canisend"
codesign --verify --strict "$binary"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
"$binary" version --json | jq -e --arg version "$version" --arg revision "$(git rev-parse --short=12 HEAD)" \
  '.ok and .data.version == $version and .data.git_revision == $revision and .data.target == "aarch64-apple-darwin"'
"$binary" doctor --json | jq -e .ok
"$binary" --workspace "$venv/workspace" workspace init --host codex --json | jq -e .ok
"$binary" --workspace "$venv/workspace" workspace check --json | jq -e .ok
for file in LICENSE THIRD_PARTY_NOTICES.md TYPST-ASSETS-LICENSE TYPST-ASSETS-NOTICE SOURCE.tar.gz; do
  test -s "$venv/share/canisend/$file"
done
tar -tzf "$venv/share/canisend/SOURCE.tar.gz" | rg '^crates/canisend-resources/resources/templates/' > /dev/null
