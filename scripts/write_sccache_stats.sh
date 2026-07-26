#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 10 ]]; then
  echo "usage: $0 SOURCE_COMMIT OWNER TARGET PROFILE FEATURE_SET NAMESPACE START_EPOCH END_EPOCH COMPLETED_AT OUTPUT" >&2
  exit 2
fi

source_commit="$1"
owner="$2"
target="$3"
profile="$4"
feature_set="$5"
namespace="$6"
start_epoch="$7"
end_epoch="$8"
completed_at="$9"
output="${10}"

: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
: "${GITHUB_RUN_ATTEMPT:?GITHUB_RUN_ATTEMPT is required}"
: "${RUNNER_OS:?RUNNER_OS is required}"
command -v jq >/dev/null

if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "sccache statistics: source commit must be 40 lowercase hexadecimal characters" >&2
  exit 1
fi
for value in "$owner" "$target" "$profile" "$feature_set"; do
  if [[ ! "$value" =~ ^[A-Za-z0-9._+-]+$ ]]; then
    echo "sccache statistics: identity fields must be body-free tokens" >&2
    exit 1
  fi
done
if [[ ! "$namespace" =~ ^canisend-v[0-9]+-rust-[0-9]+\.[0-9]+\.[0-9]+-[A-Za-z0-9._+-]+$ ]]; then
  echo "sccache statistics: cache namespace is invalid: $namespace" >&2
  exit 1
fi
for value in "$GITHUB_RUN_ID" "$GITHUB_RUN_ATTEMPT" "$start_epoch" "$end_epoch"; do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "sccache statistics: IDs and epochs must be positive integers" >&2
    exit 1
  fi
done
if (( start_epoch > end_epoch )); then
  echo "sccache statistics: timing boundaries are not monotonic" >&2
  exit 1
fi
if [[ ! "$completed_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then
  echo "sccache statistics: completion time must be UTC RFC 3339 seconds" >&2
  exit 1
fi
if [[ -e "$output" || -L "$output" ]]; then
  echo "sccache statistics: output must not already exist: $output" >&2
  exit 1
fi

mkdir -p "$(dirname "$output")"
raw="$(mktemp)"
trap 'rm -f "$raw"' EXIT

enabled="${CANISEND_SCCACHE_ENABLED:-false}"
fallback_reason="${CANISEND_SCCACHE_FALLBACK_REASON:-not-configured}"
stats_available=false
if [[ "$enabled" == "true" && -n "${SCCACHE_PATH:-}" ]]; then
  if "$SCCACHE_PATH" --show-stats --stats-format=json >"$raw" 2>/dev/null \
    && jq -e '.stats | type == "object"' "$raw" >/dev/null 2>&1; then
    stats_available=true
  fi
fi

build_seconds="$((end_epoch - start_epoch))"
jq_arguments=(
  --arg schema "canisend.sccache-stats/v1"
  --arg source_commit "$source_commit"
  --arg owner "$owner"
  --arg target "$target"
  --arg profile "$profile"
  --arg feature_set "$feature_set"
  --arg namespace "$namespace"
  --arg runner_os "$RUNNER_OS"
  --arg completed_at "$completed_at"
  --arg fallback_reason "$fallback_reason"
  --argjson github_run_id "$GITHUB_RUN_ID"
  --argjson github_run_attempt "$GITHUB_RUN_ATTEMPT"
  --argjson compile_window "$build_seconds"
  --argjson enabled "$enabled"
  --argjson stats_available "$stats_available"
)

if [[ "$stats_available" == "true" ]]; then
  jq -n "${jq_arguments[@]}" --slurpfile raw "$raw" '
    def counter_total($counter):
      (([$counter.counts[]?, $counter.adv_counts[]?] | add) // 0);
    def duration_ms($duration):
      ((($duration.secs // 0) * 1000) + ((($duration.nanos // 0) / 1000000) | floor));
    $raw[0].stats as $stats
    | counter_total($stats.cache_hits) as $hits
    | counter_total($stats.cache_misses) as $misses
    | counter_total($stats.cache_errors) as $errors
    | ($hits + $misses + $errors) as $attempts
    | {
        schema: $schema,
        source_commit: $source_commit,
        owner: $owner,
        target: $target,
        profile: $profile,
        feature_set: $feature_set,
        runner_os: $runner_os,
        github_run_id: $github_run_id,
        github_run_attempt: $github_run_attempt,
        tool: {
          name: "sccache",
          version: "v0.16.0"
        },
        cache: {
          enabled: $enabled,
          backend: "github-actions-v2",
          namespace: $namespace,
          stats_available: $stats_available,
          fallback_available: true,
          fallback_reason: $fallback_reason,
          compile_requests: ($stats.compile_requests // 0),
          requests_executed: ($stats.requests_executed // 0),
          cache_hits: $hits,
          cache_misses: $misses,
          cache_errors: $errors,
          cache_writes: ($stats.cache_writes // 0),
          cache_write_errors: ($stats.cache_write_errors // 0),
          hit_rate_percent:
            (if $attempts == 0 then 0 else ((($hits * 10000 / $attempts) | floor) / 100) end)
        },
        durations_milliseconds: {
          cache_read_hits: duration_ms($stats.cache_read_hit_duration),
          cache_writes: duration_ms($stats.cache_write_duration),
          compiler_writes: duration_ms($stats.compiler_write_duration)
        },
        measurement: {
          compile_window_seconds: $compile_window,
          time_saved_seconds: null,
          time_saved_method: "cold-warm-candidate-comparison-required"
        },
        checks: {
          body_free: true,
          authoritative_release_evidence: false,
          cache_hit_is_release_evidence: false,
          fallback_preserves_build_command: true,
          no_publication: true
        },
        completed_at: $completed_at
      }
  ' > "$output"
else
  jq -n "${jq_arguments[@]}" '
    {
      schema: $schema,
      source_commit: $source_commit,
      owner: $owner,
      target: $target,
      profile: $profile,
      feature_set: $feature_set,
      runner_os: $runner_os,
      github_run_id: $github_run_id,
      github_run_attempt: $github_run_attempt,
      tool: {
        name: "sccache",
        version: "v0.16.0"
      },
      cache: {
        enabled: $enabled,
        backend: (if $enabled then "github-actions-v2" else null end),
        namespace: $namespace,
        stats_available: false,
        fallback_available: true,
        fallback_reason: $fallback_reason,
        compile_requests: null,
        requests_executed: null,
        cache_hits: null,
        cache_misses: null,
        cache_errors: null,
        cache_writes: null,
        cache_write_errors: null,
        hit_rate_percent: null
      },
      durations_milliseconds: {
        cache_read_hits: null,
        cache_writes: null,
        compiler_writes: null
      },
      measurement: {
        compile_window_seconds: $compile_window,
        time_saved_seconds: null,
        time_saved_method: "cold-warm-candidate-comparison-required"
      },
      checks: {
        body_free: true,
        authoritative_release_evidence: false,
        cache_hit_is_release_evidence: false,
        fallback_preserves_build_command: true,
        no_publication: true
      },
      completed_at: $completed_at
    }
  ' > "$output"
fi

echo "sccache statistics: recorded $owner/$target (available=$stats_available)"
