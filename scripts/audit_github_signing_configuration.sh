#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 1 ]]; then
  echo "usage: $0 [OWNER/REPOSITORY]" >&2
  exit 2
fi
if ! command -v gh >/dev/null 2>&1; then
  echo "signing configuration audit: GitHub CLI is required" >&2
  exit 1
fi

repository="${1:-}"
if [[ -z "$repository" ]]; then
  repository="$(gh repo view --json nameWithOwner --jq '.nameWithOwner')"
fi
if [[ ! "$repository" =~ ^[^/[:space:]]+/[^/[:space:]]+$ ]]; then
  echo "signing configuration audit: repository must be OWNER/REPOSITORY" >&2
  exit 2
fi

secret_names="$(gh secret list --repo "$repository" --app actions --json name --jq '.[].name')"
variable_names="$(gh variable list --repo "$repository" --json name --jq '.[].name')"
required_secrets=(
  APPLE_DEVELOPER_ID_P12_BASE64
  APPLE_DEVELOPER_ID_P12_PASSWORD
  APPLE_NOTARY_KEY_P8_BASE64
)
required_variables=(
  APPLE_SIGNING_IDENTITY
  APPLE_TEAM_ID
  APPLE_NOTARY_KEY_ID
  APPLE_NOTARY_ISSUER_ID
  AZURE_CLIENT_ID
  AZURE_TENANT_ID
  AZURE_SUBSCRIPTION_ID
  AZURE_ARTIFACT_SIGNING_ENDPOINT
  AZURE_ARTIFACT_SIGNING_ACCOUNT
  AZURE_ARTIFACT_SIGNING_PROFILE
  WINDOWS_SIGNING_EXPECTED_SUBJECT
)

missing=0
for name in "${required_secrets[@]}"; do
  if ! grep -Fxq "$name" <<<"$secret_names"; then
    echo "signing configuration audit: missing Actions secret name: $name" >&2
    missing=$((missing + 1))
  fi
done
for name in "${required_variables[@]}"; do
  if ! grep -Fxq "$name" <<<"$variable_names"; then
    echo "signing configuration audit: missing Actions variable name: $name" >&2
    missing=$((missing + 1))
  fi
done
if (( missing > 0 )); then
  echo "signing configuration audit: $missing required names are missing for $repository" >&2
  exit 1
fi

echo "signing configuration audit: all 14 required names exist for $repository; values were not read"
