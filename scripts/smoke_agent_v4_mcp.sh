#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/canisend}"
smoke_root="${2:-${TMPDIR:-/tmp}/canisend-agent-v4-mcp-smoke}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
smoke_root="$(canisend_absolute_path "$smoke_root")"

if [[ ! -x "$binary" ]]; then
  echo "Agent v4 MCP smoke: binary is not executable: $binary" >&2
  exit 1
fi
if [[ -e "$smoke_root" || -L "$smoke_root" ]]; then
  echo "Agent v4 MCP smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi
command -v jq >/dev/null

mkdir -p "$smoke_root/candidates"
workspace="$smoke_root/workspace"
generic_candidate="$smoke_root/candidates/generic.json"
academic_candidate="$smoke_root/candidates/academic.json"
profile_source="$smoke_root/candidates/profile.md"

jq -n \
  --arg source_text \
    "MCP-V4-GENERIC-PRIVATE-SENTINEL requires a project narrative." \
  '{
  title: "Packaged generic MCP fixture",
  opportunity_metadata: {
    organization: {type: "short-text", value: "Example Foundation"},
    reference: {type: "short-text", value: "MCP-V4-001"}
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
}' > "$generic_candidate"

jq -n \
  --arg source_text \
    "MCP-V4-ACADEMIC-PRIVATE-SENTINEL requires an academic CV." \
  '{
  title: "Packaged academic MCP fixture",
  opportunity_metadata: {
    institution: {type: "short-text", value: "Example University"}
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
}' > "$academic_candidate"

printf '%s\n' \
  '# Profile' \
  '' \
  'MCP-V4-PROFILE-PRIVATE-SENTINEL managed a cross-domain programme.' \
  > "$profile_source"

"$binary" --workspace "$workspace" workspace init --json \
  > "$smoke_root/workspace-init.json"
"$binary" --workspace "$workspace" application create \
  --pack org.canisend.generic-application \
  --candidate "$generic_candidate" \
  --json > "$smoke_root/generic-create.json"
"$binary" --workspace "$workspace" application create \
  --pack org.canisend.academic-job \
  --candidate "$academic_candidate" \
  --json > "$smoke_root/academic-create.json"
"$binary" --workspace "$workspace" profile-source import \
  "$profile_source" \
  --sensitivity private-local \
  --confirm-private-read \
  --json > "$smoke_root/profile-source-import.json"
"$binary" --workspace "$workspace" profile-source list --json \
  > "$smoke_root/profile-source-list.json"

generic_id="$(jq -er '.data.stored.snapshot.application.id' "$smoke_root/generic-create.json")"
academic_id="$(jq -er '.data.stored.snapshot.application.id' "$smoke_root/academic-create.json")"

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

{
  write_initialize
  jq -nc '{jsonrpc: "2.0", id: 2, method: "tools/list", params: {}}'
  jq -nc '{
    jsonrpc: "2.0",
    id: 3,
    method: "tools/call",
    params: {name: "canisend_workspace_status", arguments: {}}
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 4,
    method: "tools/call",
    params: {name: "canisend_workspace_check", arguments: {}}
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 5,
    method: "tools/call",
    params: {name: "canisend_application_list", arguments: {}}
  }'
  jq -nc --arg application_id "$generic_id" '{
    jsonrpc: "2.0",
    id: 6,
    method: "tools/call",
    params: {
      name: "canisend_application_show",
      arguments: {application_id: $application_id}
    }
  }'
  jq -nc --arg application_id "$academic_id" '{
    jsonrpc: "2.0",
    id: 7,
    method: "tools/call",
    params: {
      name: "canisend_application_show",
      arguments: {application_id: $application_id}
    }
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 8,
    method: "tools/call",
    params: {name: "canisend_profile_source_list", arguments: {}}
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 9,
    method: "tools/call",
    params: {name: "canisend_agent_v3_context", arguments: {}}
  }'
} > "$smoke_root/requests.jsonl"

if ! "$binary" --workspace "$workspace" mcp serve \
  < "$smoke_root/requests.jsonl" \
  > "$smoke_root/responses.jsonl"; then
  echo "Agent v4 MCP smoke: server failed" >&2
  exit 1
fi

if ! jq -s -e '
  (map(select(.id == 1))[0].result.protocolVersion == "2025-11-25") and
  (map(select(.id == 2))[0].result.tools | length == 5) and
  (map(select(.id == 2))[0].result.tools | all(.[]; .outputSchema.type == "object")) and
  (map(select(.id == 2))[0].result.tools | map(.name) | sort == [
    "canisend_application_list",
    "canisend_application_show",
    "canisend_profile_source_list",
    "canisend_workspace_check",
    "canisend_workspace_status"
  ]) and
  (map(select(.id == 2))[0].result.tools |
    all(.[];
      .annotations.readOnlyHint == true and
      .annotations.destructiveHint == false and
      .annotations.idempotentHint == true and
      .annotations.openWorldHint == false)) and
  (map(select(.id == 3))[0].result.structuredContent.operation == "workspace.status") and
  (map(select(.id == 3))[0].result.structuredContent.data.status.workspace_format ==
    "canisend.workspace/v4") and
  ((map(select(.id == 3 or .id == 4)) | tostring |
    contains("MCP-V4-GENERIC-PRIVATE-SENTINEL")) | not) and
  ((map(select(.id == 3 or .id == 4)) | tostring |
    contains("MCP-V4-ACADEMIC-PRIVATE-SENTINEL")) | not) and
  (map(select(.id == 4))[0].result.structuredContent.data.check.ok == true) and
  (map(select(.id == 5))[0].result.structuredContent.data | length == 2) and
  (map(select(.id == 6))[0] | tostring |
    contains("MCP-V4-GENERIC-PRIVATE-SENTINEL")) and
  ((map(select(.id == 6))[0] | tostring |
    contains("MCP-V4-ACADEMIC-PRIVATE-SENTINEL")) | not) and
  (map(select(.id == 7))[0] | tostring |
    contains("MCP-V4-ACADEMIC-PRIVATE-SENTINEL")) and
  ((map(select(.id == 7))[0] | tostring |
    contains("MCP-V4-GENERIC-PRIVATE-SENTINEL")) | not) and
  (map(select(.id == 6))[0].result.structuredContent.data.snapshot.pack.id ==
    "org.canisend.generic-application") and
  (map(select(.id == 7))[0].result.structuredContent.data.snapshot.pack.id ==
    "org.canisend.academic-job") and
  (map(select(.id == 8))[0].result.structuredContent.operation ==
    "profile-source.list") and
  (map(select(.id == 8))[0].result.structuredContent.data.sources | length == 1) and
  ((map(select(.id == 8))[0] | tostring |
    contains("MCP-V4-PROFILE-PRIVATE-SENTINEL")) | not) and
  (map(select(.id == 9))[0].error.code == -32602) and
  (map(select(.id == 9))[0].error.message == "tool not found")
' "$smoke_root/responses.jsonl" >/dev/null; then
  echo "Agent v4 MCP smoke: response assertion failed" >&2
  jq -sc '
    map({
      id,
      is_error: (.result.isError // null),
      error_code: (.error.data.code // null),
      operation: (.result.structuredContent.operation // null)
    })
  ' "$smoke_root/responses.jsonl" >&2 || true
  exit 1
fi

"$binary" --workspace "$workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

echo "Agent v4 MCP smoke: ok"
