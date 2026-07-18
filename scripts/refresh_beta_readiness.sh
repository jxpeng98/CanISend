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
expected_url="$(jq -r '.alpha_release.release_url' "$ledger")"
gh release view "$alpha_tag" --repo "$repository" \
  --json tagName,url,isDraft,isPrerelease > "$temporary/release.json"
jq -e \
  --arg tag "$alpha_tag" \
  --arg url "$expected_url" \
  '.tagName == $tag and .url == $url and (.isDraft | not) and .isPrerelease' \
  "$temporary/release.json" >/dev/null

audited_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
jq \
  --arg audited_at "$audited_at" \
  --argjson all_issue_count "$all_issue_count" \
  --argjson open_issue_count "$open_issue_count" \
  '.audited_at = $audited_at
   | .github_issue_snapshot.all_issue_count = $all_issue_count
   | .github_issue_snapshot.open_issue_count = $open_issue_count
   | .blocker_classes |= map(.open_issue_numbers = [])
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
