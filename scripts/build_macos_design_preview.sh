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

for command in cargo git jq pnpm; do
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

printf 'Building local macOS GUI and bundled CLI binaries...\n'
cargo build \
  --manifest-path "$manifest" \
  --locked \
  --profile "$profile" \
  --package canisend-gui \
  --package canisend-cli \
  --features canisend-gui/custom-protocol

target_directory="$(
  cargo metadata \
    --manifest-path "$manifest" \
    --locked \
    --no-deps \
    --format-version 1 | jq -er '.target_directory'
)"
gui_binary="$target_directory/$profile/canisend-gui"
cli_binary="$target_directory/$profile/canisend"
for binary in "$gui_binary" "$cli_binary"; do
  if [[ ! -f "$binary" || -L "$binary" || ! -x "$binary" ]]; then
    printf 'Expected built executable is missing: %s\n' "$binary" >&2
    exit 1
  fi
done

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

app="$preview_root/CanISend.app"
manifest_path="$app.manifest.json"
preview_home="$preview_root/manual-home"
workspace="$preview_root/design-workspace"
log="$preview_root/canisend-design-preview.log"
receipt="$preview_root/canisend-design-preview.receipt.json"
mkdir -p "$preview_home/.codex" "$preview_home/.claude"

printf 'Staging and verifying the temporary App...\n'
"$script_dir/stage_macos_gui_app.sh" "$gui_binary" "$cli_binary" "$app"
if [[ ! -f "$manifest_path" ]]; then
  printf 'Design Preview integrity manifest is missing: %s\n' "$manifest_path" >&2
  exit 1
fi

bundled_cli="$app/Contents/Resources/bin/canisend"
printf 'Creating an isolated design-review workspace...\n'
HOME="$preview_home" "$bundled_cli" \
  --workspace "$workspace" workspace init --json \
  > "$preview_root/workspace-init.json"
workspace="$(CDPATH= cd -- "$workspace" && pwd -P)"

HOME="$preview_home" "$bundled_cli" \
  --workspace "$workspace" job create \
  --title "Senior Lecturer in Evidence-Based Research and Academic Programme Leadership" \
  --institution "Northbridge University School of Social Sciences" \
  --json > "$preview_root/job-primary.json"
HOME="$preview_home" "$bundled_cli" \
  --workspace "$workspace" job create \
  --title "Postdoctoral Research Fellow" \
  --institution "Institute for Public Policy" \
  --json > "$preview_root/job-secondary.json"

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
version="$("$bundled_cli" version --json | jq -er '.data.version')"

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
