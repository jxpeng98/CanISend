#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 BINARY TARGET_TRIPLE EVIDENCE_JSON" >&2
  exit 2
fi

binary="$1"
target="$2"
evidence="$3"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
evidence="$(canisend_absolute_path "$evidence")"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin) ;;
  *)
    echo "macOS signing: unsupported target: $target" >&2
    exit 2
    ;;
esac
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS signing: this script must run on macOS" >&2
  exit 1
fi
if [[ ! -f "$binary" || -L "$binary" ]]; then
  echo "macOS signing: binary must be a regular non-symlink file: $binary" >&2
  exit 1
fi
if [[ -e "$evidence" ]]; then
  echo "macOS signing: evidence destination must not exist: $evidence" >&2
  exit 1
fi

required_environment=(
  CANISEND_APPLE_DEVELOPER_ID_P12_BASE64
  CANISEND_APPLE_DEVELOPER_ID_P12_PASSWORD
  CANISEND_APPLE_NOTARY_KEY_P8_BASE64
  CANISEND_APPLE_SIGNING_IDENTITY
  CANISEND_APPLE_TEAM_ID
  CANISEND_APPLE_NOTARY_KEY_ID
  CANISEND_APPLE_NOTARY_ISSUER_ID
)
for name in "${required_environment[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "macOS signing: required environment variable is missing: $name" >&2
    exit 1
  fi
done
for command in base64 codesign ditto jq openssl security shasum xcrun; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS signing: required command is missing: $command" >&2
    exit 1
  fi
done

work="$(mktemp -d "${RUNNER_TEMP:-/tmp}/canisend-macos-signing.XXXXXX")"
keychain="$work/canisend-signing.keychain-db"
certificate="$work/developer-id.p12"
notary_key="$work/AuthKey.p8"
notary_zip="$work/canisend-notary.zip"
notary_submission="$work/notary-submission.json"
notary_log="$(dirname "$evidence")/$(basename "$evidence" .json)-notary-log.json"
codesign_details="$work/codesign-details.txt"
keychain_password="$(openssl rand -hex 32)"

cleanup() {
  security delete-keychain "$keychain" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

mkdir -p "$(dirname "$evidence")"
printf '%s' "$CANISEND_APPLE_DEVELOPER_ID_P12_BASE64" | base64 --decode > "$certificate"
printf '%s' "$CANISEND_APPLE_NOTARY_KEY_P8_BASE64" | base64 --decode > "$notary_key"
chmod 600 "$certificate" "$notary_key"

security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
security import "$certificate" \
  -k "$keychain" \
  -P "$CANISEND_APPLE_DEVELOPER_ID_P12_PASSWORD" \
  -T /usr/bin/codesign
security set-key-partition-list \
  -S apple-tool:,apple: \
  -s \
  -k "$keychain_password" \
  "$keychain" >/dev/null
if ! security find-identity -v -p codesigning "$keychain" \
  | grep -F "\"$CANISEND_APPLE_SIGNING_IDENTITY\"" >/dev/null; then
  echo "macOS signing: configured Developer ID identity was not imported" >&2
  exit 1
fi

codesign \
  --force \
  --identifier io.github.jxpeng98.canisend \
  --keychain "$keychain" \
  --options runtime \
  --sign "$CANISEND_APPLE_SIGNING_IDENTITY" \
  --timestamp \
  "$binary"
codesign --verify --strict --verbose=4 "$binary"
codesign --display --verbose=4 "$binary" > "$codesign_details" 2>&1

identifier="$(sed -n 's/^Identifier=//p' "$codesign_details" | head -n 1)"
team_id="$(sed -n 's/^TeamIdentifier=//p' "$codesign_details" | head -n 1)"
authority="$(sed -n 's/^Authority=//p' "$codesign_details" | head -n 1)"
if [[ "$identifier" != "io.github.jxpeng98.canisend" ]]; then
  echo "macOS signing: code-signing identifier is invalid" >&2
  exit 1
fi
if [[ "$team_id" != "$CANISEND_APPLE_TEAM_ID" ]]; then
  echo "macOS signing: signed TeamIdentifier does not match the configured team" >&2
  exit 1
fi
if [[ "$authority" != "$CANISEND_APPLE_SIGNING_IDENTITY" \
  || "$authority" != Developer\ ID\ Application:* ]]; then
  echo "macOS signing: signature does not use the configured Developer ID Application identity" >&2
  exit 1
fi
if ! grep -Eq '^flags=.*\(runtime\)' "$codesign_details"; then
  echo "macOS signing: hardened runtime flag is missing" >&2
  exit 1
fi
if ! grep -q '^Timestamp=' "$codesign_details"; then
  echo "macOS signing: secure timestamp is missing" >&2
  exit 1
fi
if codesign --display --entitlements :- "$binary" 2>&1 \
  | grep -q 'com.apple.security.get-task-allow'; then
  echo "macOS signing: forbidden get-task-allow entitlement is present" >&2
  exit 1
fi

ditto -c -k --keepParent "$binary" "$notary_zip"
xcrun notarytool submit "$notary_zip" \
  --key "$notary_key" \
  --key-id "$CANISEND_APPLE_NOTARY_KEY_ID" \
  --issuer "$CANISEND_APPLE_NOTARY_ISSUER_ID" \
  --wait \
  --output-format json > "$notary_submission"
notary_status="$(jq -er '.status' "$notary_submission")"
notary_id="$(jq -er '.id' "$notary_submission")"
if [[ "$notary_status" != "Accepted" ]]; then
  echo "macOS signing: Apple notarization was not accepted" >&2
  exit 1
fi
xcrun notarytool log "$notary_id" \
  --key "$notary_key" \
  --key-id "$CANISEND_APPLE_NOTARY_KEY_ID" \
  --issuer "$CANISEND_APPLE_NOTARY_ISSUER_ID" \
  "$notary_log"
if [[ "$(jq -er '.status' "$notary_log")" != "Accepted" ]]; then
  echo "macOS signing: notarization log is not accepted" >&2
  exit 1
fi
notary_error_count="$(jq '[.issues[]? | select(.severity == "error")] | length' "$notary_log")"
notary_warning_count="$(jq '[.issues[]? | select(.severity == "warning")] | length' "$notary_log")"
if [[ "$notary_error_count" != "0" ]]; then
  echo "macOS signing: notarization log contains errors" >&2
  exit 1
fi

version_json="$("$binary" version --json)"
version="$(printf '%s' "$version_json" | jq -er '.data.version')"
binary_sha256="$(shasum -a 256 "$binary" | awk '{print $1}')"
binary_size="$(stat -f '%z' "$binary")"
notary_log_sha256="$(shasum -a 256 "$notary_log" | awk '{print $1}')"

jq -n \
  --arg schema "canisend.code-signing-evidence/v1" \
  --arg version "$version" \
  --arg target "$target" \
  --arg file "$(basename "$binary")" \
  --arg binary_sha256 "$binary_sha256" \
  --argjson binary_size "$binary_size" \
  --arg identity "$authority" \
  --arg team_id "$team_id" \
  --arg submission_id "$notary_id" \
  --arg notary_log_sha256 "$notary_log_sha256" \
  --argjson notary_warning_count "$notary_warning_count" \
  '{
    schema: $schema,
    version: $version,
    target: $target,
    kind: "apple-developer-id-notarization",
    status: "verified",
    binary: {file: $file, sha256: $binary_sha256, size: $binary_size},
    archive: null,
    signer: {
      identity: $identity,
      team_id: $team_id,
      code_identifier: "io.github.jxpeng98.canisend"
    },
    verification: {
      developer_id: true,
      hardened_runtime: true,
      secure_timestamp: true,
      notarization_status: "Accepted",
      notary_submission_id: $submission_id,
      notary_log_sha256: $notary_log_sha256,
      notary_error_count: 0,
      notary_warning_count: $notary_warning_count,
      standalone_ticket_stapled: false,
      stapling_supported: false
    }
  }' > "$evidence"

echo "macOS signing: Developer ID signature and notarization verified for $target"
