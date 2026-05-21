#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

DRY_RUN="${CANISEND_RELEASE_DRY_RUN:-0}"

usage() {
  cat <<'EOF'
Usage: scripts/release.sh {test|beta|stable} [options]

Trigger CanISend TestPyPI, beta, or stable release flows.

Options:
  --version VERSION        Version to release. Defaults to pyproject.toml.
  --ref REF               Git ref used by GitHub Actions. Defaults to main.
  --skip-local-checks     Skip local pytest/build/package checks.
  --skip-testpypi-smoke   Skip install smoke test from TestPyPI.
  --no-wait-pypi          Do not wait for PyPI publish after GitHub Release.
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

capture() {
  print_command "$@" >&2
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

latest_test_run_id() {
  local ref="$1"

  if [[ "$DRY_RUN" == "1" ]]; then
    printf '%s\n' "${CANISEND_RELEASE_FAKE_TEST_RUN_ID:-123456}"
    return 0
  fi

  local run_id
  run_id="$(
    capture gh run list \
      --workflow release.yml \
      --event workflow_dispatch \
      --branch "$ref" \
      --json databaseId,status,conclusion,createdAt \
      --limit 1 \
      --jq '.[0].databaseId'
  )"
  [[ -n "$run_id" && "$run_id" != "null" ]] || die "No workflow_dispatch release run found after triggering TestPyPI"
  printf '%s\n' "$run_id"
}

latest_pypi_release_run_id() {
  if [[ "$DRY_RUN" == "1" ]]; then
    printf '%s\n' "${CANISEND_RELEASE_FAKE_PYPI_RUN_ID:-654321}"
    return 0
  fi

  local run_id
  run_id="$(
    capture gh run list \
      --workflow release.yml \
      --event release \
      --json databaseId,status,conclusion,createdAt \
      --limit 1 \
      --jq '.[0].databaseId'
  )"
  [[ -n "$run_id" && "$run_id" != "null" ]] || die "No release-event workflow run found after creating the GitHub Release"
  printf '%s\n' "$run_id"
}

trigger_testpypi() {
  local ref="$1"
  local run_id

  run gh workflow run release.yml -f publish_target=testpypi --ref "$ref"
  run_id="$(latest_test_run_id "$ref")"
  run gh run watch "$run_id" --exit-status
}

smoke_test_testpypi() {
  local version="$1"
  local venv="/tmp/canisend-testpypi-$version"
  local workspace="/tmp/canisend-testpypi-workspace-$version"

  run uv venv "$venv"
  run "$venv/bin/pip" install \
    --index-url https://test.pypi.org/simple/ \
    --extra-index-url https://pypi.org/simple/ \
    "canisend==$version"
  run "$venv/bin/canisend" --help
  run "$venv/bin/canisend" init-workspace --workspace "$workspace"
  run "$venv/bin/canisend" doctor --workspace "$workspace"
}

create_github_release() {
  local channel="$1"
  local version="$2"
  local ref="$3"
  local label="Stable"

  if [[ "$channel" == "beta" ]]; then
    label="Beta"
    run gh release create "v$version" \
      --target "$ref" \
      --title "CanISend $version" \
      --notes "$label release $version. See CHANGELOG.md and RELEASE.md for details." \
      --prerelease
    return 0
  fi

  run gh release create "v$version" \
    --target "$ref" \
    --title "CanISend $version" \
    --notes "$label release $version. See CHANGELOG.md and RELEASE.md for details."
}

wait_for_pypi_publish() {
  local run_id
  run_id="$(latest_pypi_release_run_id)"
  run gh run watch "$run_id" --exit-status
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
  local ref="main"
  local skip_local_checks=0
  local skip_testpypi_smoke=0
  local wait_pypi=1

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
      --skip-testpypi-smoke)
        skip_testpypi_smoke=1
        ;;
      --no-wait-pypi)
        wait_pypi=0
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

  if [[ "$DRY_RUN" != "1" && "$channel" != "test" ]]; then
    local project_version
    project_version="$(read_project_version)"
    [[ "$version" == "$project_version" ]] || die "--version $version does not match project version $project_version"
  fi

  if [[ "$skip_local_checks" == "0" ]]; then
    local_release_checks
  fi

  trigger_testpypi "$ref"

  if [[ "$skip_testpypi_smoke" == "0" ]]; then
    smoke_test_testpypi "$version"
  fi

  if [[ "$channel" == "test" ]]; then
    return 0
  fi

  create_github_release "$channel" "$version" "$ref"

  if [[ "$wait_pypi" == "1" ]]; then
    wait_for_pypi_publish
  fi

  printf '%s release v%s created and PyPI publish workflow was triggered.\n' "$channel" "$version"
}

main "$@"
