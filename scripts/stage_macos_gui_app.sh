#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 GUI_BINARY CLI_BINARY DESTINATION.app" >&2
  exit 2
fi

gui_binary="$1"
cli_binary="$2"
destination="$3"
companion_manifest="$destination.manifest.json"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source "$script_dir/lib/native_paths.sh"
gui_binary="$(canisend_absolute_path "$gui_binary")"
cli_binary="$(canisend_absolute_path "$cli_binary")"
destination="$(canisend_absolute_path "$destination")"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI bundle: this script must run on macOS" >&2
  exit 1
fi
if [[ "$destination" != *.app ]]; then
  echo "macOS GUI bundle: destination must end in .app" >&2
  exit 2
fi
if [[ -e "$destination" ]]; then
  echo "macOS GUI bundle: destination must not exist: $destination" >&2
  exit 1
fi
if [[ -e "$companion_manifest" ]]; then
  echo "macOS GUI bundle: companion manifest must not exist: $companion_manifest" >&2
  exit 1
fi
for binary in "$gui_binary" "$cli_binary"; do
  if [[ ! -f "$binary" || -L "$binary" ]]; then
    echo "macOS GUI bundle: binary must be a regular non-symlink file: $binary" >&2
    exit 1
  fi
done
for command in codesign jq plutil shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI bundle: required command is missing: $command" >&2
    exit 1
  fi
done

version_json="$("$cli_binary" version --json)"
version="$(printf '%s' "$version_json" | jq -er '.data.version')"
numeric_version="${version%%-*}"
contents="$destination/Contents"
macos="$contents/MacOS"
resources="$contents/Resources"
cli_destination="$resources/bin/canisend"
legal="$resources/legal"

mkdir -p "$macos" "$resources/bin" "$legal"
cp "$gui_binary" "$macos/canisend-gui"
cp "$cli_binary" "$cli_destination"
cp "$repo_root/packaging/macos/Info.plist" "$contents/Info.plist"
cp "$repo_root/LICENSE" "$legal/LICENSE"
cp "$repo_root/THIRD_PARTY_NOTICES.md" "$legal/THIRD_PARTY_NOTICES.md"
cp "$repo_root/docs/guides/desktop-gui.md" "$resources/DESKTOP-GUI.md"
cp "$repo_root/docs/guides/privacy-and-consent.md" "$resources/PRIVACY.md"
chmod 755 "$macos/canisend-gui" "$cli_destination"

plutil -replace CFBundleShortVersionString -string "$numeric_version" "$contents/Info.plist"
plutil -replace CFBundleVersion -string "$numeric_version" "$contents/Info.plist"
plutil -replace CanISendProductVersion -string "$version" "$contents/Info.plist"
plutil -lint "$contents/Info.plist" >/dev/null

codesign \
  --force \
  --identifier io.github.jxpeng98.canisend.cli \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$cli_destination"
codesign \
  --force \
  --identifier io.github.jxpeng98.canisend.gui \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$macos/canisend-gui"

jq -n \
  --arg version "$version" \
  '{
    schema: "canisend.macos-app-bundle/v2",
    version: $version,
    signing: {
      kind: "apple-adhoc",
      developer_id: false,
      notarized: false
    },
    executables: {
      gui: {path: "Contents/MacOS/canisend-gui"},
      cli: {path: "Contents/Resources/bin/canisend"}
    },
    integrity_manifest: {
      placement: "external-companion",
      suffix: ".manifest.json"
    }
  }' > "$resources/BUNDLE.json"

if find "$destination" -type l -print -quit | grep -q .; then
  echo "macOS GUI bundle: symlinks are not allowed in the staged app" >&2
  exit 1
fi

codesign \
  --force \
  --identifier io.github.jxpeng98.canisend \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$destination"
codesign --verify --deep --strict --verbose=4 "$destination"

gui_sha256="$(shasum -a 256 "$macos/canisend-gui" | awk '{print $1}')"
cli_sha256="$(shasum -a 256 "$cli_destination" | awk '{print $1}')"
bundle_metadata_sha256="$(shasum -a 256 "$resources/BUNDLE.json" | awk '{print $1}')"
info_plist_sha256="$(shasum -a 256 "$contents/Info.plist" | awk '{print $1}')"
jq -n \
  --arg version "$version" \
  --arg bundle_name "$(basename "$destination")" \
  --arg gui_sha256 "$gui_sha256" \
  --arg cli_sha256 "$cli_sha256" \
  --arg bundle_metadata_sha256 "$bundle_metadata_sha256" \
  --arg info_plist_sha256 "$info_plist_sha256" \
  '{
    schema: "canisend.macos-app-integrity/v1",
    version: $version,
    bundle: {
      name: $bundle_name,
      signing: {
        kind: "apple-adhoc",
        developer_id: false,
        notarized: false
      },
      metadata: {
        path: "Contents/Resources/BUNDLE.json",
        sha256: $bundle_metadata_sha256
      },
      info_plist: {
        path: "Contents/Info.plist",
        sha256: $info_plist_sha256
      }
    },
    executables: {
      gui: {path: "Contents/MacOS/canisend-gui", sha256: $gui_sha256},
      cli: {path: "Contents/Resources/bin/canisend", sha256: $cli_sha256}
    }
  }' > "$companion_manifest"

"$script_dir/verify_macos_gui_app.sh" "$destination" "$companion_manifest"

echo "macOS GUI bundle: staged and ad-hoc signed $destination"
echo "macOS GUI bundle: final-byte integrity manifest $companion_manifest"
