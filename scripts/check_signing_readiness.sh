#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <alpha|beta|rc|stable>" >&2
  exit 2
fi

policy_path="${CANISEND_SIGNING_POLICY:-release/signing-policy.json}"
if [[ ! -f "$policy_path" ]]; then
  echo "signing readiness: policy is missing: $policy_path" >&2
  exit 1
fi

stage="$1"
case "$stage" in
  alpha)
    echo "signing readiness: unsigned Alpha is permitted by $policy_path"
    exit 0
    ;;
  beta|rc|stable) ;;
  *)
    echo "signing readiness: unknown release stage: $stage" >&2
    exit 2
    ;;
esac

required=(
  APPLE_DEVELOPER_ID_P12_BASE64
  APPLE_DEVELOPER_ID_P12_PASSWORD
  APPLE_NOTARY_KEY_P8_BASE64
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
missing=()
for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    missing+=("$name")
  fi
done
if (( ${#missing[@]} > 0 )); then
  printf 'signing readiness: missing required configuration: %s\n' "${missing[*]}" >&2
  exit 1
fi
for name in \
  APPLE_SIGNING_IDENTITY \
  AZURE_ARTIFACT_SIGNING_ACCOUNT \
  AZURE_ARTIFACT_SIGNING_PROFILE \
  WINDOWS_SIGNING_EXPECTED_SUBJECT
do
  value="${!name}"
  if (( ${#value} > 512 )) || [[ "$value" == *$'\n'* || "$value" == *$'\r'* ]]; then
    echo "signing readiness: $name is too long or contains a line break" >&2
    exit 1
  fi
done
if [[ "$APPLE_SIGNING_IDENTITY" != "Developer ID Application:"* ]]; then
  echo "signing readiness: APPLE_SIGNING_IDENTITY must be a Developer ID Application identity" >&2
  exit 1
fi
if [[ ! "$APPLE_TEAM_ID" =~ ^[A-Z0-9]{10}$ ]]; then
  echo "signing readiness: APPLE_TEAM_ID must be 10 uppercase alphanumeric characters" >&2
  exit 1
fi
if [[ ! "$APPLE_NOTARY_KEY_ID" =~ ^[A-Z0-9]{10,}$ ]]; then
  echo "signing readiness: APPLE_NOTARY_KEY_ID must be at least 10 uppercase alphanumeric characters" >&2
  exit 1
fi
uuid_pattern='^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$'
if [[ ! "$APPLE_NOTARY_ISSUER_ID" =~ $uuid_pattern ]]; then
  echo "signing readiness: APPLE_NOTARY_ISSUER_ID must be a UUID" >&2
  exit 1
fi
for name in AZURE_CLIENT_ID AZURE_TENANT_ID AZURE_SUBSCRIPTION_ID; do
  if [[ ! "${!name}" =~ $uuid_pattern ]]; then
    echo "signing readiness: $name must be a UUID" >&2
    exit 1
  fi
done
if [[ ! "$AZURE_ARTIFACT_SIGNING_ENDPOINT" =~ ^https://[a-z0-9-]+\.codesigning\.azure\.net/$ ]]; then
  echo "signing readiness: AZURE_ARTIFACT_SIGNING_ENDPOINT must be a regional HTTPS endpoint" >&2
  exit 1
fi

echo "signing readiness: configuration present for Apple notarization and Windows Artifact Signing"
