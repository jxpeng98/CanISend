#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Build an isolated, non-publishing macOS CanISend Design Preview App.

Usage:
  ./scripts/build_macos_design_preview.sh [--open] [--keep] [--skip-ui-tests]
  pnpm --dir apps/canisend-desktop macos:preview -- [options]

Options:
  --open           Launch the preview App with an isolated HOME and wait for it to close.
  --keep           Preserve the temporary preview directory after the launched App closes.
  --skip-ui-tests  Skip Svelte diagnostics and Playwright UI tests before building.
  -h, --help       Show this help.

Without --open, the generated temporary directory is preserved so the App can be
opened manually. With --open, it is removed after the App closes unless --keep is
also supplied.

The generated App is ad-hoc signed, uses synthetic local-only fixture data, and is
for design review only. It is not notarized and must not be published or distributed.
EOF
}

open_after_build="false"
keep_preview="false"
run_ui_tests="true"
for argument in "$@"; do
  case "$argument" in
    --)
      ;;
    --open)
      open_after_build="true"
      ;;
    --keep)
      keep_preview="true"
      ;;
    --skip-ui-tests)
      run_ui_tests="false"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n\n' "$argument" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$(/usr/bin/uname -s)" != "Darwin" ]]; then
  printf 'macOS Design Preview can only be built on macOS.\n' >&2
  exit 1
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd -P)"
frontend_root="$repo_root/apps/canisend-desktop"
manifest="$repo_root/Cargo.toml"
profile="release-alpha"

for command in cargo codesign git jq plutil pnpm shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'macOS Design Preview requires command: %s\n' "$command" >&2
    exit 1
  fi
done
if [[ "$open_after_build" == "true" ]] && ! command -v open >/dev/null 2>&1; then
  printf 'macOS Design Preview requires the macOS open command.\n' >&2
  exit 1
fi
if [[ ! -d "$frontend_root/node_modules" ]]; then
  printf 'Desktop dependencies are missing; run:\n' >&2
  printf '  pnpm --dir apps/canisend-desktop install --frozen-lockfile\n' >&2
  exit 1
fi

if [[ "$run_ui_tests" == "true" ]]; then
  printf 'Checking the Svelte application...\n'
  pnpm --dir "$frontend_root" check

  printf 'Running visual, reflow, and accessibility tests...\n'
  pnpm --dir "$frontend_root" test:visual
fi

printf 'Building embedded Svelte assets...\n'
pnpm --dir "$frontend_root" build

printf 'Building the unified local macOS host...\n'
cargo build \
  --manifest-path "$manifest" \
  --locked \
  --profile "$profile" \
  --package canisend-gui \
  --features canisend-gui/custom-protocol

target_directory="$(
  cargo metadata \
    --manifest-path "$manifest" \
    --locked \
    --no-deps \
    --format-version 1 | jq -er '.target_directory'
)"
host_binary="$target_directory/$profile/canisend-gui"
if [[ ! -f "$host_binary" || -L "$host_binary" || ! -x "$host_binary" ]]; then
  printf 'Expected unified host is missing: %s\n' "$host_binary" >&2
  exit 1
fi

preview_root="$(/usr/bin/mktemp -d -t CanISend.design-preview)"
preview_root="$(CDPATH= cd -- "$preview_root" && pwd -P)"
cleanup_preview="false"
preview_ready="false"
cleanup() {
  if [[ "$preview_ready" != "true" || "$cleanup_preview" == "true" ]] \
    && [[ -d "$preview_root" ]]; then
    rm -rf -- "$preview_root"
  fi
}
trap cleanup EXIT

app="$preview_root/CanISend Design Preview.app"
manifest_path="$app.manifest.json"
preview_home="$preview_root/manual-home"
workspace="$preview_root/design-workspace"
log="$preview_root/canisend-design-preview.log"
receipt="$preview_root/canisend-design-preview.receipt.json"
mkdir -p "$preview_home/.codex" "$preview_home/.claude"

printf 'Staging and verifying the temporary App...\n'
"$script_dir/stage_macos_gui_app.sh" "$host_binary" "$app"
if [[ ! -f "$manifest_path" ]]; then
  printf 'Design Preview integrity manifest is missing: %s\n' "$manifest_path" >&2
  exit 1
fi

unified_host="$app/Contents/MacOS/canisend-gui"
info_plist="$app/Contents/Info.plist"
preview_bundle_identifier="io.github.jxpeng98.canisend.design-preview"
if [[ "$(plutil -extract CFBundleIdentifier raw "$info_plist")" != "io.github.jxpeng98.canisend" ]]; then
  printf 'Design Preview source App has an unexpected bundle identifier.\n' >&2
  exit 1
fi
plutil -replace CFBundleIdentifier -string "$preview_bundle_identifier" "$info_plist"
plutil -replace CFBundleDisplayName -string "CanISend Design Preview" "$info_plist"
plutil -replace CFBundleName -string "CanISend Design Preview" "$info_plist"
codesign \
  --force \
  --identifier "$preview_bundle_identifier" \
  --options runtime \
  --sign - \
  --timestamp=none \
  "$app"
info_plist_sha256="$(shasum -a 256 "$info_plist" | awk '{print $1}')"
preview_host_sha256="$(shasum -a 256 "$unified_host" | awk '{print $1}')"
manifest_temporary="$preview_root/.design-preview-integrity.json.tmp"
jq \
  --arg info_plist_sha256 "$info_plist_sha256" \
  --arg host_sha256 "$preview_host_sha256" \
  '.bundle.info_plist.sha256 = $info_plist_sha256 | .host.sha256 = $host_sha256' \
  "$manifest_path" > "$manifest_temporary"
mv "$manifest_temporary" "$manifest_path"
"$script_dir/verify_macos_gui_app.sh" "$app" "$manifest_path"

printf 'Creating an isolated design-review workspace...\n'
HOME="$preview_home" "$unified_host" \
  --workspace "$workspace" workspace init --json \
  > "$preview_root/workspace-init.json"
workspace="$(CDPATH= cd -- "$workspace" && pwd -P)"

generic_source="Design preview generic fixture requires a reviewed project narrative."
jq -n \
  --arg source_text "$generic_source" \
  '{
    title: "Senior Programme and Evidence Lead",
    opportunity_metadata: {
      organization: {type: "short-text", value: "Northbridge Social Impact Lab"},
      reference: {type: "short-text", value: "DESIGN-GENERIC-001"}
    },
    application_metadata: {
      status: {type: "choice", value: "planning"}
    },
    source_text: $source_text,
    requirements: [{
      category: "format",
      statement: $source_text,
      priority: "mandatory",
      start_byte: 0,
      end_byte: ($source_text | utf8bytelength)
    }]
  }' > "$preview_root/generic-candidate.json"

academic_source="Design preview academic fixture requires an academic CV."
jq -n \
  --arg source_text "$academic_source" \
  '{
    title: "Postdoctoral Research Fellow in Evidence and Public Policy",
    opportunity_metadata: {
      institution: {type: "short-text", value: "Institute for Public Policy"}
    },
    application_metadata: {},
    source_text: $source_text,
    requirements: [{
      category: "qualification",
      statement: $source_text,
      priority: "mandatory",
      start_byte: 0,
      end_byte: ($source_text | utf8bytelength)
    }]
  }' > "$preview_root/academic-candidate.json"

HOME="$preview_home" "$unified_host" \
  --workspace "$workspace" application create \
  --pack org.canisend.generic-application \
  --candidate "$preview_root/generic-candidate.json" \
  --json > "$preview_root/application-generic.json"
HOME="$preview_home" "$unified_host" \
  --workspace "$workspace" application create \
  --pack org.canisend.academic-job \
  --candidate "$preview_root/academic-candidate.json" \
  --json > "$preview_root/application-academic.json"

registry="$preview_home/Library/Application Support/CanISend/workspaces.json"
mkdir -p "$(dirname -- "$registry")"
jq -n \
  --arg workspace "$workspace" \
  '{
    format: "canisend.workspace-registry/v1",
    default_path: $workspace,
    entries: [{
      alias: "Design preview workspace",
      path: $workspace,
      pinned: true,
      last_opened_unix: 1
    }]
  }' > "$registry"

source_commit="$(git -C "$repo_root" rev-parse --verify HEAD 2>/dev/null || true)"
if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]]; then
  source_commit="unknown"
fi
source_dirty="false"
if [[ -n "$(git -C "$repo_root" status --short)" ]]; then
  source_dirty="true"
fi
version="$("$unified_host" version --json | jq -er '.data.version')"

jq -n \
  --arg version "$version" \
  --arg profile "$profile" \
  --arg generated_at "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" \
  --arg source_commit "$source_commit" \
  --argjson source_dirty "$source_dirty" \
  --arg app "$app" \
  --arg integrity_manifest "$manifest_path" \
  --arg preview_home "$preview_home" \
  --arg workspace "$workspace" \
  '{
    schema: "canisend.macos-design-preview/v1",
    status: "ready-local-design-review",
    version: $version,
    profile: $profile,
    generated_at: $generated_at,
    source: {
      commit: $source_commit,
      dirty: $source_dirty
    },
    app: $app,
    integrity_manifest: $integrity_manifest,
    isolated_home: $preview_home,
    synthetic_workspace: $workspace,
    bundle_identifier: "io.github.jxpeng98.canisend.design-preview",
    signing: "apple-adhoc",
    notarized: false,
    publication_allowed: false
  }' > "$receipt"
preview_ready="true"

printf '\nLocal macOS Design Preview is ready:\n'
printf '  App:       %s\n' "$app"
printf '  HOME:      %s\n' "$preview_home"
printf '  Workspace: %s\n' "$workspace"
printf '  Receipt:   %s\n' "$receipt"
printf '  Signing:   ad-hoc; publishing: forbidden\n'

if [[ "$open_after_build" == "true" ]]; then
  if [[ "$keep_preview" != "true" ]]; then
    cleanup_preview="true"
    printf '  Cleanup:   automatic after the App closes\n'
  else
    printf '  Cleanup:   preserved because --keep was supplied\n'
  fi

  /usr/bin/touch "$log"
  /bin/chmod 600 "$log"
  printf '\nLaunching with a clean HOME isolated from real user and Agent configuration...\n'
  /usr/bin/open \
    -n \
    -W \
    --env "HOME=$preview_home" \
    --env "CODEX_HOME=$preview_home/.codex" \
    --env "CLAUDE_CONFIG_DIR=$preview_home/.claude" \
    --stdout "$log" \
    --stderr "$log" \
    "$app"
  printf 'Design Preview closed. Log: %s\n' "$log"
else
  printf '  Cleanup:   preserved in the system temporary directory\n'
  printf '\nRun again with --open to launch an automatically cleaned preview.\n'
fi
