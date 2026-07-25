#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 CanISend.app OUTPUT.json" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI startup: this script must run on macOS" >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
app="$(canisend_absolute_path "$1")"
output="$(canisend_absolute_path "$2")"
manifest="$app.manifest.json"
gui="$app/Contents/MacOS/canisend-gui"
cli="$app/Contents/Resources/bin/canisend"
budget_ms=2000
gui_budget_bytes=67108864
bundle_budget_bytes=134217728
trials=5

for command in jq osascript perl shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI startup: required command is missing: $command" >&2
    exit 1
  fi
done
"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest"
if [[ -e "$output" ]]; then
  echo "macOS GUI startup: output must not already exist: $output" >&2
  exit 1
fi

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-startup.XXXXXX")"
gui_pid=""
cleanup() {
  if [[ -n "$gui_pid" ]] && kill -0 "$gui_pid" 2>/dev/null; then
    kill "$gui_pid" 2>/dev/null || true
    wait "$gui_pid" 2>/dev/null || true
  fi
  rm -rf "$fixture_root"
}
trap cleanup EXIT

samples_file="$fixture_root/samples"
: > "$samples_file"
for trial in $(seq 1 "$trials"); do
  home="$fixture_root/home-$trial"
  mkdir -p "$home"
  started_ms="$(perl -MTime::HiRes=time -e 'printf "%.3f", time() * 1000')"
  HOME="$home" "$gui" >"$fixture_root/gui-$trial.log" 2>&1 &
  gui_pid="$!"
  if ! osascript - "$gui_pid" <<'APPLESCRIPT'
on run arguments
    set guiPid to item 1 of arguments as integer
    tell application "System Events"
        repeat 200 times
            if exists (first process whose unix id is guiPid) then
                set guiProcess to first process whose unix id is guiPid
                if (count of windows of guiProcess) > 0 then
                    set appWindow to window 1 of guiProcess
                    tell guiProcess
                        if exists button "Overview" of group 1 of appWindow then return
                    end tell
                end if
            end if
            delay 0.01
        end repeat
    end tell
    error "GUI did not expose Overview content within two seconds" number 1
end run
APPLESCRIPT
  then
    echo "macOS GUI startup: trial $trial did not become ready" >&2
    sed -n '1,120p' "$fixture_root/gui-$trial.log" >&2
    exit 1
  fi
  finished_ms="$(perl -MTime::HiRes=time -e 'printf "%.3f", time() * 1000')"
  elapsed_ms="$(awk -v started="$started_ms" -v finished="$finished_ms" \
    'BEGIN { printf "%.3f", finished - started }')"
  printf '%s\n' "$elapsed_ms" >> "$samples_file"
  kill "$gui_pid"
  wait "$gui_pid" 2>/dev/null || true
  gui_pid=""
done

samples_json="$(jq -Rsc 'split("\n") | map(select(length > 0) | tonumber)' "$samples_file")"
median_ms="$(jq -n --argjson samples "$samples_json" \
  '$samples | sort | .[(length / 2 | floor)]')"
maximum_ms="$(jq -n --argjson samples "$samples_json" '$samples | max')"
minimum_ms="$(jq -n --argjson samples "$samples_json" '$samples | min')"
gui_bytes="$(stat -f '%z' "$gui")"
cli_bytes="$(stat -f '%z' "$cli")"
bundle_bytes="$(du -sk "$app" | awk '{print $1 * 1024}')"
gui_sha256="$(shasum -a 256 "$gui" | awk '{print $1}')"
cli_sha256="$(shasum -a 256 "$cli" | awk '{print $1}')"
machine="$(system_profiler SPHardwareDataType 2>/dev/null | awk -F: \
  '/Model Name|Model Identifier|Chip|Memory/ {gsub(/^[ \t]+/, "", $2); printf "%s%s", separator, $2; separator="; "}')"
macos_version="$(sw_vers -productVersion)"

jq -n \
  --arg version "$("$cli" version --json | jq -er '.data.version')" \
  --arg measured_at "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" \
  --arg machine "$machine" \
  --arg macos_version "$macos_version" \
  --arg gui_sha256 "$gui_sha256" \
  --arg cli_sha256 "$cli_sha256" \
  --argjson samples_ms "$samples_json" \
  --argjson minimum_ms "$minimum_ms" \
  --argjson median_ms "$median_ms" \
  --argjson maximum_ms "$maximum_ms" \
  --argjson budget_ms "$budget_ms" \
  --argjson gui_budget_bytes "$gui_budget_bytes" \
  --argjson bundle_budget_bytes "$bundle_budget_bytes" \
  --argjson gui_bytes "$gui_bytes" \
  --argjson cli_bytes "$cli_bytes" \
  --argjson bundle_bytes "$bundle_bytes" \
  '{
    schema: "canisend.macos-gui-performance/v1",
    version: $version,
    measured_at: $measured_at,
    reference_machine: $machine,
    macos_version: $macos_version,
    readiness_probe: "native window with AccessKit Overview navigation control",
    trials: ($samples_ms | length),
    samples_ms: $samples_ms,
    minimum_ms: $minimum_ms,
    median_ms: $median_ms,
    maximum_ms: $maximum_ms,
    budgets: {
      maximum_startup_ms: $budget_ms,
      gui_executable_bytes: $gui_budget_bytes,
      app_bundle_apparent_bytes: $bundle_budget_bytes
    },
    passed: (
      $maximum_ms <= $budget_ms
      and $gui_bytes <= $gui_budget_bytes
      and $bundle_bytes <= $bundle_budget_bytes
    ),
    bytes: {
      gui_executable: $gui_bytes,
      bundled_cli_executable: $cli_bytes,
      app_bundle_apparent: $bundle_bytes
    },
    sha256: {
      gui_executable: $gui_sha256,
      bundled_cli_executable: $cli_sha256
    }
  }' > "$output"

if ! jq -e '.passed == true' "$output" >/dev/null; then
  echo "macOS GUI startup: startup or size budget exceeded; inspect $output" >&2
  exit 1
fi
echo "macOS GUI startup: ${median_ms}ms median, ${maximum_ms}ms maximum; ${gui_bytes} byte GUI"
