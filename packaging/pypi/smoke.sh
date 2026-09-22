#!/usr/bin/env bash
# Exercise the installed wheel, including its native payload, source and resources.
set -euo pipefail
repo="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$repo"
wheel="${1:?usage: smoke.sh WHEEL NEW_VENV TARGET_TRIPLE}"
venv="${2:?usage: smoke.sh WHEEL NEW_VENV TARGET_TRIPLE}"
target="${3:?usage: smoke.sh WHEEL NEW_VENV TARGET_TRIPLE}"
test ! -e "$venv"
python3 -m venv "$venv"
"$venv/bin/pip" install --no-index --no-deps "$wheel"
binary="$venv/bin/canisend"
if [[ "$target" == *-apple-darwin ]]; then
  codesign --verify --strict "$binary"
fi
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
"$binary" version --json | jq -e --arg version "$version" --arg revision "$(git rev-parse --short=12 HEAD)" --arg target "$target" \
  '.ok and .data.version == $version and .data.git_revision == $revision and .data.target == $target'
"$binary" doctor --json | jq -e .ok
"$binary" --workspace "$venv/workspace" workspace init --host codex --json | jq -e .ok
"$binary" --workspace "$venv/workspace" workspace check --json | jq -e .ok
for file in LICENSE THIRD_PARTY_NOTICES.md TYPST-ASSETS-LICENSE TYPST-ASSETS-NOTICE SOURCE.tar.gz; do
  test -s "$venv/share/canisend/$file"
done
tar -tzf "$venv/share/canisend/SOURCE.tar.gz" | grep '^crates/canisend-resources/resources/templates/' > /dev/null
