#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: $0 FROM_ASSETS TO_ASSETS FROM_TAG TO_TAG TARGET ENVIRONMENT OUTPUT" >&2
  exit 2
fi

from_assets="$1"
to_assets="$2"
from_tag="$3"
to_tag="$4"
target="$5"
environment="$6"
output="$7"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
from_assets="$(canisend_absolute_path "$from_assets")"
to_assets="$(canisend_absolute_path "$to_assets")"
output="$(canisend_absolute_path "$output")"

case "$target:$environment:$(uname -m)" in
  aarch64-apple-darwin:macos-15:arm64)
    record="upgrade-aarch64-apple-darwin"
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-apple-darwin:macos-15-intel:x86_64)
    record="upgrade-x86_64-apple-darwin"
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-unknown-linux-gnu:ubuntu-24.04:x86_64)
    record="upgrade-x86_64-unknown-linux-gnu"
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-unknown-linux-musl:ubuntu-24.04:x86_64)
    record="upgrade-x86_64-unknown-linux-musl"
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-pc-windows-msvc:windows-2025:x86_64)
    record="upgrade-x86_64-pc-windows-msvc"
    archive_extension="zip"
    executable_name="canisend.exe"
    ;;
  *)
    echo "archive upgrade environment does not match $target/$environment" >&2
    exit 1
    ;;
esac

: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
command -v jq >/dev/null
if [[ -e "$output" || -L "$output" ]]; then
  echo "archive upgrade evidence destination already exists: $output" >&2
  exit 1
fi

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

extract_archive() {
  local archive="$1"
  local destination="$2"
  mkdir -p "$destination"
  case "$archive" in
    *.tar.gz)
      tar -xzf "$archive" -C "$destination"
      ;;
    *.zip)
      if command -v 7z >/dev/null 2>&1; then
        7z x -y "-o$destination" "$archive" >/dev/null
      elif command -v unzip >/dev/null 2>&1; then
        unzip -q "$archive" -d "$destination"
      else
        echo "archive upgrade requires 7z or unzip for zip archives" >&2
        exit 1
      fi
      ;;
    *)
      echo "archive upgrade does not support $archive" >&2
      exit 1
      ;;
  esac
}

verify_manifest_archive() {
  local manifest="$1"
  local archive="$2"
  local tag="$3"
  local stage="$4"
  local version="$5"
  local declared
  jq -e \
    --arg tag "$tag" \
    --arg stage "$stage" \
    --arg version "$version" \
    --arg target "$target" \
    --arg archive "$(basename "$archive")" '
      .schema == "canisend.release-manifest/v1" and
      .tag == $tag and .stage == $stage and .version == $version and
      ([.artifacts[] | select(.target == $target and .archive == $archive)] | length) == 1
    ' "$manifest" >/dev/null
  declared="$(jq -er --arg target "$target" '.artifacts[] | select(.target == $target) | .sha256' "$manifest")"
  test "$declared" = "$(sha256_file "$archive")"
}

copy_install_unit() {
  local bundle="$1"
  local destination="$2"
  rm -rf "$destination"
  mkdir -p "$destination"
  for name in \
    "$executable_name" \
    LICENSE \
    THIRD_PARTY_NOTICES.md \
    TYPST-ASSETS-LICENSE \
    TYPST-ASSETS-NOTICE \
    KNOWN_LIMITATIONS.md \
    INSTALL.md \
    PRIVACY.md \
    SECURITY.md
  do
    test -f "$bundle/$name"
    test ! -L "$bundle/$name"
    cp "$bundle/$name" "$destination/$name"
  done
  chmod +x "$destination/$executable_name"
}

from_version="${from_tag#v}"
to_version="${to_tag#v}"
from_manifest="$from_assets/canisend-$from_version-manifest.json"
to_manifest="$to_assets/canisend-$to_version-manifest.json"
from_archive="$from_assets/canisend-$from_version-$target.$archive_extension"
to_archive="$to_assets/canisend-$to_version-$target.$archive_extension"
for required in "$from_manifest" "$to_manifest" "$from_archive" "$to_archive"; do
  if [[ ! -f "$required" || -L "$required" ]]; then
    echo "archive upgrade input must be a regular non-symlink file: $required" >&2
    exit 1
  fi
done

verify_manifest_archive "$from_manifest" "$from_archive" "$from_tag" beta "$from_version"
verify_manifest_archive "$to_manifest" "$to_archive" "$to_tag" rc "$to_version"
from_manifest_sha256="$(sha256_file "$from_manifest")"
to_manifest_sha256="$(sha256_file "$to_manifest")"
from_archive_sha256="$(sha256_file "$from_archive")"
to_archive_sha256="$(sha256_file "$to_archive")"
test "$from_manifest_sha256" != "$to_manifest_sha256"
test "$from_archive_sha256" != "$to_archive_sha256"

root="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/canisend-archive-upgrade.XXXXXX")"
cleanup() {
  rm -rf "$root"
}
trap cleanup EXIT
extract_archive "$from_archive" "$root/from"
extract_archive "$to_archive" "$root/to"
from_bundle="$(find "$root/from" -mindepth 1 -maxdepth 1 -type d -name "canisend-*-$target" -print -quit)"
to_bundle="$(find "$root/to" -mindepth 1 -maxdepth 1 -type d -name "canisend-*-$target" -print -quit)"
if [[ -z "$from_bundle" || -z "$to_bundle" ]]; then
  echo "archive upgrade could not find both extracted bundles" >&2
  exit 1
fi
if find "$from_bundle" "$to_bundle" -type l -print -quit | grep -q .; then
  echo "archive upgrade rejects symlinks in extracted bundles" >&2
  exit 1
fi

from_binary="$from_bundle/$executable_name"
to_binary="$to_bundle/$executable_name"
chmod +x "$from_binary" "$to_binary"
install_root="$root/installed"
workspace="$root/workspace"
backup="$root/pre-upgrade-backup"
restored="$root/restored-workspace"
host_pack="$root/codex-host-pack"
copy_install_unit "$from_bundle" "$install_root"
installed="$install_root/$executable_name"

from_observed="$("$installed" version --json | jq -er 'select(.ok == true) | .data.version')"
test "$from_observed" = "$from_version"
"$installed" doctor --json | jq -e '
  .ok == true and .data.python_required == false and
  .data.embedded_typst == "verified" and .data.resource_manifest == "verified"
' >/dev/null
"$installed" --workspace "$workspace" workspace init --json | jq -e '.ok == true' >/dev/null
"$installed" --workspace "$workspace" job create \
  --title "Synthetic qualification role" \
  --institution "CanISend release qualification" \
  --json | jq -e '.ok == true' >/dev/null
"$installed" --workspace "$workspace" workspace check --json | jq -e '.ok == true and .data.ok == true' >/dev/null
before_schema="$("$installed" --workspace "$workspace" workspace status --json | jq -er 'select(.ok == true) | .data.database_schema_version')"
"$installed" --workspace "$workspace" workspace backup "$backup" --json | jq -e '.ok == true' >/dev/null
test -f "$backup/backup-manifest.json"

copy_install_unit "$to_bundle" "$install_root"
installed="$install_root/$executable_name"
to_observed="$("$installed" version --json | jq -er 'select(.ok == true) | .data.version')"
test "$to_observed" = "$to_version"
"$installed" doctor --json | jq -e '
  .ok == true and .data.python_required == false and
  .data.embedded_typst == "verified" and .data.resource_manifest == "verified"
' >/dev/null
"$installed" --workspace "$workspace" workspace check --json | jq -e '.ok == true and .data.ok == true' >/dev/null
after_schema="$("$installed" --workspace "$workspace" workspace status --json | jq -er 'select(.ok == true) | .data.database_schema_version')"

database="$workspace/.canisend/state.sqlite3"
test -f "$database"
before_old_attempt_sha256="$(sha256_file "$database")"
set +e
"$from_binary" --workspace "$workspace" workspace status --json >"$root/old-binary.json" 2>"$root/old-binary.stderr"
old_status=$?
set -e
after_old_attempt_sha256="$(sha256_file "$database")"
if [[ $old_status -eq 0 ]]; then
  jq -e '.ok == true' "$root/old-binary.json" >/dev/null
  test "$before_schema" = "$after_schema"
  old_binary_behavior="same-schema-accepted"
else
  jq -e '.ok == false and .error.code == "workspace.conflict"' "$root/old-binary.json" >/dev/null
  test "$after_schema" -gt "$before_schema"
  test "$before_old_attempt_sha256" = "$after_old_attempt_sha256"
  old_binary_behavior="future-schema-rejected-without-mutation"
fi

"$from_binary" workspace restore "$backup" "$restored" --json | jq -e '.ok == true' >/dev/null
"$from_binary" --workspace "$restored" workspace check --json | jq -e '.ok == true and .data.ok == true' >/dev/null
"$installed" agent assets export --host codex --destination "$host_pack" --json | jq -e '.ok == true' >/dev/null
test -f "$host_pack/canisend-agent-pack.json"

rm -rf "$install_root"
test ! -e "$install_root"
test -f "$workspace/canisend.toml"
test -f "$workspace/.canisend/state.sqlite3"
test -f "$backup/backup-manifest.json"
test -f "$restored/canisend.toml"
test -f "$restored/.canisend/state.sqlite3"

completed_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
mkdir -p "$(dirname "$output")"
jq -n \
  --arg record "$record" \
  --arg target "$target" \
  --arg environment "$environment" \
  --arg from_tag "$from_tag" \
  --arg to_tag "$to_tag" \
  --arg from_manifest_sha256 "$from_manifest_sha256" \
  --arg to_manifest_sha256 "$to_manifest_sha256" \
  --arg from_archive_sha256 "$from_archive_sha256" \
  --arg to_archive_sha256 "$to_archive_sha256" \
  --arg from_version "$from_observed" \
  --arg to_version "$to_observed" \
  --arg old_binary_behavior "$old_binary_behavior" \
  --arg completed_at "$completed_at" \
  --argjson github_run_id "$GITHUB_RUN_ID" \
  --argjson before_schema "$before_schema" \
  --argjson after_schema "$after_schema" '
  {
    schema: "canisend.upgrade-qualification/v1",
    record: $record,
    target: $target,
    environment: $environment,
    from_tag: $from_tag,
    to_tag: $to_tag,
    manifests: {
      from_sha256: $from_manifest_sha256,
      to_sha256: $to_manifest_sha256
    },
    archives: {
      from_sha256: $from_archive_sha256,
      to_sha256: $to_archive_sha256
    },
    github_run_id: $github_run_id,
    observed_versions: {from: $from_version, to: $to_version},
    database_schemas: {before: $before_schema, after: $after_schema},
    old_binary_behavior: $old_binary_behavior,
    checks: {
      "verified-release-pair": true,
      "from-version-and-doctor": true,
      "workspace-created-and-checked": true,
      "verified-pre-upgrade-backup": true,
      "to-version-and-doctor": true,
      "workspace-upgraded-and-checked": true,
      "old-binary-behavior-verified": true,
      "backup-restored-to-new-path": true,
      "restored-workspace-checked-by-old-binary": true,
      "host-pack-regenerated": true,
      "installed-binary-and-notices-uninstalled": true,
      "workspace-backup-and-restore-retained": true,
      "no-publication": true
    },
    completed_at: $completed_at
  }
' >"$output"

echo "archive upgrade qualification: wrote $output ($target)"
