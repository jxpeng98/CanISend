#!/usr/bin/env bash
set -euo pipefail

repository="${1:-jxpeng98/CanISend}"
user_evidence="${2:-}"
mode="${3:-}"
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "repository must use OWNER/REPOSITORY syntax" >&2
  exit 2
fi
if [[ -z "$user_evidence" || ! -f "$user_evidence" || ( -n "$mode" && "$mode" != "--write" ) ]]; then
  echo "usage: $0 [OWNER/REPOSITORY] BODY_FREE_USER_EVIDENCE_JSON [--write]" >&2
  exit 2
fi

for command_name in gh jq cargo git; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "beta readiness refresh requires $command_name" >&2
    exit 1
  fi
done
jq -s -e 'length == 1 and (.[0] | type == "object")' "$user_evidence" >/dev/null || {
  echo "Beta user evidence must be one JSON object" >&2
  exit 2
}

root="$(git rev-parse --show-toplevel)"
ledger="$root/release/beta-readiness.json"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/canisend-beta-readiness.XXXXXX")"
trap 'rm -rf -- "$temporary"' EXIT

gh api --paginate --slurp "repos/$repository/issues?state=all&per_page=100" \
  | jq '[.[][] | select(has("pull_request") | not) | {number, state}]' \
  > "$temporary/issues.json"

all_issue_count="$(jq 'length' "$temporary/issues.json")"
open_issue_count="$(jq '[.[] | select(.state == "open")] | length' "$temporary/issues.json")"
if [[ "$open_issue_count" -ne 0 ]]; then
  echo "beta readiness refresh stopped: every open issue requires maintainer triage" >&2
  jq -c '[.[] | select(.state == "open") | .number]' "$temporary/issues.json" >&2
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
source_commit="$(jq -er --arg tag "$alpha_tag" \
  '.tag == $tag and .contracts.agent_protocol == "canisend.agent/v4"
   and .contracts.workspace_format == "canisend.workspace/v4"
   and .source.commit' "$manifest")"
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

academic_pack="$root/crates/canisend-resources/resources/workflow-packs/org.canisend.academic-job/manifest.json"
generic_pack="$root/crates/canisend-resources/resources/workflow-packs/org.canisend.generic-application/manifest.json"
jq -n \
  --slurpfile academic "$academic_pack" \
  --slurpfile generic "$generic_pack" '
  {
    agent_protocol: "canisend.agent/v4",
    workspace_format: "canisend.workspace/v4",
    workflow_pack_format: "canisend.workflow-pack/v1",
    workflow_packs: [$academic[0], $generic[0]]
      | map({id, version, content_digest})
  }' > "$temporary/contracts.json"

audited_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
jq \
  --arg audited_at "$audited_at" \
  --arg repository "$repository" \
  --arg alpha_tag "$alpha_tag" \
  --arg source_commit "$source_commit" \
  --arg release_url "$expected_url" \
  --argjson release_run "$release_run" \
  --slurpfile contracts "$temporary/contracts.json" \
  --slurpfile user_evidence "$user_evidence" \
  --argjson all_issue_count "$all_issue_count" \
  --argjson open_issue_count "$open_issue_count" \
  '.status = "qualified"
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
       open_issue_count: $open_issue_count
     }
   | .known_limitations_reviewed = true
   | .contracts = $contracts[0]
   | .user_evidence = $user_evidence[0]
   | .blocker_classes = [
       "data-loss",
       "protocol-compatibility",
       "rendering-corruption",
       "security-privacy"
     ] | .blocker_classes = map({
       class: .,
       status: "clear",
       open_issue_numbers: [],
       evidence: [
         "Public GitHub issue snapshot contains no unresolved issue",
         $alpha_tag + " release run " + ($release_run | tostring) +
           " passed the exact public dual-Pack release matrix"
       ]
     })
   | .unresolved_release_blockers = []' \
  "$ledger" > "$temporary/candidate.json"

cargo run -p xtask --locked -- release verify-beta-readiness \
  "$temporary/candidate.json" >/dev/null

if [[ "$mode" == "--write" ]]; then
  if [[ -n "$(git -C "$root" status --porcelain --untracked-files=all)" ]]; then
    echo "beta readiness write requires a clean worktree" >&2
    exit 1
  fi
  cp "$temporary/candidate.json" "$ledger"
  echo "beta readiness refreshed at $audited_at ($all_issue_count public issues, none open)"
else
  jq -n \
    --arg repository "$repository" \
    --arg audited_at "$audited_at" \
    --argjson all_issue_count "$all_issue_count" \
    '{schema: "canisend.beta-readiness-refresh/v1", mode: "dry-run", repository: $repository,
      audited_at: $audited_at, all_issue_count: $all_issue_count, open_issue_count: 0,
      candidate_validated: true}'
  cat "$temporary/candidate.json"
fi
