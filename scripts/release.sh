#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

DRY_RUN="${CANISEND_RELEASE_DRY_RUN:-0}"

usage() {
  cat <<'EOF'
Usage: scripts/release.sh {test|beta|stable} [options]

Run local release checks, create a release tag, and push it to trigger GitHub Actions.

Options:
  --version VERSION        Version to release. Defaults to pyproject.toml.
  --skip-local-checks     Skip local pytest/build/package checks.
  -h, --help              Show this help text.
EOF
}

die() {
  printf '%s\n' "$*" >&2
  exit 1
}

print_command() {
  printf '+'
  local arg
  for arg in "$@"; do
    printf ' %s' "$arg"
  done
  printf '\n'
}

run() {
  print_command "$@"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi
  "$@"
}

ensure_clean_worktree() {
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi

  local status
  status="$(git status --porcelain)"
  [[ -z "$status" ]] || die "Working tree must be clean before release version bump."
}

read_project_version() {
  local pyproject_version package_version
  pyproject_version="$(
    sed -nE 's/^version = "([^"]+)"/\1/p' pyproject.toml | head -n 1
  )"
  package_version="$(
    sed -nE 's/^__version__ = "([^"]+)"/\1/p' src/canisend/__init__.py | head -n 1
  )"

  [[ -n "$pyproject_version" ]] || die "pyproject.toml does not define project.version"
  [[ -n "$package_version" ]] || die "src/canisend/__init__.py does not define __version__"
  [[ "$pyproject_version" == "$package_version" ]] || die "Version mismatch: pyproject.toml=$pyproject_version, __init__.py=$package_version"

  printf '%s\n' "$pyproject_version"
}

update_version_files() {
  local version="$1"

  if [[ "$DRY_RUN" == "1" ]]; then
    print_command sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"$version\"/" pyproject.toml
    print_command sed -i.bak -E "s/^__version__ = \"[^\"]+\"/__version__ = \"$version\"/" src/canisend/__init__.py
    if [[ -f README.md ]]; then
      print_command sed -i.bak -E "s/TestPyPI-[0-9][0-9A-Za-z.]*-blue/TestPyPI-$version-blue/g" README.md
      print_command sed -i.bak -E "s/canisend==[0-9][0-9A-Za-z.]*/canisend==$version/g" README.md
    fi
    if [[ -f .codex-plugin/plugin.json ]]; then
      print_command sed -i.bak -E "s/\"version\": \"[^\"]+\"/\"version\": \"$version\"/" .codex-plugin/plugin.json
    fi
    return 0
  fi

  sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"$version\"/" pyproject.toml
  sed -i.bak -E "s/^__version__ = \"[^\"]+\"/__version__ = \"$version\"/" src/canisend/__init__.py
  if [[ -f README.md ]]; then
    sed -i.bak -E "s/TestPyPI-[0-9][0-9A-Za-z.]*-blue/TestPyPI-$version-blue/g" README.md
    sed -i.bak -E "s/canisend==[0-9][0-9A-Za-z.]*/canisend==$version/g" README.md
  fi
  if [[ -f .codex-plugin/plugin.json ]]; then
    sed -i.bak -E "s/\"version\": \"[^\"]+\"/\"version\": \"$version\"/" .codex-plugin/plugin.json
  fi
  rm -f pyproject.toml.bak src/canisend/__init__.py.bak README.md.bak .codex-plugin/plugin.json.bak
}

refresh_lock_file() {
  if [[ ! -f uv.lock ]]; then
    return 0
  fi

  run uv lock
}

add_unique_bump_file() {
  local file="$1"
  local existing
  [[ -n "$file" ]] || return 0

  for existing in "${bump_files[@]}"; do
    if [[ "$existing" == "$file" ]]; then
      return 0
    fi
  done

  bump_files+=("$file")
}

add_changed_tracked_files_to_bump_list() {
  local file

  while IFS= read -r -d '' file; do
    add_unique_bump_file "$file"
  done < <(git diff --name-only -z)

  while IFS= read -r -d '' file; do
    add_unique_bump_file "$file"
  done < <(git diff --cached --name-only -z)
}

commit_version_bump() {
  local version="$1"
  local bump_files=(pyproject.toml src/canisend/__init__.py)
  if [[ -f uv.lock ]]; then
    bump_files+=(uv.lock)
  fi
  if [[ -f README.md ]]; then
    bump_files+=(README.md)
  fi
  if [[ -f .codex-plugin/plugin.json ]]; then
    bump_files+=(.codex-plugin/plugin.json)
  fi

  if [[ "$DRY_RUN" == "1" ]]; then
    print_command git add -- "${bump_files[@]}"
    print_command git commit -m "chore: bump version to $version"
    return 0
  fi

  add_changed_tracked_files_to_bump_list

  if git diff --quiet -- "${bump_files[@]}" && git diff --cached --quiet -- "${bump_files[@]}"; then
    return 0
  fi

  run git add -- "${bump_files[@]}"
  run git commit -m "chore: bump version to $version"
}

is_prerelease() {
  [[ "$1" =~ (a|b|rc)[0-9]+$ ]]
}

validate_version_for_channel() {
  local channel="$1"
  local version="$2"

  if [[ "$channel" == "beta" ]] && ! is_prerelease "$version"; then
    die "beta releases require a PEP 440 prerelease version such as 0.2.0b1 or 0.2.0rc1"
  fi

  if [[ "$channel" == "stable" ]] && is_prerelease "$version"; then
    die "stable releases require a final version such as 0.2.0, not 0.2.0b1 or 0.2.0rc1"
  fi
}

local_release_checks() {
  local version
  version="$(read_project_version)"

  run uv run python -m pytest -v
  run uv build

  local distributions=(dist/canisend-"$version"*)
  local wheels=(dist/canisend-"$version"-*.whl)

  ((${#distributions[@]} > 0)) || die "uv build did not create any distributions under dist/"
  ((${#wheels[@]} > 0)) || die "uv build did not create a wheel under dist/"

  run uvx twine check "${distributions[@]}"
  run uv run python -m canisend.package_check "${wheels[@]}"
}

tag_name_for_channel() {
  local channel="$1"
  local version="$2"

  if [[ "$channel" == "test" ]]; then
    printf 'test/v%s\n' "$version"
  else
    printf 'v%s\n' "$version"
  fi
}

tag_message_for_channel() {
  local channel="$1"
  local version="$2"

  case "$channel" in
    test) printf 'CanISend %s TestPyPI\n' "$version" ;;
    beta) printf 'CanISend %s beta\n' "$version" ;;
    stable) printf 'CanISend %s stable\n' "$version" ;;
  esac
}

create_and_push_tag() {
  local tag="$1"
  local message="$2"

  if [[ "$DRY_RUN" == "1" ]]; then
    run git tag -a "$tag" HEAD -m "$message"
    run git push origin "$tag"
    return 0
  fi

  run git tag -a "$tag" HEAD -m "$message"
  run git push origin "$tag"
}

release_branch() {
  if [[ "$DRY_RUN" == "1" ]]; then
    printf '%s\n' "${CANISEND_RELEASE_DRY_RUN_BRANCH:-main}"
    return 0
  fi

  local branch
  branch="$(git branch --show-current)"
  [[ -n "$branch" ]] || die "Cannot release from a detached HEAD."
  printf '%s\n' "$branch"
}

ensure_tag_available() {
  local tag="$1"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi

  if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    die "Tag already exists locally: $tag"
  fi
  if git ls-remote --exit-code --tags origin "$tag" >/dev/null 2>&1; then
    die "Tag already exists on origin: $tag"
  fi
}

ensure_release_branch_policy() {
  local channel="$1"
  local branch="$2"

  if [[ "$channel" == "stable" && "$branch" != "main" ]]; then
    die "stable releases must start from main; current branch is $branch"
  fi
  if [[ "$channel" != "stable" && "$branch" != "main" ]]; then
    printf 'Prerelease channel %s permits a non-main source branch after review: %s\n' "$channel" "$branch"
  fi
  if [[ "$channel" != "stable" || "$DRY_RUN" == "1" ]]; then
    return 0
  fi

  run git fetch origin main
  git rev-parse --verify refs/remotes/origin/main >/dev/null 2>&1 \
    || die "origin/main is unavailable; push or fetch the reviewed stable source first."
  git merge-base --is-ancestor HEAD refs/remotes/origin/main \
    || die "stable release source is not reachable from origin/main."
}

push_and_verify_candidate() {
  local branch="$1"
  local channel="$2"

  run git push origin "$branch"
  if [[ "$DRY_RUN" == "1" ]]; then
    return 0
  fi

  local local_head remote_head
  local_head="$(git rev-parse HEAD)"
  remote_head="$(
    git ls-remote --exit-code --heads origin "refs/heads/$branch" | awk 'NR == 1 { print $1 }'
  )"
  [[ -n "$remote_head" && "$remote_head" == "$local_head" ]] \
    || die "Candidate commit is not the current origin/$branch head."
  printf 'Verified candidate commit on origin/%s: %s\n' "$branch" "$local_head"

  if [[ "$channel" == "stable" ]]; then
    run git fetch origin main
    git merge-base --is-ancestor HEAD refs/remotes/origin/main \
      || die "stable release candidate is not reachable from origin/main."
  fi
}

main() {
  if [[ $# -eq 0 ]]; then
    usage
    exit 2
  fi

  if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    return 0
  fi

  local channel="$1"
  shift

  case "$channel" in
    test|beta|stable) ;;
    *) die "Unknown release channel: $channel" ;;
  esac

  local version=""
  local skip_local_checks=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version)
        shift
        [[ $# -gt 0 ]] || die "--version requires a value"
        version="$1"
        ;;
      --skip-local-checks)
        skip_local_checks=1
        ;;
      -h|--help)
        usage
        return 0
        ;;
      *)
        die "Unknown argument: $1"
        ;;
    esac
    shift
  done

  if [[ -z "$version" ]]; then
    version="$(read_project_version)"
  fi

  validate_version_for_channel "$channel" "$version"

  local tag message branch
  tag="$(tag_name_for_channel "$channel" "$version")"
  message="$(tag_message_for_channel "$channel" "$version")"
  branch="$(release_branch)"
  ensure_tag_available "$tag"
  ensure_release_branch_policy "$channel" "$branch"

  ensure_clean_worktree
  update_version_files "$version"
  refresh_lock_file

  if [[ "$DRY_RUN" != "1" ]]; then
    local project_version
    project_version="$(read_project_version)"
    [[ "$version" == "$project_version" ]] || die "--version $version does not match project version $project_version"
  fi

  if [[ "$skip_local_checks" == "0" ]]; then
    local_release_checks
  fi

  commit_version_bump "$version"
  push_and_verify_candidate "$branch" "$channel"
  create_and_push_tag "$tag" "$message"

  printf 'Pushed %s. GitHub Actions release.yml will publish TestPyPI first and then promote eligible release tags.\n' "$tag"
}

main "$@"
