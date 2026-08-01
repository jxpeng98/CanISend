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

host="$app/Contents/MacOS/canisend-gui"
legacy_cli="$app/Contents/Resources/bin/canisend"
metadata="$app/Contents/Resources/BUNDLE.json"
info="$app/Contents/Info.plist"
icon="$app/Contents/Resources/AppIcon.icns"
legal="$app/Contents/Resources/legal"
for file in \
  "$host" \
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
if [[ -e "$legacy_cli" || -L "$legacy_cli" ]]; then
  echo "macOS GUI verification: duplicated legacy CLI must not be packaged: $legacy_cli" >&2
  exit 1
fi
executable_files="$(find "$app/Contents" -type f -perm -111 -print | sort)"
if [[ "$executable_files" != "$host" ]]; then
  echo "macOS GUI verification: App must contain exactly one executable host" >&2
  printf '%s\n' "$executable_files" >&2
  exit 1
fi
if [[ "$(file -b "$host")" != "Mach-O"* ]]; then
  echo "macOS GUI verification: unified host is not a Mach-O executable" >&2
  exit 1
fi

jq -e '
  .schema == "canisend.macos-app-integrity/v2"
  and (keys == ["bundle", "host", "schema", "version"])
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
  and (.host | keys == ["entry_modes", "path", "sha256"])
  and .bundle.metadata.path == "Contents/Resources/BUNDLE.json"
  and .bundle.info_plist.path == "Contents/Info.plist"
  and .host.path == "Contents/MacOS/canisend-gui"
  and .host.entry_modes == ["gui", "cli", "mcp"]
  and ([.bundle.metadata.sha256, .bundle.info_plist.sha256,
        .host.sha256]
       | all(type == "string" and test("^[0-9a-f]{64}$")))
' "$manifest" >/dev/null

version="$(jq -er '.version' "$manifest")"
test "$(jq -er '.bundle.name' "$manifest")" = "$(basename "$app")"
test "$(jq -er '.bundle.metadata.sha256' "$manifest")" = \
  "$(shasum -a 256 "$metadata" | awk '{print $1}')"
test "$(jq -er '.bundle.info_plist.sha256' "$manifest")" = \
  "$(shasum -a 256 "$info" | awk '{print $1}')"
test "$(jq -er '.host.sha256' "$manifest")" = \
  "$(shasum -a 256 "$host" | awk '{print $1}')"

jq -e \
  --arg version "$version" \
  '. == {
    schema: "canisend.macos-app-bundle/v3",
    version: $version,
    signing: {
      kind: "apple-adhoc",
      developer_id: false,
      notarized: false
    },
    host: {
      path: "Contents/MacOS/canisend-gui",
      entry_modes: ["gui", "cli", "mcp"]
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
test "$("$host" version --json | jq -er '.data.version')" = "$version"
host_bytes="$(stat -f '%z' "$host")"
bundle_bytes="$(du -sk "$app" | awk '{print $1 * 1024}')"
if (( host_bytes > 67108864 )); then
  echo "macOS GUI verification: unified host exceeds the 64 MiB budget" >&2
  exit 1
fi
if (( bundle_bytes > 75497472 )); then
  echo "macOS GUI verification: App payload exceeds the provisional 72 MiB budget" >&2
  exit 1
fi
codesign --verify --strict --verbose=4 "$host"
codesign --verify --deep --strict --verbose=4 "$app"

echo "macOS GUI verification: one unified host, final bytes, version, size, and signatures passed"
