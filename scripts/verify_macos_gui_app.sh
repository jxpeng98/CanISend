#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 CanISend.app CanISend.app.manifest.json" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS GUI verification: this script must run on macOS" >&2
  exit 1
fi

app="$1"
manifest="$2"
for command in codesign file jq plutil shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "macOS GUI verification: required command is missing: $command" >&2
    exit 1
  fi
done
if [[ ! -d "$app" || -L "$app" ]]; then
  echo "macOS GUI verification: app must be a regular directory" >&2
  exit 1
fi
if [[ ! -f "$manifest" || -L "$manifest" ]]; then
  echo "macOS GUI verification: manifest must be a regular non-symlink file" >&2
  exit 1
fi
if (( $(stat -f '%z' "$manifest") > 65536 )); then
  echo "macOS GUI verification: manifest exceeds the 65536-byte limit" >&2
  exit 1
fi
if find "$app" -type l -print -quit | grep -q .; then
  echo "macOS GUI verification: symlinks are not allowed in the app" >&2
  exit 1
fi

gui="$app/Contents/MacOS/canisend-gui"
cli="$app/Contents/Resources/bin/canisend"
metadata="$app/Contents/Resources/BUNDLE.json"
info="$app/Contents/Info.plist"
icon="$app/Contents/Resources/AppIcon.icns"
legal="$app/Contents/Resources/legal"
for file in \
  "$gui" \
  "$cli" \
  "$metadata" \
  "$info" \
  "$icon" \
  "$legal/LICENSE" \
  "$legal/THIRD_PARTY_NOTICES.md"
do
  if [[ ! -f "$file" || -L "$file" ]]; then
    echo "macOS GUI verification: required regular file is missing: $file" >&2
    exit 1
  fi
done

jq -e '
  .schema == "canisend.macos-app-integrity/v1"
  and (keys == ["bundle", "executables", "schema", "version"])
  and (.version | type == "string" and length > 0)
  and (.bundle | keys == ["info_plist", "metadata", "name", "signing"])
  and (.bundle.name | type == "string" and length > 0)
  and .bundle.signing == {
    kind: "apple-adhoc",
    developer_id: false,
    notarized: false
  }
  and (.bundle.metadata | keys == ["path", "sha256"])
  and (.bundle.info_plist | keys == ["path", "sha256"])
  and (.executables | keys == ["cli", "gui"])
  and (.executables.gui | keys == ["path", "sha256"])
  and (.executables.cli | keys == ["path", "sha256"])
  and .bundle.metadata.path == "Contents/Resources/BUNDLE.json"
  and .bundle.info_plist.path == "Contents/Info.plist"
  and .executables.gui.path == "Contents/MacOS/canisend-gui"
  and .executables.cli.path == "Contents/Resources/bin/canisend"
  and ([.bundle.metadata.sha256, .bundle.info_plist.sha256,
        .executables.gui.sha256, .executables.cli.sha256]
       | all(type == "string" and test("^[0-9a-f]{64}$")))
' "$manifest" >/dev/null

version="$(jq -er '.version' "$manifest")"
test "$(jq -er '.bundle.name' "$manifest")" = "$(basename "$app")"
test "$(jq -er '.bundle.metadata.sha256' "$manifest")" = \
  "$(shasum -a 256 "$metadata" | awk '{print $1}')"
test "$(jq -er '.bundle.info_plist.sha256' "$manifest")" = \
  "$(shasum -a 256 "$info" | awk '{print $1}')"
test "$(jq -er '.executables.gui.sha256' "$manifest")" = \
  "$(shasum -a 256 "$gui" | awk '{print $1}')"
test "$(jq -er '.executables.cli.sha256' "$manifest")" = \
  "$(shasum -a 256 "$cli" | awk '{print $1}')"

jq -e \
  --arg version "$version" \
  '. == {
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
  }' "$metadata" >/dev/null

test "$(plutil -extract CanISendProductVersion raw "$info")" = "$version"
test "$(plutil -extract CFBundleIconFile raw "$info")" = "AppIcon"
if [[ "$(file -b "$icon")" != "Mac OS X icon"* ]]; then
  echo "macOS GUI verification: AppIcon.icns is not a valid macOS icon" >&2
  exit 1
fi
test "$("$cli" version --json | jq -er '.data.version')" = "$version"
codesign --verify --deep --strict --verbose=4 "$app"

echo "macOS GUI verification: final signed bytes, layout, version, and ad-hoc signature passed"
