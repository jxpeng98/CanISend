#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 TAG ASSET_NAME DESTINATION" >&2
  exit 2
fi

tag="$1"
asset_name="$2"
destination="$3"
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "draft asset download: tag must be canonical SemVer" >&2
  exit 1
fi
if [[ -z "$asset_name" || "$asset_name" == */* || "$asset_name" == "." || "$asset_name" == ".." ]]; then
  echo "draft asset download: asset name must be one safe basename" >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
destination="$(canisend_absolute_path "$destination")"
if [[ -e "$destination" || -L "$destination" ]]; then
  echo "draft asset download: destination already exists: $destination" >&2
  exit 1
fi
mkdir -p "$(dirname "$destination")"

for command in gh jq; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "draft asset download: required command is missing: $command" >&2
    exit 1
  fi
done
if command -v sha256sum >/dev/null 2>&1; then
  sha256_file() {
    sha256sum "$1" | awk '{print $1}'
  }
elif command -v shasum >/dev/null 2>&1; then
  sha256_file() {
    shasum -a 256 "$1" | awk '{print $1}'
  }
else
  echo "draft asset download: sha256sum or shasum is required" >&2
  exit 1
fi

repository="${GITHUB_REPOSITORY:-}"
if [[ -z "$repository" ]]; then
  repository="$(gh repo view --json nameWithOwner --jq '.nameWithOwner')"
fi
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "draft asset download: repository identity is invalid" >&2
  exit 1
fi

metadata="$(mktemp "${TMPDIR:-/tmp}/canisend-draft-asset-metadata.XXXXXX")"
temporary="$(mktemp "$(dirname "$destination")/.${asset_name}.download.XXXXXX")"
cleanup() {
  rm -f "$metadata" "$temporary"
}
trap cleanup EXIT

gh release view "$tag" \
  --repo "$repository" \
  --json tagName,isDraft,isPrerelease,assets \
  > "$metadata"
jq -e \
  --arg tag "$tag" \
  '.tagName == $tag and .isDraft == true and .isPrerelease == false' \
  "$metadata" >/dev/null

asset="$(
  jq -ce \
    --arg name "$asset_name" \
    '
      [.assets[] | select(.name == $name)]
      | if length == 1 then .[0] else error("draft asset must exist exactly once") end
      | select(
          .state == "uploaded"
          and (.size | type == "number" and . > 0)
          and (.digest | type == "string" and test("^sha256:[0-9a-f]{64}$"))
          and (.apiUrl | type == "string"
               and test("^https://api[.]github[.]com/repos/[^/]+/[^/]+/releases/assets/[0-9]+$"))
        )
    ' \
    "$metadata"
)"
api_url="$(jq -er '.apiUrl' <<< "$asset")"
expected_size="$(jq -er '.size' <<< "$asset")"
expected_sha256="$(jq -er '.digest | sub("^sha256:"; "")' <<< "$asset")"

gh api \
  --method GET \
  -H "Accept: application/octet-stream" \
  "$api_url" \
  > "$temporary"

actual_size="$(wc -c < "$temporary" | tr -d ' ')"
actual_sha256="$(sha256_file "$temporary")"
if [[ "$actual_size" != "$expected_size" || "$actual_sha256" != "$expected_sha256" ]]; then
  echo "draft asset download: downloaded bytes do not match GitHub metadata" >&2
  exit 1
fi
mv "$temporary" "$destination"
echo "draft asset download: verified $asset_name ($actual_sha256)"
