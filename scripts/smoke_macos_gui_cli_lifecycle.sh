#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI CLI lifecycle: this script must run on macOS" >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
cd "$repo_root"

cargo build -p canisend-cli --release --locked

source_cli="$repo_root/target/release/canisend"
if [[ ! -f "$source_cli" || -L "$source_cli" ]]; then
  echo "macOS GUI CLI lifecycle: exact release CLI is missing or unsafe" >&2
  exit 1
fi

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-cli-lifecycle.XXXXXX")"
trap 'rm -rf "$fixture_root"' EXIT
first="$fixture_root/first/canisend"
second="$fixture_root/second/canisend"
mkdir -p "$(dirname "$first")" "$(dirname "$second")"
cp "$source_cli" "$first"
cp "$source_cli" "$second"
codesign \
  --force \
  --identifier io.github.jxpeng98.canisend.cli.lifecycle.first \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$first"
codesign \
  --force \
  --identifier io.github.jxpeng98.canisend.cli.lifecycle.second \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$second"
codesign --verify --strict "$first"
codesign --verify --strict "$second"

CANISEND_TEST_CLI_FIRST="$first" \
CANISEND_TEST_CLI_SECOND="$second" \
  cargo test \
    -p canisend-app \
    --locked \
    --test macos_gui_cli_lifecycle \
    -- \
    --ignored \
    --exact packaged_cli_migrates_updates_rolls_back_and_retains_workspace \
    --nocapture

echo "macOS GUI CLI lifecycle: migration, update, rollback, uninstall, and workspace retention passed"
