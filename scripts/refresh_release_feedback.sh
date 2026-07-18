#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 OWNER/REPOSITORY RC_TAG [--write]" >&2
  exit 2
fi
repository="$1"
tag="$2"
mode="${3:-}"
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "release feedback refresh: repository must use OWNER/REPOSITORY syntax" >&2
  exit 2
fi
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-rc\.[1-9][0-9]*$ ]]; then
  echo "release feedback refresh: tag must identify a release candidate" >&2
  exit 2
fi
if [[ -n "$mode" && "$mode" != "--write" ]]; then
  echo "usage: $0 OWNER/REPOSITORY RC_TAG [--write]" >&2
  exit 2
fi
for command_name in gh jq cargo git awk; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "release feedback refresh requires $command_name" >&2
    exit 1
  fi
done

root="$(git rev-parse --show-toplevel)"
snapshot="$root/release/feedback-snapshot.json"
roadmap="$root/docs/superpowers/plans/2026-07-18-post-0.7-roadmap.md"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/canisend-release-feedback.XXXXXX")"
trap 'rm -rf -- "$temporary"' EXIT

gh api --paginate --slurp "repos/$repository/issues?state=all&per_page=100" \
  | jq '[.[][] | select(has("pull_request") | not) | {number, state}] | sort_by(.number)' \
  >"$temporary/issues.json"
gh release view "$tag" --repo "$repository" \
  --json tagName,isDraft,isPrerelease,publishedAt,assets \
  >"$temporary/release.json"
jq -e \
  --arg tag "$tag" \
  '.tagName == $tag and (.isDraft | not) and .isPrerelease and (.publishedAt | endswith("Z"))' \
  "$temporary/release.json" >/dev/null

open_count="$(jq '[.[] | select(.state == "open")] | length' "$temporary/issues.json")"
closed_count="$(jq '[.[] | select(.state == "closed")] | length' "$temporary/issues.json")"
total_count="$(jq 'length' "$temporary/issues.json")"
issue_numbers="$(jq '[.[].number]' "$temporary/issues.json")"
version="${tag#v}"
native_names="$(jq -n --arg version "$version" '[
  "canisend-\($version)-aarch64-apple-darwin.tar.gz",
  "canisend-\($version)-x86_64-apple-darwin.tar.gz",
  "canisend-\($version)-x86_64-unknown-linux-gnu.tar.gz",
  "canisend-\($version)-x86_64-unknown-linux-musl.tar.gz",
  "canisend-\($version)-x86_64-pc-windows-msvc.zip"
]')"
asset_count="$(jq '.assets | length' "$temporary/release.json")"
total_downloads="$(jq '[.assets[].downloadCount] | add // 0' "$temporary/release.json")"
native_archive_count="$(jq --argjson names "$native_names" '[.assets[] | select(.name as $name | $names | index($name))] | length' "$temporary/release.json")"
native_archive_downloads="$(jq --argjson names "$native_names" '[.assets[] | select(.name as $name | $names | index($name)) | .downloadCount] | add // 0' "$temporary/release.json")"
if [[ "$native_archive_count" -ne 5 ]]; then
  echo "release feedback refresh: RC release must contain all five native archives" >&2
  exit 1
fi

captured_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
published_at="$(jq -r '.publishedAt' "$temporary/release.json")"
jq \
  --arg captured_at "$captured_at" \
  --arg repository "$repository" \
  --arg tag "$tag" \
  --arg published_at "$published_at" \
  --argjson issue_numbers "$issue_numbers" \
  --argjson open_count "$open_count" \
  --argjson closed_count "$closed_count" \
  --argjson total_count "$total_count" \
  --argjson asset_count "$asset_count" \
  --argjson total_downloads "$total_downloads" \
  --argjson native_archive_count "$native_archive_count" \
  --argjson native_archive_downloads "$native_archive_downloads" '
  .captured_at = $captured_at
  | .next_roadmap.status = "reviewed"
  | .public_feedback = {
      closed_issue_count: $closed_count,
      issue_numbers: $issue_numbers,
      open_issue_count: $open_count,
      total_issue_count: $total_count
    }
  | .release = {
      published_at: $published_at,
      repository: $repository,
      tag: $tag
    }
  | .release_downloads = {
      asset_count: $asset_count,
      maintainer_verification_included: true,
      native_archive_count: $native_archive_count,
      native_archive_downloads: $native_archive_downloads,
      total_downloads: $total_downloads
    }
  | .snapshot_stage = "rc"
' "$snapshot" >"$temporary/feedback-snapshot.json"

{
  printf 'Snapshot stage: `rc`; captured at `%s`; public GitHub issues: **%s** open, **%s** closed, **%s** total.' \
    "$captured_at" "$open_count" "$closed_count" "$total_count"
  if [[ "$total_count" -eq 0 ]]; then
    printf ' No public user issue was present at capture time.'
  fi
  printf '\n\n'
  printf 'Release: `%s`; public assets: **%s**; downloads: **%s** total, **%s** across **%s** native archives. ' \
    "$tag" "$asset_count" "$total_downloads" "$native_archive_downloads" "$native_archive_count"
  printf '%s\n' 'Counts include maintainer verification and are not unique-user, adoption, retention, or platform-demand measurements.'
  printf '%s\n' 'CanISend has no default telemetry and this roadmap does not infer private use from workspace data.'
} >"$temporary/measured-roadmap.md"

awk '
  NR == FNR {
    measured = measured $0 ORS
    next
  }
  !status_replaced && $0 == "**Status:** Draft" {
    print "**Status:** Reviewed"
    status_replaced = 1
    next
  }
  $0 == "<!-- release-feedback-measured:start -->" {
    print
    printf "%s", measured
    in_measured = 1
    measured_replaced = 1
    next
  }
  $0 == "<!-- release-feedback-measured:end -->" {
    in_measured = 0
    print
    next
  }
  !in_measured { print }
  END {
    if (!status_replaced || !measured_replaced || in_measured) exit 1
  }
' "$temporary/measured-roadmap.md" "$roadmap" >"$temporary/post-0.7-roadmap.md"

cargo run -p xtask --locked -- release verify-feedback-candidate \
  "$temporary/feedback-snapshot.json" \
  "$temporary/post-0.7-roadmap.md" >/dev/null

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

if [[ "$mode" == "--write" ]]; then
  if [[ -n "$(git -C "$root" status --porcelain --untracked-files=all)" ]]; then
    echo "release feedback write requires a clean worktree" >&2
    exit 1
  fi
  cp "$temporary/feedback-snapshot.json" "$snapshot"
  cp "$temporary/post-0.7-roadmap.md" "$roadmap"
  echo "release feedback refreshed for $tag ($total_count public issues, $total_downloads downloads)"
else
  jq -n \
    --arg repository "$repository" \
    --arg tag "$tag" \
    --arg captured_at "$captured_at" \
    --arg snapshot_before "$(sha256_file "$snapshot")" \
    --arg snapshot_after "$(sha256_file "$temporary/feedback-snapshot.json")" \
    --arg roadmap_before "$(sha256_file "$roadmap")" \
    --arg roadmap_after "$(sha256_file "$temporary/post-0.7-roadmap.md")" \
    --argjson issue_count "$total_count" \
    --argjson download_count "$total_downloads" '
    {
      schema: "canisend.release-feedback-refresh/v1",
      mode: "dry-run",
      writes_performed: false,
      repository: $repository,
      tag: $tag,
      captured_at: $captured_at,
      public_issue_count: $issue_count,
      release_download_count: $download_count,
      privacy_boundary: "issue-number-and-state-plus-release-asset-counts",
      candidate_validated: true,
      files: [
        {path: "release/feedback-snapshot.json", before_sha256: $snapshot_before, after_sha256: $snapshot_after},
        {path: "docs/superpowers/plans/2026-07-18-post-0.7-roadmap.md", before_sha256: $roadmap_before, after_sha256: $roadmap_after}
      ]
    }
  '
fi
