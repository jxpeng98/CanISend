#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  echo "usage: $0" >&2
  exit 2
fi

workflow=".github/workflows/release.yml"
policy="release/signing-policy.json"
for path in "$workflow" "$policy"; do
  if [[ ! -f "$path" ]]; then
    echo "community signing audit: required file is missing: $path" >&2
    exit 1
  fi
done

for forbidden in \
  APPLE_DEVELOPER_ID_P12_BASE64 \
  APPLE_NOTARY_KEY_P8_BASE64 \
  azure/artifact-signing-action \
  AZURE_ARTIFACT_SIGNING_ACCOUNT
do
  if grep -Fq "$forbidden" "$workflow" "$policy"; then
    echo "community signing audit: paid signing dependency remains: $forbidden" >&2
    exit 1
  fi
done

grep -Fq '"external_credentials_required": false' "$policy"
grep -Fq './scripts/sign_macos_adhoc.sh' "$workflow"
grep -Fq './scripts/sign_windows_self_signed.ps1' "$workflow"

echo "community signing audit: external credentials are not required; platform integrity signing remains fail-closed"
