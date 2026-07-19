#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: $0 FROM_CANDIDATE TO_CANDIDATE FROM_TAG TO_TAG TARGET ENVIRONMENT OUTPUT" >&2
  exit 2
fi

from_candidate="$1"
to_candidate="$2"
from_tag="$3"
to_tag="$4"
target="$5"
environment="$6"
output="$7"

case "$target:$environment:$(uname -m)" in
  aarch64-apple-darwin:macos-15:arm64)
    record="homebrew-aarch64-apple-darwin"
    ;;
  x86_64-apple-darwin:macos-15-intel:x86_64)
    record="homebrew-x86_64-apple-darwin"
    ;;
  *)
    echo "Homebrew qualification environment does not match $target/$environment" >&2
    exit 1
    ;;
esac

: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
command -v brew >/dev/null
command -v jq >/dev/null
command -v shasum >/dev/null

from_version="${from_tag#v}"
to_version="${to_tag#v}"
from_source="$from_candidate/candidate-source.json"
to_source="$to_candidate/candidate-source.json"
from_cask="$from_candidate/homebrew/Casks/canisend.rb"
to_cask="$to_candidate/homebrew/Casks/canisend.rb"

jq -e --arg tag "$from_tag" --arg version "$from_version" '
  .candidate_only == true and
  .publication_authorized == false and
  .release.stage == "beta" and
  .release.tag == $tag and
  .release.version == $version
' "$from_source" >/dev/null
jq -e --arg tag "$to_tag" --arg version "$to_version" '
  .candidate_only == true and
  .publication_authorized == false and
  .release.stage == "rc" and
  .release.tag == $tag and
  .release.version == $version
' "$to_source" >/dev/null
test -f "$from_cask"
test -f "$to_cask"

if brew list --cask canisend >/dev/null 2>&1; then
  echo "CanISend is already installed through Homebrew on this runner" >&2
  exit 1
fi

workspace="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/canisend-homebrew-workspace.XXXXXX")"
installed=false
tap="canisend/qualification"
tap_created=false
cleanup() {
  if [[ "$installed" == true ]] || brew list --cask canisend >/dev/null 2>&1; then
    brew uninstall --cask canisend >/dev/null 2>&1 || true
  fi
  if [[ "$tap_created" == true ]]; then
    brew untap "$tap" >/dev/null 2>&1 || true
  fi
  rm -rf "$workspace"
}
trap cleanup EXIT

export HOMEBREW_NO_AUTO_UPDATE=1
export HOMEBREW_NO_INSTALL_FROM_API=1
if brew tap | grep -Fqx "$tap"; then
  echo "Temporary qualification tap already exists: $tap" >&2
  exit 1
fi
brew tap-new --no-git "$tap"
tap_created=true
tap_root="$(brew --repo "$tap")"
tap_cask="$tap_root/Casks/canisend.rb"

brew style "$from_cask"
brew style "$to_cask"
cp "$from_cask" "$tap_cask"
brew audit --strict --cask "$tap/canisend"

brew install --cask "$tap/canisend"
installed=true
hash -r
from_observed="$(canisend version --json | jq -er '.data.version')"
test "$from_observed" = "$from_version"
canisend doctor --json | jq -e '
  .ok == true and
  .data.python_required == false and
  .data.embedded_typst == "verified" and
  .data.resource_manifest == "verified"
' >/dev/null
canisend --workspace "$workspace" workspace init --json | jq -e '.ok == true' >/dev/null
canisend --workspace "$workspace" workspace check --json | jq -e '.ok == true' >/dev/null
test -f "$workspace/canisend.toml"
test -d "$workspace/.canisend"

cp "$to_cask" "$tap_cask"
brew audit --strict --cask "$tap/canisend"
brew upgrade --cask "$tap/canisend"
hash -r
to_observed="$(canisend version --json | jq -er '.data.version')"
test "$to_observed" = "$to_version"
canisend doctor --json | jq -e '.ok == true and .data.resource_manifest == "verified"' >/dev/null

brew uninstall --cask canisend
installed=false
if brew list --cask canisend >/dev/null 2>&1; then
  echo "Homebrew still reports CanISend installed after uninstall" >&2
  exit 1
fi
test -f "$workspace/canisend.toml"
test -d "$workspace/.canisend"

from_digest="$(shasum -a 256 "$from_source" | awk '{print $1}')"
to_digest="$(shasum -a 256 "$to_source" | awk '{print $1}')"
tool_version="$(brew --version | head -n 1)"
completed_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
mkdir -p "$(dirname "$output")"
jq -n \
  --arg record "$record" \
  --arg target "$target" \
  --arg environment "$environment" \
  --arg from_tag "$from_tag" \
  --arg to_tag "$to_tag" \
  --arg from_digest "$from_digest" \
  --arg to_digest "$to_digest" \
  --arg tool_version "$tool_version" \
  --arg from_version "$from_observed" \
  --arg to_version "$to_observed" \
  --arg completed_at "$completed_at" \
  --argjson run_id "$GITHUB_RUN_ID" '
  {
    schema: "canisend.package-manager-qualification/v1",
    record: $record,
    channel: "homebrew-cask",
    target: $target,
    environment: $environment,
    from_tag: $from_tag,
    to_tag: $to_tag,
    from_candidate_source_sha256: $from_digest,
    to_candidate_source_sha256: $to_digest,
    github_run_id: $run_id,
    tool_version: $tool_version,
    observed_versions: {from: $from_version, to: $to_version},
    checks: {
      "candidate-sources-verified": true,
      "official-validation": true,
      install: true,
      "from-version": true,
      "from-doctor": true,
      "workspace-created": true,
      upgrade: true,
      "to-version": true,
      "to-doctor": true,
      uninstall: true,
      "workspace-retained": true,
      "no-publication": true
    },
    completed_at: $completed_at
  }
' >"$output"

echo "Homebrew package qualification: wrote $output"
