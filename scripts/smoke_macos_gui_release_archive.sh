#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 && $# -ne 5 ]]; then
  echo "usage: $0 ARCHIVE NEW_SMOKE_DIRECTORY [TAG ENVIRONMENT EVIDENCE]" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI archive smoke: this script must run on macOS" >&2
  exit 1
fi

archive="$1"
smoke_root="$2"
qualification=false
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
archive="$(canisend_absolute_path "$archive")"
smoke_root="$(canisend_absolute_path "$smoke_root")"
if [[ $# -eq 5 ]]; then
  qualification=true
  tag="$3"
  environment="$4"
  evidence="$(canisend_absolute_path "$5")"
  : "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required for qualification evidence}"
  if [[ ! "$GITHUB_RUN_ID" =~ ^[1-9][0-9]*$ ]]; then
    echo "macOS GUI archive smoke: GITHUB_RUN_ID must be a positive integer" >&2
    exit 1
  fi
  if [[ "$environment:$(uname -m)" != "macos-15:arm64" ]]; then
    echo "macOS GUI archive smoke: qualification requires macos-15 on arm64" >&2
    exit 1
  fi
  if [[ -e "$evidence" || -L "$evidence" ]]; then
    echo "macOS GUI archive smoke: evidence destination must not exist: $evidence" >&2
    exit 1
  fi
fi

for command in ditto jq shasum unzip zipinfo; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI archive smoke: required command is missing: $command" >&2
    exit 1
  fi
done
if [[ ! -f "$archive" || -L "$archive" || "$archive" != *.zip ]]; then
  echo "macOS GUI archive smoke: archive must be a regular non-symlink ZIP file" >&2
  exit 1
fi
if [[ -e "$smoke_root" ]]; then
  echo "macOS GUI archive smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi

entry_count="$(unzip -Z1 "$archive" | wc -l | tr -d ' ')"
if (( entry_count < 3 || entry_count > 4096 )); then
  echo "macOS GUI archive smoke: archive entry count is outside the 3..4096 bound" >&2
  exit 1
fi
if zipinfo -l "$archive" | awk '/^l/ { found = 1 } END { exit !found }'; then
  echo "macOS GUI archive smoke: archive contains a symbolic link entry" >&2
  exit 1
fi
uncompressed_bytes="$(
  zipinfo -l "$archive" \
    | awk '/^[d-]/ { total += $4 } END { printf "%.0f", total }'
)"
if (( uncompressed_bytes <= 0 || uncompressed_bytes > 268435456 )); then
  echo "macOS GUI archive smoke: uncompressed size exceeds the 256 MiB bound" >&2
  exit 1
fi

while IFS= read -r entry; do
  if [[ -z "$entry" || ${#entry} -gt 4096 || "$entry" == /* || "$entry" == *\\* ]]; then
    echo "macOS GUI archive smoke: archive contains an unsafe entry name" >&2
    exit 1
  fi
  trimmed="${entry%/}"
  if [[ "$trimmed" == *"//"* ]]; then
    echo "macOS GUI archive smoke: archive entry contains an empty path component: $entry" >&2
    exit 1
  fi
  IFS='/' read -r -a components <<< "$trimmed"
  for component in "${components[@]}"; do
    if [[ "$component" == "." || "$component" == ".." ]]; then
      echo "macOS GUI archive smoke: archive entry escapes its root: $entry" >&2
      exit 1
    fi
    if [[ "$component" == "__MACOSX" || "$component" == ._* ]]; then
      echo "macOS GUI archive smoke: AppleDouble metadata is not allowed: $entry" >&2
      exit 1
    fi
  done
  case "$entry" in
    CanISend.app | CanISend.app/ | CanISend.app/* | CanISend.app.manifest.json) ;;
    *)
      echo "macOS GUI archive smoke: unexpected top-level archive entry: $entry" >&2
      exit 1
      ;;
  esac
done < <(unzip -Z1 "$archive")

mkdir -p "$smoke_root/extracted"
ditto -x -k "$archive" "$smoke_root/extracted"
app="$smoke_root/extracted/CanISend.app"
manifest="$smoke_root/extracted/CanISend.app.manifest.json"
if [[ ! -d "$app" || -L "$app" || ! -f "$manifest" || -L "$manifest" ]]; then
  echo "macOS GUI archive smoke: frozen app and companion manifest are missing" >&2
  exit 1
fi
top_level="$(
  find "$smoke_root/extracted" -mindepth 1 -maxdepth 1 -print \
    | sed "s#^$smoke_root/extracted/##" \
    | sort
)"
if [[ "$top_level" != $'CanISend.app\nCanISend.app.manifest.json' ]]; then
  echo "macOS GUI archive smoke: extracted top level differs from the frozen contract" >&2
  exit 1
fi
if find "$smoke_root/extracted" -type l -print -quit | grep -q .; then
  echo "macOS GUI archive smoke: extracted archive contains a symbolic link" >&2
  exit 1
fi

"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest"
gui="$app/Contents/MacOS/canisend-gui"
cli="$app/Contents/Resources/bin/canisend"
version="$("$cli" version --json | jq -er 'select(.ok == true) | .data.version')"
expected_name="CanISend-$version-aarch64-apple-darwin.zip"
if [[ "$(basename "$archive")" != "$expected_name" ]]; then
  echo "macOS GUI archive smoke: archive must be named $expected_name" >&2
  exit 1
fi
if [[ "$qualification" == true && "$tag" != "v$version" ]]; then
  echo "macOS GUI archive smoke: qualification tag must be v$version" >&2
  exit 1
fi

"$cli" doctor --json > "$smoke_root/doctor.json"
"$cli" agent capabilities --json > "$smoke_root/capabilities.json"
jq -e '
  .ok == true
  and .data.python_required == false
  and .data.embedded_typst == "verified"
  and .data.runtime_package_downloads == false
' "$smoke_root/doctor.json" >/dev/null
"$script_dir/smoke_documented_quickstart.sh" \
  "$cli" \
  "$smoke_root/documented-workflow"
"$script_dir/smoke_host_agent.sh" \
  "$cli" \
  "$smoke_root/host-agent-workflow"

home="$smoke_root/home"
mkdir -p "$home"
HOME="$home" "$gui" >"$smoke_root/gui.log" 2>&1 &
gui_pid="$!"
cleanup_gui() {
  if kill -0 "$gui_pid" 2>/dev/null; then
    kill "$gui_pid" 2>/dev/null || true
    wait "$gui_pid" 2>/dev/null || true
  fi
}
trap cleanup_gui EXIT
for _ in $(seq 1 40); do
  if ! kill -0 "$gui_pid" 2>/dev/null; then
    echo "macOS GUI archive smoke: packaged GUI exited during launch smoke" >&2
    sed -n '1,120p' "$smoke_root/gui.log" >&2
    exit 1
  fi
  sleep 0.05
done
kill "$gui_pid"
wait "$gui_pid" 2>/dev/null || true
trap - EXIT

archive_sha256="$(shasum -a 256 "$archive" | awk '{print $1}')"
archive_size="$(stat -f '%z' "$archive")"
if [[ "$qualification" == true ]]; then
  mkdir -p "$(dirname "$evidence")"
  completed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  jq -n \
    --arg tag "$tag" \
    --arg version "$version" \
    --arg environment "$environment" \
    --arg archive "$(basename "$archive")" \
    --arg archive_sha256 "$archive_sha256" \
    --argjson archive_size "$archive_size" \
    --arg completed_at "$completed_at" \
    --argjson github_run_id "$GITHUB_RUN_ID" \
    '{
      schema: "canisend.macos-gui-qualification/v1",
      record: "desktop-macos-aarch64",
      target: "aarch64-apple-darwin",
      environment: $environment,
      tag: $tag,
      version: $version,
      archive: {
        file: $archive,
        sha256: $archive_sha256,
        size: $archive_size
      },
      github_run_id: $github_run_id,
      checks: {
        bounded_archive: true,
        exact_top_level: true,
        no_symlinks: true,
        companion_integrity: true,
        nested_adhoc_signatures: true,
        outer_adhoc_signature: true,
        version_match: true,
        packaged_cli_doctor: true,
        packaged_cli_synthetic_workflow: true,
        packaged_host_agent_workflow: true,
        packaged_gui_launch: true,
        no_publication: true
      },
      completed_at: $completed_at
    }' > "$evidence"
fi

echo "macOS GUI archive smoke: exact packaged app passed ($archive_sha256)"
