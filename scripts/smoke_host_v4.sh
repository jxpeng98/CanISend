#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 BINARY NEW_SMOKE_DIRECTORY" >&2
  exit 2
fi

binary="$1"
smoke_root="$2"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
smoke_root="$(canisend_absolute_path "$smoke_root")"
registered_binary="$binary"
if command -v cygpath >/dev/null 2>&1; then
  registered_binary="$(cygpath -w "$binary")"
fi

if [[ ! -x "$binary" || -L "$binary" ]]; then
  echo "Agent v4 host smoke: binary is not an executable regular file: $binary" >&2
  exit 1
fi
if [[ -e "$smoke_root" || -L "$smoke_root" ]]; then
  echo "Agent v4 host smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "Agent v4 host smoke: required command is missing: jq" >&2
  exit 1
fi

mkdir -p "$smoke_root"
workspace="$smoke_root/workspace"
"$binary" --workspace "$workspace" workspace init --json \
  > "$smoke_root/workspace-init.json"
jq -e '
  .ok == true
  and .operation == "workspace.initialize.commit"
  and .data.workspace_format == "canisend.workspace/v4"
  and .data.application_count == 0
' "$smoke_root/workspace-init.json" >/dev/null

for host in codex claude; do
  "$binary" --workspace "$workspace" host setup \
    --host "$host" \
    --executable "$registered_binary" \
    --json > "$smoke_root/$host-setup.json"
  jq -e \
    --arg host "$host" \
    --arg binary "$registered_binary" '
      .ok == true
      and .operation == "host.setup"
      and .status == "ready"
      and .data.host == $host
      and .data.skills.state == "installed"
      and (.data.skills.files | length) >= 4
      and .data.mcp.protocol_version == "2025-11-25"
      and .data.mcp.transport == "stdio"
      and .data.mcp.executable == $binary
      and (.data.mcp.tools | length) == 29
      and (.data.mcp.read_only_tools | length) == 21
      and (.data.mcp.guarded_write_tools | length) == 8
      and (.data.mcp.registration_command | contains("mcp serve"))
      and .data.mcp_configuration_mutated == false
    ' "$smoke_root/$host-setup.json" >/dev/null

  "$binary" --workspace "$workspace" host status \
    --host "$host" \
    --executable "$registered_binary" \
    --json > "$smoke_root/$host-status.json"
  jq -e \
    --arg host "$host" '
      .ok == true
      and .operation == "host.status"
      and .status == "ready"
      and .data.host == $host
      and .data.skills.state == "up-to-date"
      and (.data.mcp.tools | length) == 29
      and .data.mcp_configuration_mutated == false
    ' "$smoke_root/$host-status.json" >/dev/null
done

test -f "$workspace/.agents/canisend-agent-v4.json"
test -d "$workspace/.agents/skills"
test -f "$workspace/.claude/canisend-agent-v4.json"
test -d "$workspace/.claude/skills"
test ! -e "$workspace/.codex/config.toml"
test ! -e "$workspace/.mcp.json"

for host in codex claude; do
  "$binary" --workspace "$workspace" host remove \
    --host "$host" \
    --json > "$smoke_root/$host-remove.json"
  jq -e \
    --arg host "$host" '
      .ok == true
      and .operation == "host.remove"
      and .status == "removed"
      and .data.host == $host
      and .data.skills.state == "removed"
      and .data.mcp_configuration_removed == false
    ' "$smoke_root/$host-remove.json" >/dev/null
done

test ! -e "$workspace/.agents/canisend-agent-v4.json"
test ! -e "$workspace/.claude/canisend-agent-v4.json"
"$binary" --workspace "$workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

legacy_workspace="$smoke_root/legacy-refusal-workspace"
"$binary" --workspace "$legacy_workspace" workspace init --json \
  > "$smoke_root/legacy-workspace-init.json"
mkdir -p "$legacy_workspace/.agents/skills/canisend-application"
printf '%s\n' 'PRE-V4-HOST-RESOURCE-SENTINEL' \
  > "$legacy_workspace/.agents/skills/canisend-application/SKILL.md"
if "$binary" --workspace "$legacy_workspace" host setup \
  --host codex \
  --executable "$registered_binary" \
  --json > "$smoke_root/legacy-refusal.json"; then
  echo "Agent v4 host smoke: unsupported pre-v4 resource was accepted" >&2
  exit 1
fi
jq -e '
  .ok == false
  and .operation == "host.setup"
  and (.error.message | contains("unsupported pre-v4 host resources"))
' "$smoke_root/legacy-refusal.json" >/dev/null
grep -qx 'PRE-V4-HOST-RESOURCE-SENTINEL' \
  "$legacy_workspace/.agents/skills/canisend-application/SKILL.md"
test ! -e "$legacy_workspace/.agents/canisend-agent-v4.json"
"$binary" --workspace "$legacy_workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

echo "Agent v4 host smoke: ok (setup, status, remove, and legacy refusal passed)"
