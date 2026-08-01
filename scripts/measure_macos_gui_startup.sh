#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
  echo "usage: $0 CanISend.app OUTPUT.json PROFILE [--nonpublishing-profile-candidate]" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI startup: this script must run on macOS" >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source "$script_dir/lib/native_paths.sh"
app="$(canisend_absolute_path "$1")"
output="$(canisend_absolute_path "$2")"
profile="$3"
nonpublishing_profile_candidate=false
if [[ $# -eq 4 ]]; then
  if [[ "$4" != "--nonpublishing-profile-candidate" ]]; then
    echo "macOS GUI startup: unknown candidate mode: $4" >&2
    exit 2
  fi
  nonpublishing_profile_candidate=true
fi
manifest="$app.manifest.json"
host="$app/Contents/MacOS/canisend-gui"
budget_ms=2000
host_budget_bytes=67108864
payload_budget_bytes=75497472
trials=5

case "$profile" in
  release-alpha | release) ;;
  *)
    echo "macOS GUI startup: profile must be release-alpha or release" >&2
    exit 1
    ;;
esac

for command in jq open osascript perl shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI startup: required command is missing: $command" >&2
    exit 1
  fi
done
"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest"
version="$("$host" version --json | jq -er '.data.version')"
if [[ "$version" == *-alpha.* ]]; then
  expected_profile="release-alpha"
else
  expected_profile="release"
fi
if [[ "$profile" != "$expected_profile" && "$nonpublishing_profile_candidate" != true ]]; then
  echo "macOS GUI startup: profile does not match the packaged release stage" >&2
  exit 1
fi
if [[ -e "$output" ]]; then
  echo "macOS GUI startup: output must not already exist: $output" >&2
  exit 1
fi

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-gui-startup.XXXXXX")"
gui_pid=""
launcher_pid=""
cleanup() {
  if [[ -n "$gui_pid" ]] && kill -0 "$gui_pid" 2>/dev/null; then
    kill "$gui_pid" 2>/dev/null || true
  fi
  if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" 2>/dev/null; then
    kill "$launcher_pid" 2>/dev/null || true
    wait "$launcher_pid" 2>/dev/null || true
  fi
  rm -rf "$fixture_root"
}
trap cleanup EXIT

samples_file="$fixture_root/samples"
: > "$samples_file"
for trial in $(seq 1 "$trials"); do
  home="$fixture_root/home-$trial"
  runtime_bin="$home/.local/share/mise/shims"
  mkdir -p "$runtime_bin"
  cp "$repo_root/fixtures/runtime/fake-codex-runtime.sh" "$runtime_bin/codex"
  chmod 700 "$runtime_bin/codex"
  started_ms="$(perl -MTime::HiRes=time -e 'printf "%.3f", time() * 1000')"
  open -n -W \
    --env "HOME=$home" \
    --env "PATH=$runtime_bin:/usr/bin:/bin" \
    --stdout "$fixture_root/gui-$trial.log" \
    --stderr "$fixture_root/gui-$trial.log" \
    "$app" &
  launcher_pid="$!"
  if ! gui_pid="$(osascript - <<'APPLESCRIPT'
tell application "System Events"
    repeat 200 times
        set guiProcesses to every application process whose bundle identifier is "io.github.jxpeng98.canisend"
        if (count of guiProcesses) is 1 then return unix id of item 1 of guiProcesses
        delay 0.01
    end repeat
end tell
error "GUI process did not appear uniquely within two seconds" number 1
APPLESCRIPT
)"; then
    echo "macOS GUI startup: trial $trial process did not appear" >&2
    sed -n '1,120p' "$fixture_root/gui-$trial.log" >&2
    exit 1
  fi
  if ! osascript - "$gui_pid" <<'APPLESCRIPT'
on findMainLandmark(appWindow)
    tell application "System Events"
        try
            set nativeRoot to UI element 1 of appWindow
            set contentRoot to UI element 1 of nativeRoot
            set scrollArea to UI element 1 of contentRoot
            set webArea to UI element 1 of scrollArea
            repeat with childElement in UI elements of webArea
                try
                    set candidateElement to contents of childElement
                    set candidateName to name of candidateElement as text
                    if candidateName is "CanISend main content" or candidateName is "CanISend 主要内容" then
                        return candidateElement
                    end if
                end try
            end repeat
        end try
    end tell
    return missing value
end findMainLandmark

on run arguments
    set guiPid to item 1 of arguments as integer
    tell application "System Events"
        repeat 200 times
            if exists (first process whose unix id is guiPid) then
                set guiProcess to first process whose unix id is guiPid
                if (count of windows of guiProcess) > 0 then
                    set appWindow to window 1 of guiProcess
                    set mainContent to my findMainLandmark(appWindow)
                    if mainContent is not missing value then return
                end if
            end if
            delay 0.01
        end repeat
    end tell
    error "GUI did not expose the stable Svelte main landmark within two seconds" number 1
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
  wait "$launcher_pid" 2>/dev/null || true
  gui_pid=""
  launcher_pid=""
done

samples_json="$(jq -Rsc 'split("\n") | map(select(length > 0) | tonumber)' "$samples_file")"
median_ms="$(jq -n --argjson samples "$samples_json" \
  '$samples | sort | .[(length / 2 | floor)]')"
maximum_ms="$(jq -n --argjson samples "$samples_json" '$samples | max')"
minimum_ms="$(jq -n --argjson samples "$samples_json" '$samples | min')"
host_bytes="$(stat -f '%z' "$host")"
payload_bytes="$(du -sk "$app" | awk '{print $1 * 1024}')"
host_sha256="$(shasum -a 256 "$host" | awk '{print $1}')"
machine="$(system_profiler SPHardwareDataType 2>/dev/null | awk -F: \
  '/Model Name|Model Identifier|Chip|Memory/ {gsub(/^[ \t]+/, "", $2); printf "%s%s", separator, $2; separator="; "}')"
macos_version="$(sw_vers -productVersion)"

jq -n \
  --arg version "$version" \
  --arg profile "$profile" \
  --arg measured_at "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" \
  --arg machine "$machine" \
  --arg macos_version "$macos_version" \
  --arg host_sha256 "$host_sha256" \
  --argjson nonpublishing_profile_candidate "$nonpublishing_profile_candidate" \
  --argjson samples_ms "$samples_json" \
  --argjson minimum_ms "$minimum_ms" \
  --argjson median_ms "$median_ms" \
  --argjson maximum_ms "$maximum_ms" \
  --argjson budget_ms "$budget_ms" \
  --argjson host_budget_bytes "$host_budget_bytes" \
  --argjson payload_budget_bytes "$payload_budget_bytes" \
  --argjson host_bytes "$host_bytes" \
  --argjson payload_bytes "$payload_bytes" \
  '({
    schema: "canisend.macos-gui-performance/v2",
    version: $version,
    profile: $profile,
    measured_at: $measured_at,
    reference_machine: $machine,
    macos_version: $macos_version,
    readiness_probe: "native WebView window with the CanISend main content landmark",
    trials: ($samples_ms | length),
    samples_ms: $samples_ms,
    minimum_ms: $minimum_ms,
    median_ms: $median_ms,
    maximum_ms: $maximum_ms,
    budgets: {
      maximum_startup_ms: $budget_ms,
      unified_host_bytes: $host_budget_bytes,
      application_payload_bytes: $payload_budget_bytes
    },
    passed: (
      $maximum_ms <= $budget_ms
      and $host_bytes <= $host_budget_bytes
      and $payload_bytes <= $payload_budget_bytes
    ),
    bytes: {
      unified_host: $host_bytes,
      application_payload: $payload_bytes
    },
    sha256: {
      unified_host: $host_sha256
    }
  } + if $nonpublishing_profile_candidate then {
    qualification_scope: "nonpublishing-profile-candidate",
    authoritative_release_evidence: false
  } else {} end)' > "$output"

if ! jq -e '.passed == true' "$output" >/dev/null; then
  echo "macOS GUI startup: startup or size budget exceeded; inspect $output" >&2
  exit 1
fi
echo "macOS GUI startup: ${median_ms}ms median, ${maximum_ms}ms maximum; ${host_bytes} byte unified host"
