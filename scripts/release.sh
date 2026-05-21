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
  --ref REF               Git ref or commit to tag. Defaults to HEAD.
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
  run uv run pytest -v
  run uv build

  local distributions=(dist/*)
  local wheels=(dist/*.whl)

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
  local ref="$2"
  local message="$3"

  if [[ "$DRY_RUN" != "1" ]] && git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    die "Tag already exists locally: $tag"
  fi

  run git tag -a "$tag" "$ref" -m "$message"
  run git push origin "$tag"
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
  local ref="HEAD"
  local skip_local_checks=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version)
        shift
        [[ $# -gt 0 ]] || die "--version requires a value"
        version="$1"
        ;;
      --ref)
        shift
        [[ $# -gt 0 ]] || die "--ref requires a value"
        ref="$1"
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

  if [[ "$DRY_RUN" != "1" ]]; then
    local project_version
    project_version="$(read_project_version)"
    [[ "$version" == "$project_version" ]] || die "--version $version does not match project version $project_version"
  fi

  if [[ "$skip_local_checks" == "0" ]]; then
    local_release_checks
  fi

  local tag message
  tag="$(tag_name_for_channel "$channel" "$version")"
  message="$(tag_message_for_channel "$channel" "$version")"
  create_and_push_tag "$tag" "$ref" "$message"

  printf 'Pushed %s. GitHub Actions release.yml will publish TestPyPI first and then promote eligible release tags.\n' "$tag"
}

main "$@"
