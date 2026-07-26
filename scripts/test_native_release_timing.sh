#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

export GITHUB_RUN_ID=42
export GITHUB_RUN_ATTEMPT=3
commit="0123456789abcdef0123456789abcdef01234567"
cases=(
  "cli|aarch64-apple-darwin|macos-15|macOS"
  "cli|x86_64-apple-darwin|macos-15-intel|macOS"
  "cli|x86_64-unknown-linux-gnu|ubuntu-24.04|Linux"
  "cli|x86_64-unknown-linux-musl|ubuntu-24.04|Linux"
  "cli|x86_64-pc-windows-msvc|windows-2025|Windows"
  "desktop-gui|aarch64-apple-darwin|macos-15|macOS"
)
index=0
for specification in "${cases[@]}"; do
  IFS='|' read -r surface target environment runner_os <<< "$specification"
  export RUNNER_OS="$runner_os"
  output="$temporary/timing-$index.json"
  "$script_dir/write_native_release_timing.sh" \
    "v1.0.0-alpha.1" \
    "$commit" \
    "$surface" \
    "$target" \
    "$environment" \
    release-alpha \
    100 \
    110 \
    114 \
    120 \
    130 \
    "2026-07-26T12:00:00Z" \
    "$output"
  jq -e \
    --arg commit "$commit" \
    --arg surface "$surface" \
    --arg target "$target" \
    --arg environment "$environment" \
    --arg runner_os "$runner_os" \
    '
      .schema == "canisend.native-release-timing/v1"
      and .source_commit == $commit
      and .surface == $surface
      and .target == $target
      and .environment == $environment
      and .profile == "release-alpha"
      and .runner_os == $runner_os
      and .durations_seconds == {
        release_build: 10,
        extra_validation: 4,
        package: 6,
        archive_smoke: 10,
        measured_total: 30
      }
      and .checks.workspace_suite_repeated_on_target == false
      and .checks.authoritative_release_evidence == false
    ' "$output" >/dev/null
  index="$((index + 1))"
done

export RUNNER_OS=Linux
if "$script_dir/write_native_release_timing.sh" \
  "v1.0.0-alpha.1" \
  "$commit" \
  cli \
  x86_64-unknown-linux-gnu \
  ubuntu-24.04 \
  release-alpha \
  100 \
  99 \
  114 \
  120 \
  130 \
  "2026-07-26T12:00:00Z" \
  "$temporary/invalid.json" \
  >"$temporary/invalid.stdout" \
  2>"$temporary/invalid.stderr"; then
  echo "native release timing test: non-monotonic timing was accepted" >&2
  exit 1
fi
grep -q "timing boundaries are not monotonic" "$temporary/invalid.stderr"

if "$script_dir/write_native_release_timing.sh" \
  "v1.0.0-alpha.1" \
  "$commit" \
  cli \
  x86_64-unknown-linux-gnu \
  ubuntu-24.04 \
  release \
  100 \
  110 \
  114 \
  120 \
  130 \
  "2026-07-26T12:00:00Z" \
  "$temporary/profile-mismatch.json" \
  >"$temporary/profile-mismatch.stdout" \
  2>"$temporary/profile-mismatch.stderr"; then
  echo "native release timing test: mismatched Alpha profile was accepted" >&2
  exit 1
fi
grep -q "profile does not match the validated release stage" \
  "$temporary/profile-mismatch.stderr"

echo "native release timing test: ok"
