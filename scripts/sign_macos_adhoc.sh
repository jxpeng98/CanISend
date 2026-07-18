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
    echo "macOS ad-hoc signing: unsupported target: $target" >&2
    exit 2
    ;;
esac
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS ad-hoc signing: this script must run on macOS" >&2
  exit 1
fi
if [[ ! -f "$binary" || -L "$binary" ]]; then
  echo "macOS ad-hoc signing: binary must be a regular non-symlink file: $binary" >&2
  exit 1
fi
if [[ -e "$evidence" ]]; then
  echo "macOS ad-hoc signing: evidence destination must not exist: $evidence" >&2
  exit 1
fi
for command in codesign jq shasum stat; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS ad-hoc signing: required command is missing: $command" >&2
    exit 1
  fi
done

work="$(mktemp -d "${RUNNER_TEMP:-/tmp}/canisend-macos-adhoc.XXXXXX")"
details="$work/codesign-details.txt"
cleanup() {
  rm -rf "$work"
}
trap cleanup EXIT

codesign \
  --force \
  --identifier io.github.jxpeng98.canisend \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$binary"
codesign --verify --deep --strict --verbose=4 "$binary"
codesign --display --verbose=4 "$binary" > "$details" 2>&1

identifier="$(sed -n 's/^Identifier=//p' "$details" | head -n 1)"
signature="$(sed -n 's/^Signature=//p' "$details" | head -n 1)"
team_id="$(sed -n 's/^TeamIdentifier=//p' "$details" | head -n 1)"
timestamp="$(sed -n 's/^Timestamp=//p' "$details" | head -n 1)"
if [[ "$identifier" != "io.github.jxpeng98.canisend" ]]; then
  echo "macOS ad-hoc signing: code-signing identifier is invalid" >&2
  exit 1
fi
if [[ "$signature" != "adhoc" ]]; then
  echo "macOS ad-hoc signing: signature is not ad-hoc" >&2
  exit 1
fi
if [[ -n "$team_id" && "$team_id" != "not set" ]]; then
  echo "macOS ad-hoc signing: an unexpected TeamIdentifier is present" >&2
  exit 1
fi
if grep -q '^Authority=' "$details"; then
  echo "macOS ad-hoc signing: an unexpected certificate authority is present" >&2
  exit 1
fi
if [[ -n "$timestamp" && "$timestamp" != "none" ]]; then
  echo "macOS ad-hoc signing: an unexpected timestamp is present" >&2
  exit 1
fi
if ! grep -Eq 'flags=.*\([^)]*runtime' "$details"; then
  echo "macOS ad-hoc signing: hardened runtime flag is missing" >&2
  exit 1
fi
if codesign --display --entitlements :- "$binary" 2>&1 \
  | grep -q 'com.apple.security.get-task-allow'; then
  echo "macOS ad-hoc signing: forbidden get-task-allow entitlement is present" >&2
  exit 1
fi

version_json="$("$binary" version --json)"
version="$(printf '%s' "$version_json" | jq -er '.data.version')"
binary_sha256="$(shasum -a 256 "$binary" | awk '{print $1}')"
binary_size="$(stat -f '%z' "$binary")"
mkdir -p "$(dirname "$evidence")"

jq -n \
  --arg version "$version" \
  --arg target "$target" \
  --arg file "$(basename "$binary")" \
  --arg binary_sha256 "$binary_sha256" \
  --argjson binary_size "$binary_size" \
  '{
    schema: "canisend.code-signing-evidence/v2",
    version: $version,
    target: $target,
    kind: "apple-adhoc",
    status: "verified",
    binary: {file: $file, sha256: $binary_sha256, size: $binary_size},
    archive: null,
    signer: {
      identity: "adhoc",
      code_identifier: "io.github.jxpeng98.canisend"
    },
    verification: {
      codesign_valid: true,
      adhoc: true,
      developer_id: false,
      hardened_runtime: true,
      secure_timestamp: false,
      notarized: false,
      gatekeeper_trusted_publisher: false,
      get_task_allow: false
    }
  }' > "$evidence"

echo "macOS ad-hoc signing: integrity signature verified for $target (not Developer ID signed or notarized)"
