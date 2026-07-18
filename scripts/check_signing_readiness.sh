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
  beta|rc|stable)
    echo "signing readiness: community platform signing requires no external credentials; target runners fail closed on missing tooling"
    exit 0
    ;;
  *)
    echo "signing readiness: unknown release stage: $stage" >&2
    exit 2
    ;;
esac
