#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 PUBLIC_V0.7.0_RC2_ASSETS NEW_EVIDENCE_JSON" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "public 0.7 upgrade qualification requires Apple Silicon macOS" >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source "$script_dir/lib/native_paths.sh"
assets="$(canisend_absolute_path "$1")"
evidence="$(canisend_absolute_path "$2")"
if [[ ! -d "$assets" || -L "$assets" ]]; then
  echo "public 0.7 assets must be a regular directory: $assets" >&2
  exit 1
fi
if [[ -e "$evidence" || -L "$evidence" ]]; then
  echo "public 0.7 upgrade evidence destination already exists: $evidence" >&2
  exit 1
fi

for command in cargo codesign git jq shasum tar; do
  command -v "$command" >/dev/null
done

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

tag="v0.7.0-rc.2"
old_version="${tag#v}"
target="aarch64-apple-darwin"
archive_name="canisend-$old_version-$target.tar.gz"
manifest_name="canisend-$old_version-manifest.json"
signing_name="canisend-$old_version-$target-signing.json"
archive="$assets/$archive_name"
manifest="$assets/$manifest_name"
signing="$assets/$signing_name"
for required in "$archive" "$manifest" "$signing" "$assets/SHA256SUMS"; do
  if [[ ! -f "$required" || -L "$required" ]]; then
    echo "public 0.7 upgrade input must be a regular non-symlink file: $required" >&2
    exit 1
  fi
done

cd "$repo_root"
cargo run -p xtask --locked -- release verify "$tag" "$assets"

archive_sha256="$(sha256_file "$archive")"
manifest_sha256="$(sha256_file "$manifest")"
declared_archive_sha256="$(jq -er \
  --arg target "$target" \
  '.artifacts[] | select(.target == $target) | .sha256' \
  "$manifest")"
signing_archive_sha256="$(jq -er '.archive.sha256' "$signing")"
old_binary_sha256="$(jq -er '.binary.sha256' "$signing")"
if [[ "$archive_sha256" != "$declared_archive_sha256" \
  || "$archive_sha256" != "$signing_archive_sha256" ]]; then
  echo "public 0.7 archive digest does not match its verified metadata" >&2
  exit 1
fi
jq -e \
  --arg version "$old_version" \
  --arg target "$target" '
    .schema == "canisend.code-signing-evidence/v2" and
    .status == "verified" and .kind == "apple-adhoc" and
    .version == $version and .target == $target and
    .binary.file == "canisend" and
    .verification.codesign_valid == true and
    .verification.adhoc == true and
    .verification.developer_id == false and
    .verification.notarized == false
  ' "$signing" >/dev/null

fixture_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-public-07-upgrade.XXXXXX")"
cleanup() {
  rm -rf "$fixture_root"
}
trap cleanup EXIT
bundle_name="canisend-$old_version-$target"
while IFS= read -r entry; do
  case "$entry" in
    "$bundle_name"|"$bundle_name/"|"$bundle_name/"*) ;;
    *)
      echo "public 0.7 archive contains an unexpected path: $entry" >&2
      exit 1
      ;;
  esac
done < <(tar -tzf "$archive")
mkdir -p "$fixture_root/extracted"
tar -xzf "$archive" -C "$fixture_root/extracted"
old_bundle="$fixture_root/extracted/$bundle_name"
old_cli="$old_bundle/canisend"
if [[ ! -f "$old_cli" || -L "$old_cli" ]] \
  || find "$old_bundle" -type l -print -quit | grep -q .; then
  echo "public 0.7 extracted bundle is missing its regular CLI or contains a symlink" >&2
  exit 1
fi
if [[ "$(sha256_file "$old_cli")" != "$old_binary_sha256" ]]; then
  echo "public 0.7 extracted CLI digest does not match signing evidence" >&2
  exit 1
fi
chmod +x "$old_cli"
codesign --verify --strict "$old_cli"
old_observed="$("$old_cli" version --json | jq -er 'select(.ok == true) | .data.version')"
if [[ "$old_observed" != "$old_version" ]]; then
  echo "public 0.7 extracted CLI reported unexpected version: $old_observed" >&2
  exit 1
fi

cargo build -p canisend-cli --release --locked
current_source="$repo_root/target/release/canisend"
current_cli="$fixture_root/current/canisend"
mkdir -p "$(dirname "$current_cli")"
cp "$current_source" "$current_cli"
codesign \
  --force \
  --identifier io.github.jxpeng98.canisend.cli.public-upgrade \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$current_cli"
codesign --verify --strict "$current_cli"
current_observed="$("$current_cli" version --json | jq -er 'select(.ok == true) | .data.version')"
if [[ "$current_observed" != "1.0.0-alpha.1" ]]; then
  echo "current CLI reported unexpected version: $current_observed" >&2
  exit 1
fi
current_cli_sha256="$(sha256_file "$current_cli")"

CANISEND_TEST_PUBLIC_07_CLI="$old_cli" \
CANISEND_TEST_PUBLIC_07_CLI_SHA256="$old_binary_sha256" \
CANISEND_TEST_CURRENT_CLI="$current_cli" \
  cargo test \
    -p canisend-app \
    --locked \
    --test macos_gui_cli_lifecycle \
    -- \
    --ignored \
    --exact public_07_cli_upgrades_with_verified_backup_and_exact_rollback \
    --nocapture

completed_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
source_commit="$(git rev-parse HEAD)"
mkdir -p "$(dirname "$evidence")"
jq -S -n \
  --arg target "$target" \
  --arg from_tag "$tag" \
  --arg to_version "$current_observed" \
  --arg source_commit "$source_commit" \
  --arg manifest_sha256 "$manifest_sha256" \
  --arg archive_sha256 "$archive_sha256" \
  --arg old_binary_sha256 "$old_binary_sha256" \
  --arg current_cli_sha256 "$current_cli_sha256" \
  --arg completed_at "$completed_at" '
  {
    schema: "canisend.alpha-public-upgrade-evidence/v1",
    target: $target,
    from_tag: $from_tag,
    to_version: $to_version,
    source_commit: $source_commit,
    public_assets: {
      manifest_sha256: $manifest_sha256,
      archive_sha256: $archive_sha256,
      binary_sha256: $old_binary_sha256
    },
    candidate: {binary_sha256: $current_cli_sha256},
    checks: {
      "complete-public-release-verified": true,
      "public-archive-bound-to-manifest": true,
      "public-binary-bound-to-signing-evidence": true,
      "public-adhoc-signature-verified": true,
      "explicit-replacement-required": true,
      "failed-replacement-preserved-state": true,
      "verified-pre-upgrade-backup": true,
      "workspace-upgraded-and-checked": true,
      "backup-restored-by-old-binary": true,
      "backup-restored-by-new-binary": true,
      "old-binary-behavior-verified": true,
      "exact-old-binary-restored-on-uninstall": true,
      "workspace-backup-and-restores-retained": true,
      "no-publication": true
    },
    completed_at: $completed_at
  }
' >"$evidence"

echo "public 0.7 upgrade qualification: wrote $evidence"
