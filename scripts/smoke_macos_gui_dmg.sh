#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 && $# -ne 6 ]]; then
  echo "usage: $0 DMG NEW_SMOKE_DIRECTORY [TAG ENVIRONMENT PROFILE EVIDENCE]" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI DMG smoke: this script must run on macOS" >&2
  exit 1
fi

dmg="$1"
smoke_root="$2"
qualification=false
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
dmg="$(canisend_absolute_path "$dmg")"
smoke_root="$(canisend_absolute_path "$smoke_root")"
if [[ $# -eq 6 ]]; then
  qualification=true
  tag="$3"
  environment="$4"
  profile="$5"
  evidence="$(canisend_absolute_path "$6")"
  : "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required for qualification evidence}"
  if [[ ! "$GITHUB_RUN_ID" =~ ^[1-9][0-9]*$ ]]; then
    echo "macOS GUI DMG smoke: GITHUB_RUN_ID must be a positive integer" >&2
    exit 1
  fi
  if [[ "$environment:$(uname -m)" != "macos-15:arm64" ]]; then
    echo "macOS GUI DMG smoke: qualification requires macos-15 on arm64" >&2
    exit 1
  fi
  if [[ "$tag" == *-alpha.* ]]; then
    expected_profile="release-alpha"
  else
    expected_profile="release"
  fi
  if [[ "$profile" != "$expected_profile" ]]; then
    echo "macOS GUI DMG smoke: profile does not match the validated release stage" >&2
    exit 1
  fi
  if [[ -e "$evidence" || -L "$evidence" ]]; then
    echo "macOS GUI DMG smoke: evidence destination must not exist: $evidence" >&2
    exit 1
  fi
fi

for command in hdiutil jq plutil readlink shasum stat; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI DMG smoke: required command is missing: $command" >&2
    exit 1
  fi
done
if [[ ! -f "$dmg" || -L "$dmg" || "$dmg" != *.dmg ]]; then
  echo "macOS GUI DMG smoke: image must be a regular non-symlink DMG" >&2
  exit 1
fi
if (( $(stat -f '%z' "$dmg") <= 0 || $(stat -f '%z' "$dmg") > 268435456 )); then
  echo "macOS GUI DMG smoke: image size is outside the 1..256 MiB bound" >&2
  exit 1
fi
if [[ -e "$smoke_root" ]]; then
  echo "macOS GUI DMG smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi

mkdir -p "$smoke_root"
hdiutil verify "$dmg" >/dev/null
attach_plist="$smoke_root/attach.plist"
mounted=false
mount_point=""
cleanup() {
  if [[ "$mounted" == true && -n "$mount_point" ]]; then
    hdiutil detach "$mount_point" -quiet >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

hdiutil attach \
  -readonly \
  -nobrowse \
  -noautoopen \
  -plist \
  "$dmg" > "$attach_plist"
mount_points="$(
  plutil -convert json -o - "$attach_plist" \
    | jq -r '.["system-entities"][] | select(.["mount-point"] != null) | .["mount-point"]'
)"
if [[ -z "$mount_points" || "$mount_points" == *$'\n'* ]]; then
  echo "macOS GUI DMG smoke: image must mount exactly one filesystem" >&2
  exit 1
fi
mount_point="$mount_points"
mounted=true
if [[ "$mount_point" != /Volumes/* || ! -d "$mount_point" ]]; then
  echo "macOS GUI DMG smoke: image mounted at an unexpected location" >&2
  exit 1
fi

top_level="$(
  find "$mount_point" -mindepth 1 -maxdepth 1 -print \
    | sed "s#^$mount_point/##" \
    | sort
)"
if [[ "$top_level" != $'Applications\nCanISend.app\nCanISend.app.manifest.json' ]]; then
  echo "macOS GUI DMG smoke: mounted top level differs from the frozen contract" >&2
  exit 1
fi
applications_link="$mount_point/Applications"
if [[ ! -L "$applications_link" || "$(readlink "$applications_link")" != "/Applications" ]]; then
  echo "macOS GUI DMG smoke: Applications must be the exact /Applications symlink" >&2
  exit 1
fi

app="$mount_point/CanISend.app"
manifest="$mount_point/CanISend.app.manifest.json"
"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest"
cli="$app/Contents/Resources/bin/canisend"
version="$("$cli" version --json | jq -er 'select(.ok == true) | .data.version')"
expected_name="CanISend-$version-aarch64-apple-darwin.dmg"
if [[ "$(basename "$dmg")" != "$expected_name" ]]; then
  echo "macOS GUI DMG smoke: image must be named $expected_name" >&2
  exit 1
fi
if [[ "$qualification" == true && "$tag" != "v$version" ]]; then
  echo "macOS GUI DMG smoke: qualification tag must be v$version" >&2
  exit 1
fi

cleanup
mounted=false
trap - EXIT

dmg_sha256="$(shasum -a 256 "$dmg" | awk '{print $1}')"
dmg_size="$(stat -f '%z' "$dmg")"
if [[ "$qualification" == true ]]; then
  mkdir -p "$(dirname "$evidence")"
  completed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  jq -n \
    --arg tag "$tag" \
    --arg version "$version" \
    --arg environment "$environment" \
    --arg profile "$profile" \
    --arg dmg "$(basename "$dmg")" \
    --arg dmg_sha256 "$dmg_sha256" \
    --argjson dmg_size "$dmg_size" \
    --arg completed_at "$completed_at" \
    --argjson github_run_id "$GITHUB_RUN_ID" \
    '{
      schema: "canisend.macos-gui-dmg-qualification/v1",
      record: "desktop-macos-aarch64-dmg",
      target: "aarch64-apple-darwin",
      environment: $environment,
      profile: $profile,
      tag: $tag,
      version: $version,
      image: {
        file: $dmg,
        sha256: $dmg_sha256,
        size: $dmg_size
      },
      github_run_id: $github_run_id,
      checks: {
        bounded_image: true,
        hdiutil_verify: true,
        readonly_mount: true,
        exact_top_level: true,
        applications_link: true,
        companion_integrity: true,
        nested_adhoc_signatures: true,
        outer_adhoc_signature: true,
        version_match: true,
        no_publication: true
      },
      completed_at: $completed_at
    }' > "$evidence"
fi

echo "macOS GUI DMG smoke: exact read-only image passed ($dmg_sha256)"
