#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 CACHE_NAMESPACE" >&2
  exit 2
fi

namespace="$1"
: "${GITHUB_ENV:?GITHUB_ENV is required}"
: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"

if [[ ! "$namespace" =~ ^canisend-v[0-9]+-rust-[0-9]+\.[0-9]+\.[0-9]+-[A-Za-z0-9._+-]+$ ]]; then
  echo "sccache configuration: cache namespace is invalid: $namespace" >&2
  exit 1
fi

install_outcome="${CANISEND_SCCACHE_INSTALL_OUTCOME:-unknown}"
tool="${SCCACHE_PATH:-}"
if [[ "$install_outcome" == "success" ]]; then
  if [[ -z "$tool" ]] || ! "$tool" --version >/dev/null 2>&1; then
    tool="$(command -v sccache || true)"
  fi
  export SCCACHE_GHA_ENABLED=true
  export SCCACHE_GHA_VERSION="$namespace"
  export SCCACHE_IGNORE_SERVER_IO_ERROR=1
  if [[ -n "$tool" ]] \
    && "$tool" --start-server >/dev/null 2>&1 \
    && "$tool" --zero-stats >/dev/null 2>&1; then
    {
      echo "SCCACHE_PATH=$tool"
      echo "RUSTC_WRAPPER=$tool"
      echo "SCCACHE_GHA_ENABLED=true"
      echo "SCCACHE_GHA_VERSION=$namespace"
      echo "SCCACHE_IGNORE_SERVER_IO_ERROR=1"
      echo "CARGO_INCREMENTAL=0"
      echo "CANISEND_SCCACHE_ENABLED=true"
      echo "CANISEND_SCCACHE_FALLBACK_REASON=none"
      echo "CANISEND_SCCACHE_NAMESPACE=$namespace"
    } >> "$GITHUB_ENV"
    {
      echo "enabled=true"
      echo "fallback_reason=none"
    } >> "$GITHUB_OUTPUT"
    echo "sccache configuration: enabled ($namespace)"
    exit 0
  fi
  "$tool" --stop-server >/dev/null 2>&1 || true
fi

{
  echo "CANISEND_SCCACHE_ENABLED=false"
  echo "CANISEND_SCCACHE_FALLBACK_REASON=installation-or-server-unavailable"
  echo "CANISEND_SCCACHE_NAMESPACE=$namespace"
} >> "$GITHUB_ENV"
{
  echo "enabled=false"
  echo "fallback_reason=installation-or-server-unavailable"
} >> "$GITHUB_OUTPUT"
echo "sccache configuration: unavailable; continuing with ordinary Cargo compilation"
