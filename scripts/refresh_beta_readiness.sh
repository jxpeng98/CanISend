#!/usr/bin/env bash
set -euo pipefail

repository="${1:-jxpeng98/CanISend}"
mode="${2:-}"
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "repository must use OWNER/REPOSITORY syntax" >&2
  exit 2
fi
if [[ -n "$mode" && "$mode" != "--write" ]]; then
  echo "usage: $0 [OWNER/REPOSITORY] [--write]" >&2
  exit 2
fi

for command_name in gh jq cargo git; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "beta readiness refresh requires $command_name" >&2
    exit 1
  fi
done

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
if [[ ! "$alpha_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-alpha\.7$ ]]; then
  echo "beta readiness refresh requires the dual-Pack Alpha.7 checkpoint" >&2
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
  '.tag == $tag and .contracts.agent_protocol == "canisend.agent/v3"
   and .contracts.workspace_format == "canisend.workspace/v3"
   and .source.commit' "$manifest")"
if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ || "$(git -C "$root" rev-parse HEAD)" != "$source_commit" ]]; then
  echo "beta readiness refresh requires a clean checkout of the exact Alpha.7 source commit" >&2
  exit 1
fi
release_run="$(gh api "repos/$repository/actions/workflows/release.yml/runs?head_sha=$source_commit&status=success&per_page=100" \
  --jq '[.workflow_runs[] | select(.conclusion == "success") | .id] | max // empty')"
if [[ ! "$release_run" =~ ^[1-9][0-9]*$ ]]; then
  echo "beta readiness refresh could not resolve the successful Alpha.7 release run" >&2
  exit 1
fi

academic_pack="$root/crates/canisend-resources/resources/workflow-packs/org.canisend.academic-job/manifest.json"
generic_pack="$root/crates/canisend-resources/resources/workflow-packs/org.canisend.generic-application/manifest.json"
jq -n \
  --slurpfile academic "$academic_pack" \
  --slurpfile generic "$generic_pack" '
  {
    agent_protocol: "canisend.agent/v3",
    workspace_format: "canisend.workspace/v3",
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
         "Alpha.7 release run " + ($release_run | tostring) +
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
