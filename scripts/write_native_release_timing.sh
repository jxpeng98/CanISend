#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 12 ]]; then
  echo "usage: $0 TAG SOURCE_COMMIT SURFACE TARGET ENVIRONMENT START_EPOCH BUILD_DONE_EPOCH PACKAGE_START_EPOCH PACKAGE_DONE_EPOCH COMPLETED_EPOCH COMPLETED_AT OUTPUT" >&2
  exit 2
fi

tag="$1"
source_commit="$2"
surface="$3"
target="$4"
environment="$5"
start_epoch="$6"
build_done_epoch="$7"
package_start_epoch="$8"
package_done_epoch="$9"
completed_epoch="${10}"
completed_at="${11}"
output="${12}"

: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
: "${GITHUB_RUN_ATTEMPT:?GITHUB_RUN_ATTEMPT is required}"
: "${RUNNER_OS:?RUNNER_OS is required}"
command -v jq >/dev/null

if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "native release timing: tag is not canonical SemVer: $tag" >&2
  exit 1
fi
if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "native release timing: source commit must be 40 lowercase hexadecimal characters" >&2
  exit 1
fi
for value in \
  "$GITHUB_RUN_ID" \
  "$GITHUB_RUN_ATTEMPT" \
  "$start_epoch" \
  "$build_done_epoch" \
  "$package_start_epoch" \
  "$package_done_epoch" \
  "$completed_epoch"
do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "native release timing: IDs and epochs must be positive integers" >&2
    exit 1
  fi
done
if (( start_epoch > build_done_epoch \
  || build_done_epoch > package_start_epoch \
  || package_start_epoch > package_done_epoch \
  || package_done_epoch > completed_epoch )); then
  echo "native release timing: timing boundaries are not monotonic" >&2
  exit 1
fi
if [[ ! "$completed_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then
  echo "native release timing: completion time must be UTC RFC 3339 seconds" >&2
  exit 1
fi

case "$surface:$target:$environment:$RUNNER_OS" in
  cli:aarch64-apple-darwin:macos-15:macOS \
    | cli:x86_64-apple-darwin:macos-15-intel:macOS \
    | cli:x86_64-unknown-linux-gnu:ubuntu-24.04:Linux \
    | cli:x86_64-unknown-linux-musl:ubuntu-24.04:Linux \
    | cli:x86_64-pc-windows-msvc:windows-2025:Windows \
    | desktop-gui:aarch64-apple-darwin:macos-15:macOS)
    ;;
  *)
    echo "native release timing: surface, target, environment, and runner do not match" >&2
    exit 1
    ;;
esac

if [[ -e "$output" || -L "$output" ]]; then
  echo "native release timing: output must not already exist: $output" >&2
  exit 1
fi
mkdir -p "$(dirname "$output")"

version="${tag#v}"
build_seconds="$((build_done_epoch - start_epoch))"
extra_validation_seconds="$((package_start_epoch - build_done_epoch))"
package_seconds="$((package_done_epoch - package_start_epoch))"
archive_smoke_seconds="$((completed_epoch - package_done_epoch))"
measured_total_seconds="$((completed_epoch - start_epoch))"

jq -n \
  --arg tag "$tag" \
  --arg version "$version" \
  --arg source_commit "$source_commit" \
  --arg surface "$surface" \
  --arg target "$target" \
  --arg environment "$environment" \
  --arg runner_os "$RUNNER_OS" \
  --arg completed_at "$completed_at" \
  --argjson github_run_id "$GITHUB_RUN_ID" \
  --argjson github_run_attempt "$GITHUB_RUN_ATTEMPT" \
  --argjson release_build "$build_seconds" \
  --argjson extra_validation "$extra_validation_seconds" \
  --argjson package "$package_seconds" \
  --argjson archive_smoke "$archive_smoke_seconds" \
  --argjson measured_total "$measured_total_seconds" \
  '{
    schema: "canisend.native-release-timing/v1",
    tag: $tag,
    version: $version,
    source_commit: $source_commit,
    surface: $surface,
    target: $target,
    environment: $environment,
    runner_os: $runner_os,
    github_run_id: $github_run_id,
    github_run_attempt: $github_run_attempt,
    durations_seconds: {
      release_build: $release_build,
      extra_validation: $extra_validation,
      package: $package,
      archive_smoke: $archive_smoke,
      measured_total: $measured_total
    },
    checks: {
      source_gate_completed: true,
      workspace_suite_repeated_on_target: false,
      release_build_completed: true,
      package_completed: true,
      archive_smoke_completed: true,
      authoritative_release_evidence: false,
      no_publication: true
    },
    completed_at: $completed_at
  }' > "$output"

echo "native release timing: recorded $surface/$target ($measured_total_seconds seconds)"
