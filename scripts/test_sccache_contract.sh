#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

fake="$temporary/sccache"
cat > "$fake" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --version)
    echo "sccache 0.16.0"
    ;;
  --start-server|--zero-stats|--stop-server)
    exit 0
    ;;
  --show-stats)
    printf '%s\n' '{
      "stats": {
        "compile_requests": 20,
        "requests_executed": 18,
        "cache_errors": {"counts": {}, "adv_counts": {}},
        "cache_hits": {"counts": {"Rust": 12}, "adv_counts": {}},
        "cache_misses": {"counts": {"Rust": 6}, "adv_counts": {}},
        "cache_write_errors": 0,
        "cache_writes": 6,
        "cache_write_duration": {"secs": 1, "nanos": 500000000},
        "cache_read_hit_duration": {"secs": 0, "nanos": 240000000},
        "compiler_write_duration": {"secs": 9, "nanos": 750000000}
      }
    }'
    ;;
  *)
    exit 1
    ;;
esac
EOF
chmod +x "$fake"

namespace="canisend-v1-rust-1.97.0-x86_64-unknown-linux-gnu-release-cli-default"
environment_file="$temporary/github.env"
output_file="$temporary/github.output"
CANISEND_SCCACHE_INSTALL_OUTCOME=success \
SCCACHE_PATH="$fake" \
GITHUB_ENV="$environment_file" \
GITHUB_OUTPUT="$output_file" \
  "$script_dir/configure_sccache.sh" "$namespace"
grep -q '^enabled=true$' "$output_file"
grep -q "^RUSTC_WRAPPER=$fake$" "$environment_file"
grep -q "^SCCACHE_GHA_VERSION=$namespace$" "$environment_file"

set -a
source "$environment_file"
set +a
export SCCACHE_PATH="$fake"
export GITHUB_RUN_ID=42
export GITHUB_RUN_ATTEMPT=3
export RUNNER_OS=Linux
commit="0123456789abcdef0123456789abcdef01234567"
stats="$temporary/stats.json"
"$script_dir/write_sccache_stats.sh" \
  "$commit" \
  native-release \
  x86_64-unknown-linux-gnu \
  release \
  canisend-cli-default \
  "$namespace" \
  100 \
  115 \
  "2026-07-26T12:00:00Z" \
  "$stats"
jq -e \
  --arg commit "$commit" \
  --arg namespace "$namespace" \
  '
    .schema == "canisend.sccache-stats/v1"
    and .source_commit == $commit
    and .tool == {name: "sccache", version: "v0.16.0"}
    and .cache.enabled == true
    and .cache.namespace == $namespace
    and .cache.compile_requests == 20
    and .cache.cache_hits == 12
    and .cache.cache_misses == 6
    and .cache.cache_errors == 0
    and .cache.hit_rate_percent == 66.66
    and .durations_milliseconds.cache_read_hits == 240
    and .durations_milliseconds.compiler_writes == 9750
    and .measurement.compile_window_seconds == 15
    and .measurement.time_saved_seconds == null
    and .checks.authoritative_release_evidence == false
    and .checks.cache_hit_is_release_evidence == false
  ' "$stats" >/dev/null

failed="$temporary/failed-sccache"
cat > "$failed" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
chmod +x "$failed"
fallback_environment="$temporary/fallback.env"
fallback_output="$temporary/fallback.output"
CANISEND_SCCACHE_INSTALL_OUTCOME=success \
SCCACHE_PATH="$failed" \
GITHUB_ENV="$fallback_environment" \
GITHUB_OUTPUT="$fallback_output" \
  "$script_dir/configure_sccache.sh" "$namespace"
grep -q '^enabled=false$' "$fallback_output"
grep -q '^CANISEND_SCCACHE_ENABLED=false$' "$fallback_environment"
if grep -q '^RUSTC_WRAPPER=' "$fallback_environment"; then
  echo "sccache contract test: fallback configured a compiler wrapper" >&2
  exit 1
fi

fallback_stats="$temporary/fallback-stats.json"
CANISEND_SCCACHE_ENABLED=false \
CANISEND_SCCACHE_FALLBACK_REASON=installation-or-server-unavailable \
SCCACHE_PATH="$failed" \
  "$script_dir/write_sccache_stats.sh" \
    "$commit" \
    native-release \
    x86_64-unknown-linux-gnu \
    release \
    canisend-cli-default \
    "$namespace" \
    100 \
    115 \
    "2026-07-26T12:00:00Z" \
    "$fallback_stats"
jq -e '
  .cache.enabled == false
  and .cache.backend == null
  and .cache.stats_available == false
  and .cache.compile_requests == null
  and .cache.fallback_available == true
  and .checks.fallback_preserves_build_command == true
' "$fallback_stats" >/dev/null

if CANISEND_SCCACHE_INSTALL_OUTCOME=success \
  SCCACHE_PATH="$fake" \
  GITHUB_ENV="$temporary/invalid.env" \
  GITHUB_OUTPUT="$temporary/invalid.output" \
  "$script_dir/configure_sccache.sh" "unsafe namespace" \
  >"$temporary/invalid.stdout" \
  2>"$temporary/invalid.stderr"; then
  echo "sccache contract test: invalid namespace was accepted" >&2
  exit 1
fi
grep -q "cache namespace is invalid" "$temporary/invalid.stderr"

echo "sccache contract test: ok"
