#!/usr/bin/env bash
# Local source/tarball installs only; never publish or touch a user's install prefix.
set -euo pipefail
if [[ $# -ne 1 ]]; then
  echo "usage: $0 NEW_OUTPUT_DIRECTORY" >&2
  exit 2
fi
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
output="$(canisend_absolute_path "$1")"
if [[ -e "$output" || -L "$output" ]]; then
  echo "local installers: output must not exist: $output" >&2
  exit 2
fi
# ponytail: one native Unix package for local validation; public multi-platform dispatch is deferred.
case "$(uname -s)" in Darwin|Linux) ;; *) echo 'local installers: Unix host required' >&2; exit 2;; esac
for tool in cargo npm node jq; do command -v "$tool" >/dev/null; done
cd "$script_dir/.."
mkdir -p "$output"
export npm_config_cache="$output/npm-cache"
export npm_config_offline=true
export npm_config_audit=false
export npm_config_fund=false
export npm_config_ignore_scripts=true
cargo install --path crates/canisend-cli --locked --offline --root "$output/cargo" \
  > "$output/cargo-install.log" 2>&1
binary="$output/cargo/bin/canisend"
"$binary" version --json > "$output/version.json"
target="$(jq -er '.data.target' "$output/version.json")"
version="$(jq -er '.data.version' "$output/version.json")"
"$script_dir/stage_native_bundle.sh" "$binary" "$output/npm-package" "$target"
node - "$output/npm-package/package.json" "$version" <<'JS'
const fs = require('node:fs');
fs.writeFileSync(process.argv[2], JSON.stringify({
  name: 'canisend-cli-local', version: process.argv[3], private: true,
  description: 'Local-only CanISend native CLI installation fixture',
  license: 'GPL-3.0-only', os: [process.platform], cpu: [process.arch],
  bin: {canisend: 'canisend'},
}, null, 2) + '\n');
JS
(cd "$output/npm-package" && npm pack --json --pack-destination "$output") > "$output/npm-pack.json"
tarball="$output/$(jq -er '.[0].filename' "$output/npm-pack.json")"
npm install --global --prefix "$output/npm" "$tarball" > "$output/npm-install.log" 2>&1
npm_binary="$output/npm/bin/canisend"
"$npm_binary" version --json > "$output/npm-version.json"
cmp "$output/version.json" "$output/npm-version.json"
cmp "$binary" "$npm_binary"
for channel in cargo npm; do
  installed="$output/$channel/bin/canisend"
  # Exercise the package-manager entry itself, including npm's command symlink.
  workspace="$output/$channel-workspace"
  "$installed" --workspace "$workspace" workspace init --host codex --json > "$output/$channel-init.json"
  test "$(find "$workspace/.agents/skills" -name SKILL.md | wc -l | tr -d ' ')" = 5
  "$installed" --workspace "$workspace" workspace check --json > "$output/$channel-before.json"
  CANISEND_TEST_CLI_BINARY="$installed" cargo test -p canisend-cli --locked --offline --test mcp_protocol \
    > "$output/$channel-host.log" 2>&1
  # Existing setup/remove smoke requires a regular file, not the npm bin symlink.
  resolved="$(node -e 'process.stdout.write(require("node:fs").realpathSync(process.argv[1]))' "$installed")"
  "$script_dir/smoke_host_v4.sh" "$resolved" "$output/$channel-host-setup" > "$output/$channel-setup.log" 2>&1
done
npm uninstall --global --prefix "$output/npm" canisend-cli-local > "$output/npm-uninstall.log" 2>&1
test ! -e "$npm_binary"
"$binary" --workspace "$output/npm-workspace" workspace check --json > "$output/npm-after.json"
cmp "$output/npm-before.json" "$output/npm-after.json"
# Keep an independent verification copy so uninstall cannot hide workspace damage.
cp "$binary" "$output/verification-cli"
cargo uninstall --root "$output/cargo" canisend-cli > "$output/cargo-uninstall.log" 2>&1
test ! -e "$binary"
"$output/verification-cli" --workspace "$output/cargo-workspace" workspace check --json > "$output/cargo-after.json"
cmp "$output/cargo-before.json" "$output/cargo-after.json"
jq -n --slurpfile identity "$output/version.json" \
  '{local_installation_verified: true, channels: ["cargo-path", "npm-local-tarball"],
    identity: $identity[0].data, published: false, release_qualified: false}' > "$output/result.json"
echo "local installers: Cargo and npm install, five Skills, simulated Host and uninstall passed: $output"
