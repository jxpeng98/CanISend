#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: $0 ALPHA5_ASSETS ALPHA6_ARCHIVE TARGET ENVIRONMENT TAG SOURCE_COMMIT OUTPUT" >&2
  exit 2
fi

alpha5_assets="$1"
alpha6_archive="$2"
target="$3"
environment="$4"
tag="$5"
source_commit="$6"
output="$7"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source "$script_dir/lib/native_paths.sh"
alpha5_assets="$(canisend_absolute_path "$alpha5_assets")"
alpha6_archive="$(canisend_absolute_path "$alpha6_archive")"
output="$(canisend_absolute_path "$output")"

case "$target:$environment:$(uname -m)" in
  aarch64-apple-darwin:macos-15:arm64)
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-apple-darwin:macos-15-intel:x86_64)
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-unknown-linux-gnu:ubuntu-24.04:x86_64)
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-unknown-linux-musl:ubuntu-24.04:x86_64)
    archive_extension="tar.gz"
    executable_name="canisend"
    ;;
  x86_64-pc-windows-msvc:windows-2025:x86_64)
    archive_extension="zip"
    executable_name="canisend.exe"
    ;;
  *)
    echo "Alpha.6 migration environment does not match $target/$environment" >&2
    exit 1
    ;;
esac

if [[ "$tag" != "v1.0.0-alpha.6" ]]; then
  echo "Alpha.6 migration qualification requires v1.0.0-alpha.6, got $tag" >&2
  exit 1
fi
if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Alpha.6 migration source commit must be a full lowercase SHA-1" >&2
  exit 1
fi
if [[ -e "$output" || -L "$output" ]]; then
  echo "Alpha.6 migration evidence destination already exists: $output" >&2
  exit 1
fi
command -v jq >/dev/null

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
        echo "Alpha.6 migration qualification requires 7z or unzip" >&2
        exit 1
      fi
      ;;
    *)
      echo "unsupported migration qualification archive: $archive" >&2
      exit 1
      ;;
  esac
}

alpha5_tag="v1.0.0-alpha.5"
alpha5_version="1.0.0-alpha.5"
alpha6_version="${tag#v}"
alpha5_manifest="$alpha5_assets/canisend-$alpha5_version-manifest.json"
alpha5_archive="$alpha5_assets/canisend-$alpha5_version-$target.$archive_extension"
for required in "$alpha5_manifest" "$alpha5_archive" "$alpha6_archive"; do
  if [[ ! -f "$required" || -L "$required" ]]; then
    echo "migration qualification input must be a regular non-symlink file: $required" >&2
    exit 1
  fi
done

jq -e \
  --arg tag "$alpha5_tag" \
  --arg version "$alpha5_version" \
  --arg target "$target" \
  --arg archive "$(basename "$alpha5_archive")" '
    .schema == "canisend.release-manifest/v1" and
    .tag == $tag and .stage == "alpha" and .version == $version and
    ([.artifacts[] | select(.target == $target and .archive == $archive)] | length) == 1
  ' "$alpha5_manifest" >/dev/null
alpha5_declared_sha256="$(
  jq -er --arg target "$target" '.artifacts[] | select(.target == $target) | .sha256' \
    "$alpha5_manifest"
)"
test "$alpha5_declared_sha256" = "$(sha256_file "$alpha5_archive")"

root="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/canisend-alpha6-migration.XXXXXX")"
cleanup() {
  rm -rf "$root"
}
trap cleanup EXIT
extract_archive "$alpha5_archive" "$root/alpha5"
extract_archive "$alpha6_archive" "$root/alpha6"
alpha5_bundle="$(
  find "$root/alpha5" -mindepth 1 -maxdepth 1 -type d \
    -name "canisend-$alpha5_version-$target" -print -quit
)"
alpha6_bundle="$(
  find "$root/alpha6" -mindepth 1 -maxdepth 1 -type d \
    -name "canisend-$alpha6_version-$target" -print -quit
)"
if [[ -z "$alpha5_bundle" || -z "$alpha6_bundle" ]]; then
  echo "migration qualification could not find both extracted bundles" >&2
  exit 1
fi
if find "$alpha5_bundle" "$alpha6_bundle" -type l -print -quit | grep -q .; then
  echo "migration qualification rejects symlinks in extracted bundles" >&2
  exit 1
fi

alpha5_binary="$alpha5_bundle/$executable_name"
alpha6_binary="$alpha6_bundle/$executable_name"
chmod +x "$alpha5_binary" "$alpha6_binary"
test "$("$alpha5_binary" version --json | jq -er 'select(.ok == true) | .data.version')" = "$alpha5_version"
test "$("$alpha6_binary" version --json | jq -er 'select(.ok == true) | .data.version')" = "$alpha6_version"
"$alpha5_binary" doctor --json | jq -e '.ok == true and .data.resource_manifest == "verified"' >/dev/null
"$alpha6_binary" doctor --json | jq -e '.ok == true and .data.resource_manifest == "verified"' >/dev/null

workspace="$root/academic-v2"
pre_upgrade_backup="$root/pre-upgrade-backup"
pre_migration_backup="$root/pre-migration-backup"
"$alpha5_binary" --workspace "$workspace" workspace init --json | jq -e '
  .ok == true and .data.workspace_format == "canisend.workspace/v2" and
  .data.database_schema_version == 13
' >/dev/null
job_id="$(
  "$alpha5_binary" --workspace "$workspace" job create \
    --title "Synthetic migration role" \
    --institution "CanISend qualification" \
    --json | jq -er 'select(.ok == true) | .data.id'
)"
"$alpha5_binary" --workspace "$workspace" job import "$job_id" \
  --file "$repo_root/fixtures/v2-spec/job-advert.md" --json | jq -e '.ok == true' >/dev/null
"$alpha5_binary" --workspace "$workspace" workflow start --job "$job_id" --json \
  | jq -e '.ok == true' >/dev/null
"$alpha5_binary" --workspace "$workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null
"$alpha5_binary" --workspace "$workspace" workspace backup "$pre_upgrade_backup" --json \
  | jq -e '.ok == true and .data.format == "canisend.backup/v2"' >/dev/null
test -f "$pre_upgrade_backup/backup-manifest.json"

"$alpha6_binary" --workspace "$workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null
preview="$root/migration-preview.json"
"$alpha6_binary" --workspace "$workspace" workspace migration-preview --json >"$preview"
academic_pack_id="$(jq -er 'select(.ok == true) | .data.pack.id' "$preview")"
academic_pack_version="$(jq -er '.data.pack.version' "$preview")"
academic_pack_digest="$(jq -er '.data.pack.content_digest' "$preview")"
migration_plan_sha256="$(jq -er '.data.migration_plan_sha256' "$preview")"
jq -e '
  .data.format == "canisend.workspace-migration-preview/v3" and
  .data.source_workspace_format == "canisend.workspace/v2" and
  .data.target_workspace_format == "canisend.workspace/v3" and
  .data.application_count == 1 and .data.referenced_blob_count == 1 and
  .data.rollback_boundary == "restore-verified-pre-migration-backup-to-new-path"
' "$preview" >/dev/null

set +e
"$alpha6_binary" --workspace "$workspace" workspace migrate \
  --expected-plan-sha256 0000000000000000000000000000000000000000000000000000000000000000 \
  --backup-destination "$root/rejected-backup" --json >"$root/rejected-migration.json"
rejected_status=$?
set -e
test "$rejected_status" -ne 0
jq -e '.ok == false and .error.code == "workspace.conflict"' "$root/rejected-migration.json" >/dev/null
test ! -e "$root/rejected-backup"
jq -e --arg digest "$migration_plan_sha256" '.data.migration_plan_sha256 == $digest' "$preview" >/dev/null

"$alpha6_binary" --workspace "$workspace" workspace migrate \
  --expected-plan-sha256 "$migration_plan_sha256" \
  --backup-destination "$pre_migration_backup" --json >"$root/migration-result.json"
jq -e \
  --arg plan "$migration_plan_sha256" \
  --arg pack_id "$academic_pack_id" \
  --arg pack_version "$academic_pack_version" \
  --arg pack_digest "$academic_pack_digest" '
    .ok == true and .data.migration.format == "canisend.workspace-migration-result/v3" and
    .data.migration.migration_plan_sha256 == $plan and
    .data.migration.pack.id == $pack_id and
    .data.migration.pack.version == $pack_version and
    .data.migration.pack.content_digest == $pack_digest and
    .data.migration.source_inventory_sha256 == .data.migration.post_migration_inventory_sha256 and
    (.data.migration.application_ids | length) == 1
  ' "$root/migration-result.json" >/dev/null
test -f "$pre_migration_backup/backup-manifest.json"
"$alpha6_binary" --workspace "$workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null
"$alpha6_binary" --workspace "$workspace" workspace status --json \
  | jq -e '.ok == true and .data.workspace_format == "canisend.workspace/v3"' >/dev/null

database="$workspace/.canisend/state.sqlite3"
before_old_attempt_sha256="$(sha256_file "$database")"
set +e
"$alpha5_binary" --workspace "$workspace" workspace status --json >"$root/old-binary.json"
old_binary_status=$?
set -e
test "$old_binary_status" -ne 0
jq -e '.ok == false' "$root/old-binary.json" >/dev/null
test "$before_old_attempt_sha256" = "$(sha256_file "$database")"

restored_alpha5="$root/restored-alpha5"
restored_alpha6="$root/restored-alpha6-v2"
"$alpha5_binary" workspace restore "$pre_upgrade_backup" "$restored_alpha5" --json \
  | jq -e '.ok == true and .data.workspace_format == "canisend.workspace/v2"' >/dev/null
"$alpha5_binary" --workspace "$restored_alpha5" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null
"$alpha6_binary" workspace restore "$pre_migration_backup" "$restored_alpha6" --json \
  | jq -e '.ok == true and .data.workspace_format == "canisend.workspace/v2"' >/dev/null
"$alpha6_binary" --workspace "$restored_alpha6" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

generic_workspace="$root/generic-v3"
"$alpha6_binary" --workspace "$generic_workspace" workspace init \
  --pack generic-application --json | jq -e '
    .ok == true and .data.workspace_format == "canisend.workspace/v3"
  ' >/dev/null
"$alpha6_binary" --workspace "$generic_workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

install_root="$root/installed"
retained_workspace="$root/retained-workspace"
mkdir -p "$install_root"
cp "$alpha6_binary" "$install_root/$executable_name"
chmod +x "$install_root/$executable_name"
"$install_root/$executable_name" --workspace "$retained_workspace" workspace init \
  --pack generic-application --json | jq -e '.ok == true' >/dev/null
rm -rf "$install_root"
test ! -e "$install_root"
test -f "$retained_workspace/canisend.toml"
test -d "$retained_workspace/.canisend"

completed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
mkdir -p "$(dirname "$output")"
jq -n \
  --arg target "$target" \
  --arg environment "$environment" \
  --arg from_tag "$alpha5_tag" \
  --arg to_tag "$tag" \
  --arg source_commit "$source_commit" \
  --arg from_archive_sha256 "$alpha5_declared_sha256" \
  --arg to_archive_sha256 "$(sha256_file "$alpha6_archive")" \
  --arg academic_pack_id "$academic_pack_id" \
  --arg academic_pack_version "$academic_pack_version" \
  --arg academic_pack_digest "$academic_pack_digest" \
  --arg migration_plan_sha256 "$migration_plan_sha256" \
  --arg completed_at "$completed_at" \
  --argjson github_run_id "${GITHUB_RUN_ID:-0}" '
  {
    schema: "canisend.alpha6-migration-qualification/v1",
    record: ("alpha6-migration-" + $target),
    target: $target,
    environment: $environment,
    from_tag: $from_tag,
    to_tag: $to_tag,
    source_commit: $source_commit,
    from_archive_sha256: $from_archive_sha256,
    to_archive_sha256: $to_archive_sha256,
    pack: {
      id: $academic_pack_id,
      version: $academic_pack_version,
      content_digest: $academic_pack_digest
    },
    migration_plan_sha256: $migration_plan_sha256,
    github_run_id: $github_run_id,
    checks: {
      "exact-alpha5-public-archive": true,
      "exact-alpha6-candidate-archive": true,
      "fresh-academic-v2-workspace": true,
      "pre-upgrade-backup": true,
      "stale-plan-rejected-without-backup": true,
      "v2-to-v3-migration": true,
      "inventory-digest-preserved": true,
      "old-binary-refused-without-mutation": true,
      "pre-upgrade-backup-restored-by-alpha5": true,
      "pre-migration-backup-restored-by-alpha6": true,
      "generic-v3-initialization": true,
      uninstall: true,
      "workspace-retained": true,
      "no-publication": true
    },
    completed_at: $completed_at
  }
  ' >"$output"

echo "Alpha.6 migration qualification: ok ($target)"
