#!/usr/bin/env bash
# Local development/package checks only: never publish or manufacture CI evidence.
set -euo pipefail
mode="${1:-cli}"
case "$mode" in
  cli|desktop|package|npm|pypi) ;;
  *) echo "usage: bash scripts/check_macos_local.sh [cli|desktop|package|npm|pypi]" >&2; exit 2 ;;
esac
if [[ $# -gt 1 || "${GITHUB_ACTIONS:-false}" == true || "$(uname -s)" != Darwin ]]; then
  echo "Run locally on macOS, outside GitHub Actions, with at most one mode." >&2
  exit 2
fi
case "$(uname -m)" in
  arm64) target=aarch64-apple-darwin; npm_platform=darwin-arm64 ;;
  x86_64) target=x86_64-apple-darwin; npm_platform=darwin-x64 ;;
  *) echo "Unsupported native macOS architecture" >&2; exit 2 ;;
esac
repo="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo"
if [[ -n "${CARGO_BUILD_TARGET:-}" ]]; then
  echo "Unset CARGO_BUILD_TARGET: these checks use the native host toolchain." >&2
  exit 2
fi
case "$mode" in
  package|npm|pypi)
    if [[ -n "$(git status --porcelain)" ]]; then
      echo "Commit the intended source first: packaged SOURCE.tar.gz must match the build." >&2
      exit 1
    fi
    ;;
esac
build_root="$(cargo metadata --locked --no-deps --format-version 1 | jq -er .target_directory)"
mkdir -p "$repo/dist"
checks="$(mktemp -d "$repo/dist/local-macos-$mode.XXXXXX")"
echo "Local results: $checks (not release qualification)"
# Retain a log and source identity even when a command fails; no private inputs are used.
{
git rev-parse HEAD > "$checks/source-revision.txt"
git diff --binary HEAD > "$checks/source.patch"
sw_vers > "$checks/platform.txt"
uname -m >> "$checks/platform.txt"

case "$mode" in
  cli)
    cargo fmt --all -- --check
    cargo clippy --workspace --exclude canisend-gui --all-targets --all-features --locked -- -D warnings
    cargo run -p xtask --locked -- source check
    cargo test --workspace --exclude canisend-gui --locked
    cargo build --locked -p canisend
    "$build_root/debug/canisend" version --json | jq -e --arg target "$target" '.ok and .data.target == $target'
    "$build_root/debug/canisend" doctor --json
    bash scripts/smoke_host_v4.sh "$build_root/debug/canisend" "$checks/host"
    bash scripts/smoke_agent_v4_mcp.sh "$build_root/debug/canisend" "$checks/mcp"
    ;;
  desktop)
    pnpm --dir apps/canisend-desktop install --frozen-lockfile
    pnpm --dir apps/canisend-desktop format:check
    pnpm --dir apps/canisend-desktop check
    pnpm --dir apps/canisend-desktop test
    node --test tools/native-preview/tests/qualification-evidence.test.mjs
    pnpm --dir apps/canisend-desktop build
    if cargo tree --locked -p canisend-gui -e normal | grep -F 'tauri-plugin-wdio-webdriver'; then
      echo "Test-only WebDriver plugin leaked into the production graph" >&2; exit 1
    fi
    cargo clippy -p canisend-gui --all-targets --all-features --locked -- -D warnings
    cargo test -p canisend-gui --locked
    cargo build --locked -p canisend-gui --features canisend-gui/custom-protocol
    test -x "$build_root/debug/canisend-gui"
    file "$build_root/debug/canisend-gui" | grep "$(uname -m)"
    ;;
  package|npm)
    cargo build --release --locked -p canisend
    binary="$build_root/release/canisend"
    "$binary" version --json | jq -e --arg revision "$(git rev-parse --short=12 HEAD)" \
      '.ok and .data.git_revision == $revision'
    codesign --force --sign - "$binary"
    codesign --verify --strict "$binary"
    bash scripts/package_native_release.sh "$binary" "$target" "$checks/native"
    version="$("$binary" version --json | jq -er .data.version)"
    bundle="$checks/native/canisend-$version-$target"
    bash scripts/smoke_release_archive.sh "$bundle.tar.gz" "$target" "$checks/archive-smoke" "$binary"
    shasum -a 256 "$bundle.tar.gz" > "$checks/SHA256SUMS"
    if [[ "$mode" == npm ]]; then
      node --test packaging/npm/launcher.test.cjs packaging/npm/pack.test.mjs
      git archive --format=tar.gz HEAD > "$bundle/SOURCE.tar.gz"
      node packaging/npm/pack.mjs "$checks/npm" "$bundle"
      npm install --offline --ignore-scripts --no-audit --no-fund --prefix "$checks/install" "$checks/npm/canisend-$version.tgz"
      installed="$checks/install/node_modules/canisend"
      cmp "$binary" "$installed/native/$npm_platform/canisend"
      cmp "$bundle/SOURCE.tar.gz" "$installed/SOURCE.tar.gz"
      CANISEND_TEST_CLI_BINARY="$installed/canisend.cjs" cargo test -p canisend --locked --test mcp_protocol
      CANISEND_TEST_CLI_BINARY="$installed/canisend.cjs" cargo test -p canisend --locked --test binary_contract workspace_upgrade
      shasum -a 256 "$checks/npm/canisend-$version.tgz" >> "$checks/SHA256SUMS"
    fi
    ;;
  pypi)
    test "$target" = aarch64-apple-darwin # Current wheel smoke supports ARM64 only.
    bash packaging/pypi/build.sh "$checks/wheels"
    twine check --strict "$checks/wheels"/*.whl
    bash packaging/pypi/smoke.sh "$checks/wheels" "$checks/venv"
    CANISEND_TEST_CLI_BINARY="$checks/venv/bin/canisend" cargo test -p canisend --locked --test mcp_protocol
    CANISEND_TEST_CLI_BINARY="$checks/venv/bin/canisend" cargo test -p canisend --locked --test binary_contract workspace_upgrade
    shasum -a 256 "$checks/wheels"/*.whl > "$checks/SHA256SUMS"
    ;;
esac
echo "PASS: local macOS $mode checks; not formal release qualification."
} 2>&1 | tee "$checks/check.log"
