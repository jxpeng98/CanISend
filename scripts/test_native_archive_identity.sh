#!/usr/bin/env bash
set -euo pipefail

# Bounded negative fixtures; no release qualification or full workflow smoke.
if [[ $# -ne 1 ]]; then
  echo "usage: $0 BINARY" >&2
  exit 2
fi
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$1")"
root="$(mktemp -d "${TMPDIR:-/tmp}/canisend-archive-identity.XXXXXX")"
trap 'rm -rf "$root"' EXIT
"$binary" version --json > "$root/version.json"
target="$(jq -er '.data.target' "$root/version.json")"
wrong_target=aarch64-apple-darwin
if [[ "$target" == "$wrong_target" ]]; then wrong_target=x86_64-apple-darwin; fi
if bash "$script_dir/package_native_release.sh" "$binary" "$wrong_target" "$root/package" \
  > "$root/package.log" 2>&1; then
  echo "identity regression: wrong-target package unexpectedly succeeded" >&2
  exit 1
fi
grep -q 'build target does not match' "$root/package.log"
test ! -e "$root/package"

bundle="$root/canisend-fixture-$target"
mkdir "$bundle"
executable=canisend
if [[ "$target" == x86_64-pc-windows-msvc ]]; then executable=canisend.exe; fi
cp "$binary" "$bundle/$executable"
for name in LICENSE THIRD_PARTY_NOTICES.md TYPST-ASSETS-LICENSE TYPST-ASSETS-NOTICE \
  KNOWN_LIMITATIONS.md FEEDBACK.md INSTALL.md PRIVACY.md SECURITY.md; do
  printf 'identity regression fixture\n' > "$bundle/$name"
done
printf '%s\n' "$target" > "$bundle/TARGET"
jq -c '.data.version = "0.0.0-mismatched"' "$root/version.json" > "$bundle/RELEASE.json"
tar -czf "$root/fixture.tar.gz" -C "$root" "$(basename "$bundle")"
if bash "$script_dir/smoke_release_archive.sh" "$root/fixture.tar.gz" "$target" \
  "$root/smoke" "$binary" > "$root/smoke.log" 2>&1; then
  echo "identity regression: mismatched RELEASE.json unexpectedly succeeded" >&2
  exit 1
fi
grep -q 'RELEASE.json differs from the executable identity' "$root/smoke.log"
test ! -e "$root/smoke/documented-workflow"
# A caller-controlled archive name and TARGET file cannot relabel the executable.
wrong_bundle="$root/canisend-fixture-$wrong_target"
mv "$bundle" "$wrong_bundle"
printf '%s\n' "$wrong_target" > "$wrong_bundle/TARGET"
cp "$root/version.json" "$wrong_bundle/RELEASE.json"
tar -czf "$root/wrong-target.tar.gz" -C "$root" "$(basename "$wrong_bundle")"
if bash "$script_dir/smoke_release_archive.sh" "$root/wrong-target.tar.gz" "$wrong_target" \
  "$root/wrong-target-smoke" "$binary" > "$root/wrong-target.log" 2>&1; then
  echo "identity regression: relabeled archive unexpectedly succeeded" >&2
  exit 1
fi
grep -q 'executable build target does not match' "$root/wrong-target.log"
test ! -e "$root/wrong-target-smoke/documented-workflow"
echo "native archive identity regressions: ok"
