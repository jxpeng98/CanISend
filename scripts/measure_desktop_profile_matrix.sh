#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 TARGET EVIDENCE_DIRECTORY [BUILD_ROOT]" >&2
}

if [[ $# -lt 2 || $# -gt 3 ]]; then
  usage
  exit 2
fi

target="$1"
evidence_directory="$2"
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ $# -eq 3 ]]; then
  build_root="$3"
else
  build_root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-profile-matrix.XXXXXX")"
fi

case "$target" in
  *windows*) host_name="canisend-gui.exe" ;;
  *) host_name="canisend-gui" ;;
esac

mkdir -p "$evidence_directory" "$build_root"
evidence_directory="$(cd "$evidence_directory" && pwd)"
build_root="$(cd "$build_root" && pwd)"

cd "$repository_root"
cargo run -p xtask --locked -- desktop template-audit
pnpm --dir apps/canisend-desktop build

while IFS=' ' read -r candidate opt_level lto; do
  candidate_root="$build_root/$candidate"
  CARGO_TARGET_DIR="$candidate_root" \
    CARGO_PROFILE_RELEASE_OPT_LEVEL="$opt_level" \
    CARGO_PROFILE_RELEASE_LTO="$lto" \
    cargo build --locked -p canisend-gui --release \
      --target "$target" --features custom-protocol
  host="$candidate_root/$target/release/$host_name"
  test -f "$host"
  cargo run -p xtask --locked -- desktop profile-record \
    "$target" "$candidate" "$opt_level" "$lto" "$host" \
    "$evidence_directory/$candidate.json"
done <<'MATRIX'
release 3 thin
size-s-thin s thin
size-z-thin z thin
size-z-fat z fat
MATRIX

cargo run -p xtask --locked -- desktop profile-summary \
  "$evidence_directory/release.json" \
  "$evidence_directory/size-s-thin.json" \
  "$evidence_directory/size-z-thin.json" \
  "$evidence_directory/size-z-fat.json" \
  "$evidence_directory/summary.json"

echo "desktop profile evidence: $evidence_directory"
echo "desktop profile build root: $build_root"
