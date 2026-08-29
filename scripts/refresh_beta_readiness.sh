#!/usr/bin/env bash
set -euo pipefail

repository="${1:-jxpeng98/CanISend}"
maintainer_validation="${2:-}"
mode="${3:-}"
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "repository must use OWNER/REPOSITORY syntax" >&2
  exit 2
fi
if [[ -z "$maintainer_validation" || ! -f "$maintainer_validation" || ( -n "$mode" && "$mode" != "--write" ) ]]; then
  echo "usage: $0 [OWNER/REPOSITORY] BODY_FREE_MAINTAINER_VALIDATION_JSON [--write]" >&2
  exit 2
fi

for command_name in gh jq cargo git; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "beta readiness refresh requires $command_name" >&2
    exit 1
  fi
done
sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "beta readiness refresh requires sha256sum or shasum" >&2
    exit 1
  fi
}
jq -s -e 'length == 1 and (.[0] | type == "object")' "$maintainer_validation" >/dev/null || {
  echo "Beta maintainer validation must be one JSON object" >&2
  exit 2
}

root="$(git rev-parse --show-toplevel)"
ledger="$root/release/beta-readiness.json"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/canisend-beta-readiness.XXXXXX")"
trap 'rm -rf -- "$temporary"' EXIT

gh api --paginate --slurp "repos/$repository/issues?state=all&per_page=100" \
  | jq '[.[][] | select(has("pull_request") | not) |
      {number, state, labels: [.labels[].name]}]' \
  > "$temporary/issues.json"

all_issue_count="$(jq 'length' "$temporary/issues.json")"
open_issue_count="$(jq '[.[] | select(.state == "open")] | length' "$temporary/issues.json")"
blocked_issue_numbers="$(jq -c '[.[] | select(
  .state == "open" and
  (.labels | index("priority:P0")) and
  (.labels | index("state:blocked"))
) | .number]' "$temporary/issues.json")"
if [[ "$blocked_issue_numbers" != "[]" ]]; then
  echo "beta readiness refresh stopped: applicable P0 blockers remain" >&2
  echo "$blocked_issue_numbers" >&2
  exit 1
fi

alpha_tag="$(jq -r '.alpha_release.tag' "$ledger")"
if [[ ! "$alpha_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-alpha\.([0-9]+)$ ]] \
  || (( BASH_REMATCH[1] < 7 )); then
  echo "beta readiness refresh requires a dual-Pack Alpha iteration of 7 or greater" >&2
  exit 1
fi
version="${alpha_tag#v}"
expected_url="https://github.com/$repository/releases/tag/$alpha_tag"
gh release view "$alpha_tag" --repo "$repository" \
  --json tagName,url,isDraft,isPrerelease > "$temporary/release.json"
jq -e \
  --arg tag "$alpha_tag" \
  --arg url "$expected_url" \
  '.tagName == $tag and .url == $url and (.isDraft | not) and .isPrerelease' \
  "$temporary/release.json" >/dev/null

gh release download "$alpha_tag" --repo "$repository" \
  --pattern "canisend-$version-manifest.json" --dir "$temporary"
manifest="$temporary/canisend-$version-manifest.json"
source_commit="$(jq -er --arg tag "$alpha_tag" '
  select(
    .tag == $tag and
    .contracts.agent_protocol == "canisend.agent/v4" and
    .contracts.workspace_format == "canisend.workspace/v4"
  ) | .source.commit' "$manifest")"
if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]] || ! git -C "$root" cat-file -e "$source_commit^{commit}"; then
  echo "beta readiness refresh could not verify the exact eligible Alpha source commit" >&2
  exit 1
fi
provider_record="$root/release/provider-dogfood.json"
provider_tag="$(jq -er '.candidate.tag' "$provider_record")"
provider_source="$(jq -er '.candidate.source_commit' "$provider_record")"
release_run="$(jq -er '.candidate.release_run' "$provider_record")"
if [[ ! "$release_run" =~ ^[1-9][0-9]*$ ]]; then
  echo "beta readiness refresh has no provider-qualified Alpha candidate run" >&2
  exit 1
fi
if [[ "$provider_tag" != "$alpha_tag" || "$provider_source" != "$source_commit" ]]; then
  echo "beta readiness refresh found public Alpha bytes that differ from provider qualification" >&2
  exit 1
fi
provider_sha256="$(sha256_file "$provider_record")"

jq -n \
  --slurpfile provider "$provider_record" '
  {
    agent_protocol: $provider[0].contracts.agent_protocol,
    workspace_format: $provider[0].contracts.workspace_format,
    resource_format: $provider[0].contracts.resource_format,
    task_resource_model_sha256: $provider[0].contracts.task_resource_model_sha256,
    workflow_pack_format: "canisend.workflow-pack/v1",
    workflow_packs: $provider[0].packs,
    skills: $provider[0].skills
  }' > "$temporary/contracts.json"

audited_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
jq \
  --arg audited_at "$audited_at" \
  --arg repository "$repository" \
  --arg alpha_tag "$alpha_tag" \
  --arg source_commit "$source_commit" \
  --arg release_url "$expected_url" \
  --arg provider_sha256 "$provider_sha256" \
  --argjson release_run "$release_run" \
  --slurpfile contracts "$temporary/contracts.json" \
  --slurpfile provider "$provider_record" \
  --slurpfile maintainer_validation "$maintainer_validation" \
  --argjson all_issue_count "$all_issue_count" \
  --argjson open_issue_count "$open_issue_count" \
  --argjson blocked_issue_numbers "$blocked_issue_numbers" \
  '.schema = "canisend.beta-readiness/v2"
   | .status = "qualified"
   | .audited_at = $audited_at
   | .alpha_release = {
       tag: $alpha_tag,
       source_commit: $source_commit,
       release_run: $release_run,
       release_url: $release_url
     }
   | .github_issue_snapshot = {
       query: "https://github.com/" + $repository + "/issues?q=is%3Aissue",
       all_issue_count: $all_issue_count,
       open_issue_count: $open_issue_count,
       open_p0_blocker_issue_numbers: $blocked_issue_numbers
     }
   | .contracts = $contracts[0]
   | .provider_evidence = {
       schema: $provider[0].schema,
       record_sha256: $provider_sha256,
       scenario_ids: ($provider[0].scenarios | map(.scenario_id))
     }
   | .maintainer_validation = $maintainer_validation[0]
   | .cohort_evidence = {
       status: "not-started",
       synthetic_users: 0,
       invited_users: 0,
       completed_user_flows: 0,
       starts_at: "v1.0.0-beta.1",
       required_before: "v1.0.0-rc.1"
     }
   | .blocker_classes = ([
       "data-loss",
       "privacy",
       "evidence",
       "pack",
       "rendering",
       "recovery",
       "host-setup",
       "supported-install",
       "release-integrity"
     ] | map({
       class: .,
       status: "clear",
       open_issue_numbers: [],
       evidence: [
         "alpha10-public-release-matrix",
         "alpha10-codex-provider-evidence",
         "maintainer-validation-note"
       ]
     }))
   | .unresolved_release_blockers = []
   | del(.known_limitations_reviewed, .user_evidence)' \
  "$ledger" > "$temporary/candidate.json"

cargo run -p xtask --locked -- release verify-beta-readiness \
  "$temporary/candidate.json" >/dev/null

if [[ "$mode" == "--write" ]]; then
  if [[ -n "$(git -C "$root" status --porcelain --untracked-files=all)" ]]; then
    echo "beta readiness write requires a clean worktree" >&2
    exit 1
  fi
  cp "$temporary/candidate.json" "$ledger"
  echo "beta readiness refreshed at $audited_at ($all_issue_count public issues, no applicable P0 blocker)"
else
  jq -n \
    --arg repository "$repository" \
    --arg audited_at "$audited_at" \
    --argjson all_issue_count "$all_issue_count" \
    --argjson open_issue_count "$open_issue_count" \
    '{schema: "canisend.beta-readiness-refresh/v2", mode: "dry-run", repository: $repository,
      audited_at: $audited_at, all_issue_count: $all_issue_count,
      open_issue_count: $open_issue_count, open_p0_blocker_issue_numbers: [],
      candidate_validated: true}'
  cat "$temporary/candidate.json"
fi
