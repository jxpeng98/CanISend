#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 ARCHIVE TARGET_TRIPLE NEW_SMOKE_DIRECTORY" >&2
  exit 2
fi

archive="$1"
target="$2"
smoke_root="$3"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
archive="$(canisend_absolute_path "$archive")"
smoke_root="$(canisend_absolute_path "$smoke_root")"

if [[ ! -f "$archive" || -L "$archive" ]]; then
  echo "release smoke: archive must be a regular non-symlink file: $archive" >&2
  exit 1
fi
if [[ -e "$smoke_root" ]]; then
  echo "release smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi

mkdir -p "$smoke_root/extracted"
case "$archive" in
  *.tar.gz)
    tar -xzf "$archive" -C "$smoke_root/extracted"
    ;;
  *.zip)
    if command -v 7z >/dev/null 2>&1; then
      7z x -y "-o$smoke_root/extracted" "$archive" >/dev/null
    elif command -v unzip >/dev/null 2>&1; then
      unzip -q "$archive" -d "$smoke_root/extracted"
    else
      echo "release smoke: 7z or unzip is required for a zip archive" >&2
      exit 1
    fi
    ;;
  *)
    echo "release smoke: unsupported archive format: $archive" >&2
    exit 2
    ;;
esac

bundle="$(find "$smoke_root/extracted" -mindepth 1 -maxdepth 1 -type d -name "canisend-*-$target" -print -quit)"
if [[ -z "$bundle" ]]; then
  echo "release smoke: target bundle was not found after extraction" >&2
  exit 1
fi
if find "$bundle" -type l -print -quit | grep -q .; then
  echo "release smoke: extracted bundle contains a symlink" >&2
  exit 1
fi

case "$target" in
  x86_64-pc-windows-msvc) executable="$bundle/canisend.exe" ;;
  *) executable="$bundle/canisend" ;;
esac
chmod +x "$executable"

for required in \
  "$executable" \
  "$bundle/LICENSE" \
  "$bundle/THIRD_PARTY_NOTICES.md" \
  "$bundle/TYPST-ASSETS-LICENSE" \
  "$bundle/TYPST-ASSETS-NOTICE" \
  "$bundle/KNOWN_LIMITATIONS.md" \
  "$bundle/INSTALL.md" \
  "$bundle/PRIVACY.md" \
  "$bundle/SECURITY.md" \
  "$bundle/RELEASE.json" \
  "$bundle/TARGET"
do
  test -f "$required"
done
grep -qx "$target" "$bundle/TARGET"
grep -q '"operation":"product.version"' "$bundle/RELEASE.json"
grep -q '"version":"' "$bundle/RELEASE.json"

"$executable" version --json > "$smoke_root/version.json"
"$executable" doctor --json > "$smoke_root/doctor.json"
"$executable" agent capabilities --json > "$smoke_root/capabilities.json"
grep -q '"python_required":false' "$smoke_root/doctor.json"
grep -q '"embedded_typst":"verified"' "$smoke_root/doctor.json"
grep -q '"runtime_package_downloads":false' "$smoke_root/doctor.json"

"$script_dir/smoke_documented_quickstart.sh" "$executable" "$smoke_root/documented-workflow"
"$script_dir/smoke_host_agent.sh" "$executable" "$smoke_root/host-agent-workflow"

echo "release archive smoke: ok ($target)"
