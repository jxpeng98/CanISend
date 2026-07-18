#!/usr/bin/env bash

# Convert a repository-relative, POSIX absolute, or GitHub Actions Windows path to a Git Bash-safe absolute path.
canisend_absolute_path() {
  if [[ $# -ne 1 || -z "$1" ]]; then
    echo "native path: expected one non-empty path" >&2
    return 2
  fi
  local value="$1"
  case "$value" in
    [A-Za-z]:\\*|[A-Za-z]:/*)
      if ! command -v cygpath >/dev/null 2>&1; then
        echo "native path: cygpath is required for Windows path: $value" >&2
        return 1
      fi
      cygpath -u "$value"
      ;;
    /*)
      printf '%s\n' "$value"
      ;;
    *)
      printf '%s/%s\n' "$PWD" "$value"
      ;;
  esac
}
