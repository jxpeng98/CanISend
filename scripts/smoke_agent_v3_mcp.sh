#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/canisend}"
smoke_root="${2:-${TMPDIR:-/tmp}/canisend-agent-v3-mcp-smoke}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
smoke_root="$(canisend_absolute_path "$smoke_root")"

if [[ ! -x "$binary" ]]; then
  echo "Agent v3 MCP smoke: binary is not executable: $binary" >&2
  exit 1
fi
if [[ -e "$smoke_root" || -L "$smoke_root" ]]; then
  echo "Agent v3 MCP smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi
command -v jq >/dev/null

mkdir -p "$smoke_root"
generic_workspace="$smoke_root/generic-workspace"
academic_workspace="$smoke_root/academic-workspace"

run_mcp() {
  local label="$1"
  local workspace="$2"
  local input="$3"
  local output="$4"
  if ! "$binary" --workspace "$workspace" mcp serve < "$input" > "$output"; then
    echo "Agent v3 MCP smoke: $label server failed" >&2
    return 1
  fi
}

assert_mcp() {
  local label="$1"
  local filter="$2"
  local responses="$3"
  if jq -s -e "$filter" "$responses" >/dev/null; then
    return 0
  fi
  echo "Agent v3 MCP smoke: $label assertion failed" >&2
  jq -sc '
    map({
      id,
      is_error: (.result.isError // null),
      error_code: (.error.data.code // null),
      operation: (.result.structuredContent.operation // null)
    })
  ' "$responses" >&2 || true
  return 1
}

write_initialize() {
  jq -nc '{
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: {
      protocolVersion: "2025-11-25",
      capabilities: {},
      clientInfo: {name: "canisend-packaged-smoke", version: "1.0"}
    }
  }'
  jq -nc '{jsonrpc: "2.0", method: "notifications/initialized"}'
}

"$binary" --workspace "$generic_workspace" workspace init \
  --pack generic-application --json > "$smoke_root/generic-init.json"

{
  write_initialize
  jq -nc '{jsonrpc: "2.0", id: 2, method: "tools/list", params: {}}'
  jq -nc '{
    jsonrpc: "2.0",
    id: 3,
    method: "tools/call",
    params: {name: "canisend_agent_v3_capabilities", arguments: {}}
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 4,
    method: "tools/call",
    params: {name: "canisend_agent_v3_context", arguments: {}}
  }'
} > "$smoke_root/generic-empty.requests.jsonl"
run_mcp \
  "generic empty context" \
  "$generic_workspace" \
  "$smoke_root/generic-empty.requests.jsonl" \
  "$smoke_root/generic-empty.responses.jsonl"

assert_mcp "generic empty context" '
  (map(select(.id == 1))[0].result.protocolVersion == "2025-11-25") and
  (map(select(.id == 2))[0].result.tools | map(.name) |
    index("canisend_agent_v3_context") != null and
    index("canisend_application_create") != null) and
  (map(select(.id == 3))[0].result.isError == false) and
  (map(select(.id == 3))[0].result.structuredContent.data.protocol == "canisend.agent/v3") and
  (map(select(.id == 3))[0].result.structuredContent.data.pack.id ==
    "org.canisend.generic-application") and
  (map(select(.id == 4))[0].result.isError == false) and
  (map(select(.id == 4))[0].result.structuredContent.data.next_actions[0].action ==
    "canisend_application_create")
' "$smoke_root/generic-empty.responses.jsonl"

{
  write_initialize
  jq -nc '{
    jsonrpc: "2.0",
    id: 2,
    method: "tools/call",
    params: {
      name: "canisend_application_create",
      arguments: {
        title: "Packaged neutral MCP fixture",
        opportunity_metadata: {
          organization: {type: "short-text", value: "Synthetic qualification organization"}
        },
        application_metadata: {},
        source_text: "MCP-V3-PRIVATE-SENTINEL must produce one primary narrative.",
        requirements: [{
          category: "format",
          statement: "MCP-V3-PRIVATE-SENTINEL must produce one primary narrative.",
          priority: "mandatory",
          start_byte: 0,
          end_byte: 59
        }]
      }
    }
  }'
} > "$smoke_root/generic-create.requests.jsonl"
run_mcp \
  "generic create" \
  "$generic_workspace" \
  "$smoke_root/generic-create.requests.jsonl" \
  "$smoke_root/generic-create.responses.jsonl"

assert_mcp "generic create" '
  (map(select(.id == 1))[0].result.protocolVersion == "2025-11-25") and
  (map(select(.id == 2))[0].result.isError == false) and
  (map(select(.id == 2))[0].result.structuredContent.operation == "application.create")
' "$smoke_root/generic-create.responses.jsonl"

application_id="$(
  jq -sr '
    map(select(.id == 2))[0]
      .result.structuredContent.data.stored.snapshot.application.id
  ' "$smoke_root/generic-create.responses.jsonl"
)"
test -n "$application_id"

{
  write_initialize
  jq -nc --arg application_id "$application_id" '{
    jsonrpc: "2.0",
    id: 2,
    method: "tools/call",
    params: {
      name: "canisend_agent_v3_context",
      arguments: {application_id: $application_id}
    }
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 3,
    method: "tools/call",
    params: {name: "canisend_context", arguments: {}}
  }'
} > "$smoke_root/generic-resume.requests.jsonl"
run_mcp \
  "generic resume" \
  "$generic_workspace" \
  "$smoke_root/generic-resume.requests.jsonl" \
  "$smoke_root/generic-resume.responses.jsonl"

if grep -q 'MCP-V3-PRIVATE-SENTINEL\|Synthetic qualification organization' \
  "$smoke_root/generic-resume.responses.jsonl"; then
  echo "Agent v3 MCP smoke: body-free resumed context exposed private fixture content" >&2
  exit 1
fi
assert_mcp "generic resume" '
  (map(select(.id == 2))[0].result.isError == false) and
  (map(select(.id == 2))[0].result.structuredContent.data.protocol == "canisend.agent/v3") and
  (map(select(.id == 2))[0].result.structuredContent.data.submission_supported == false) and
  (map(select(.id == 2))[0].result.structuredContent.data.next_actions[0].action ==
    "canisend_application_plan") and
  ((map(select(.id == 3))[0].result.isError == true) or
    (map(select(.id == 3))[0].error.data.code == "compatibility.unavailable"))
' "$smoke_root/generic-resume.responses.jsonl"

"$binary" --workspace "$academic_workspace" workspace init \
  --pack academic-job --json > "$smoke_root/academic-init.json"

{
  write_initialize
  jq -nc '{
    jsonrpc: "2.0",
    id: 2,
    method: "tools/call",
    params: {name: "canisend_context", arguments: {}}
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 3,
    method: "tools/call",
    params: {name: "canisend_agent_v3_context", arguments: {}}
  }'
} > "$smoke_root/academic-compat.requests.jsonl"
run_mcp \
  "academic compatibility" \
  "$academic_workspace" \
  "$smoke_root/academic-compat.requests.jsonl" \
  "$smoke_root/academic-compat.responses.jsonl"

assert_mcp "academic compatibility" '
  (map(select(.id == 2))[0].result.isError == false) and
  (map(select(.id == 2))[0].result.structuredContent.data.protocol == "canisend.agent/v2") and
  (map(select(.id == 2))[0].result.structuredContent.compatibility.pack.id ==
    "org.canisend.academic-job") and
  ((map(select(.id == 3))[0].result.isError == true) or
    (map(select(.id == 3))[0].error.data.code == "compatibility.unavailable"))
' "$smoke_root/academic-compat.responses.jsonl"

"$binary" --workspace "$generic_workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null
"$binary" --workspace "$academic_workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

echo "Agent v3 MCP smoke: ok"
