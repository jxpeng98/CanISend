#!/usr/bin/env bash
# Verify or explicitly publish an exact candidate produced by check_macos_local.sh.
set -euo pipefail

action="${1:-}"
channel="${2:-}"
candidate="${3:-}"
if [[ ! "$action" =~ ^(verify|publish)$ || ! "$channel" =~ ^(npm|pypi)$ || -z "$candidate" ]]; then
  echo "usage: $0 <verify|publish> <npm|pypi> CANDIDATE_DIRECTORY" >&2
  exit 2
fi
if [[ "${GITHUB_ACTIONS:-false}" == true || "$(uname -s)" != Darwin || "$(uname -m)" != arm64 ]]; then
  echo "This path accepts only local Apple Silicon macOS candidates." >&2
  exit 2
fi

repo="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
candidate="$(CDPATH= cd -- "$candidate" && pwd)"
case "$candidate" in
  "$repo"/dist/local-macos-"$channel".*) ;;
  *) echo "Candidate must be an exact dist/local-macos-$channel.* directory." >&2; exit 2 ;;
esac
cd "$repo"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Publication candidates require a clean committed worktree." >&2
  exit 1
fi
revision="$(git rev-parse HEAD)"
test "$(tr -d '\n' < "$candidate/source-revision.txt")" = "$revision"
test ! -s "$candidate/source.patch"
(cd "$candidate" && shasum -a 256 -c SHA256SUMS)

version="$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -er '.packages[] | select(.name == "canisend") | .version')"
[[ "$version" == *-* ]]

if [[ "$action" == publish ]]; then
  test "$(git branch --show-current)" = main
  if [[ "${CANISEND_PUBLISH:-}" != "yes" ]]; then
    echo "Set CANISEND_PUBLISH=yes after reviewing the candidate and registry target." >&2
    exit 2
  fi
fi

case "$channel" in
  npm)
    archive="$candidate/npm/canisend-$version.tgz"
    test -f "$archive"
    test "$(tar -xOf "$archive" package/package.json | jq -er .version)" = "$version"
    if [[ "$action" == publish ]]; then
      npm publish "$archive" --access public --tag next --ignore-scripts
      expected="$(shasum "$archive" | awk '{print $1}')"
      test "$(npm view "canisend@$version" dist.shasum)" = "$expected"
      test "$(npm view canisend dist-tags.next)" = "$version"
      install_root="$(mktemp -d "$candidate/registry-install.XXXXXX")"
      npm install --ignore-scripts --no-audit --no-fund --prefix "$install_root" "canisend@$version"
      "$install_root/node_modules/.bin/canisend" version --json \
        | jq -e --arg version "$version" '.ok and .data.version == $version'
    fi
    ;;
  pypi)
    shopt -s nullglob
    wheels=("$candidate"/wheels/canisend-*.whl)
    test "${#wheels[@]}" -eq 1
    wheel="${wheels[0]}"
    twine check --strict "$wheel"
    python_version="$(unzip -p "$wheel" '*.dist-info/METADATA' \
      | awk '/^Version: / {print $2; exit}')"
    test -n "$python_version"
    if [[ "$action" == publish ]]; then
      twine upload --non-interactive "$wheel"
      expected="$(shasum -a 256 "$wheel" | awk '{print $1}')"
      actual="$(curl -fsSL "https://pypi.org/pypi/canisend/$python_version/json" \
        | jq -er --arg name "$(basename "$wheel")" '.urls[] | select(.filename == $name) | .digests.sha256')"
      test "$actual" = "$expected"
      install_root="$(mktemp -d "$candidate/registry-install.XXXXXX")"
      python3 -m venv "$install_root/venv"
      "$install_root/venv/bin/pip" install --only-binary=:all: "canisend==$python_version"
      "$install_root/venv/bin/canisend" version --json \
        | jq -e --arg version "$version" '.ok and .data.version == $version'
    fi
    ;;
esac

echo "PASS: $action $channel candidate $version at $revision"
