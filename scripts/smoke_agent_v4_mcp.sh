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
generic_requirement_id="$(jq -er '.data.stored.snapshot.requirements[0].id' "$smoke_root/generic-create.json")"
academic_requirement_id="$(jq -er '.data.stored.snapshot.requirements[0].id' "$smoke_root/academic-create.json")"
profile_source_id="$(jq -er '.data.source.id' "$smoke_root/profile-source-import.json")"
profile_source_revision="$(jq -er '.data.source.revision' "$smoke_root/profile-source-import.json")"
profile_source_sha256="$(jq -er '.data.source.original.sha256' "$smoke_root/profile-source-import.json")"
"$binary" --workspace "$workspace" profile association list \
  --application "$generic_id" --json > "$smoke_root/profile-association-list.json"
"$binary" --workspace "$workspace" evidence association list \
  --application "$generic_id" --json > "$smoke_root/evidence-association-list.json"

write_initialize() {
  jq -nc --arg application_id "$generic_id" '{
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
  jq -nc --arg application_id "$generic_id" '{
    jsonrpc: "2.0",
    id: 9,
    method: "tools/call",
    params: {
      name: "canisend_profile_association_list",
      arguments: {application_id: $application_id}
    }
  }'
  jq -nc --arg application_id "$generic_id" '{
    jsonrpc: "2.0",
    id: 10,
    method: "tools/call",
    params: {
      name: "canisend_evidence_association_list",
      arguments: {application_id: $application_id}
    }
  }'
  jq -nc \
    --arg application_id "$generic_id" \
    --arg profile_source_id "$profile_source_id" \
    --argjson profile_source_revision "$profile_source_revision" \
    --arg profile_source_sha256 "$profile_source_sha256" '{
    jsonrpc: "2.0",
    id: 11,
    method: "tools/call",
    params: {
      name: "canisend_profile_association_preview",
      arguments: {
        application_id: $application_id,
        profile_source: {
          id: $profile_source_id,
          revision: $profile_source_revision,
          sha256: $profile_source_sha256
        },
        change: "associate"
      }
    }
  }'
  jq -nc '{
    jsonrpc: "2.0",
    id: 12,
    method: "tools/call",
    params: {name: "canisend_agent_v3_context", arguments: {}}
  }'
  for pack_context in \
    "$generic_id:$generic_requirement_id:13" \
    "$academic_id:$academic_requirement_id:17"; do
    application_id="${pack_context%%:*}"
    remainder="${pack_context#*:}"
    requirement_id="${remainder%%:*}"
    base_id="${remainder##*:}"
    jq -nc --arg application_id "$application_id" --argjson id "$base_id" '{
      jsonrpc: "2.0", id: $id, method: "tools/call",
      params: {name: "canisend_requirement_list", arguments: {application_id: $application_id}}
    }'
    jq -nc --arg application_id "$application_id" --arg requirement_id "$requirement_id" \
      --argjson id "$((base_id + 1))" '{
      jsonrpc: "2.0", id: $id, method: "tools/call",
      params: {name: "canisend_requirement_show", arguments: {
        application_id: $application_id, requirement_id: $requirement_id
      }}
    }'
    jq -nc --arg application_id "$application_id" --argjson id "$((base_id + 2))" '{
      jsonrpc: "2.0", id: $id, method: "tools/call",
      params: {name: "canisend_plan_show", arguments: {application_id: $application_id}}
    }'
    jq -nc --arg application_id "$application_id" --argjson id "$((base_id + 3))" '{
      jsonrpc: "2.0", id: $id, method: "tools/call",
      params: {name: "canisend_deliverable_list", arguments: {application_id: $application_id}}
    }'
  done
  jq -nc --arg application_id "$generic_id" '{
    jsonrpc: "2.0", id: 21, method: "tools/call",
    params: {name: "canisend_deliverable_show", arguments: {
      application_id: $application_id,
      deliverable_id: "019f2f55-7c00-7000-8000-000000000999"
    }}
  }'
  jq -nc --arg application_id "$generic_id" --arg requirement_id "$generic_requirement_id" '{
    jsonrpc: "2.0", id: 22, method: "tools/call",
    params: {name: "canisend_requirement_confirm_preview", arguments: {
      application_id: $application_id,
      expected_revision: 1,
      decisions: [{requirement_id: $requirement_id, decision: "confirm"}]
    }}
  }'
} > "$smoke_root/requests.jsonl"

if ! "$binary" --workspace "$workspace" mcp serve \
  < "$smoke_root/requests.jsonl" \
  > "$smoke_root/responses.jsonl"; then
  echo "Agent v4 MCP smoke: server failed" >&2
  exit 1
fi

if ! jq -s -e '
  . as $responses |
  (map(select(.id == 1))[0].result.protocolVersion == "2025-11-25") and
  (map(select(.id == 2))[0].result.tools | length == 29) and
  (map(select(.id == 2))[0].result.tools | all(.[]; .outputSchema.type == "object")) and
  (map(select(.id == 2))[0].result.tools | map(.name) | sort == [
    "canisend_application_list",
    "canisend_application_show",
    "canisend_deliverable_audit",
    "canisend_deliverable_draft_commit",
    "canisend_deliverable_draft_preview",
    "canisend_deliverable_list",
    "canisend_deliverable_revise_commit",
    "canisend_deliverable_revise_preview",
    "canisend_deliverable_show",
    "canisend_evidence_association_commit",
    "canisend_evidence_association_list",
    "canisend_evidence_association_preview",
    "canisend_plan_confirm_commit",
    "canisend_plan_confirm_preview",
    "canisend_plan_propose_commit",
    "canisend_plan_propose_preview",
    "canisend_plan_show",
    "canisend_profile_association_commit",
    "canisend_profile_association_list",
    "canisend_profile_association_preview",
    "canisend_profile_source_list",
    "canisend_requirement_confirm_commit",
    "canisend_requirement_confirm_preview",
    "canisend_requirement_extract_commit",
    "canisend_requirement_extract_preview",
    "canisend_requirement_list",
    "canisend_requirement_show",
    "canisend_workspace_check",
    "canisend_workspace_status"
  ]) and
  (map(select(.id == 2))[0].result.tools |
    all(.[];
      (if (.name | endswith("_commit")) then
        .annotations.readOnlyHint == false and
        .annotations.destructiveHint == true and
        .annotations.idempotentHint == false
      elif (.name | endswith("_preview")) then
        .annotations.readOnlyHint == true and
        .annotations.destructiveHint == false and
        .annotations.idempotentHint == false
      else
        .annotations.readOnlyHint == true and
        .annotations.destructiveHint == false and
        .annotations.idempotentHint == true
      end) and
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
  (map(select(.id == 9))[0].result.structuredContent.operation ==
    "profile.association.list") and
  ((map(select(.id == 9))[0] | tostring |
    contains("MCP-V4-PROFILE-PRIVATE-SENTINEL")) | not) and
  (map(select(.id == 10))[0].result.structuredContent.operation ==
    "evidence.association.list") and
  (map(select(.id == 11))[0].result.structuredContent.preview_token |
    startswith("apv1_")) and
  (map(select(.id == 11))[0].result.structuredContent.preview.operation ==
    "profile.association.preview") and
  (map(select(.id == 11))[0].result.structuredContent.preview.data.requires_private_read == true) and
  ((map(select(.id == 11))[0] | tostring |
    contains("MCP-V4-PROFILE-PRIVATE-SENTINEL")) | not) and
  (map(select(.id == 12))[0].error.code == -32602) and
  (map(select(.id == 12))[0].error.message == "tool not found")
  and
  ([13, 17] | all(.[]; . as $id |
    ($responses | map(select(.id == $id))[0].result.structuredContent.operation == "requirement.list") and
    ($responses | map(select(.id == $id))[0].result.structuredContent.data.requirements | length == 1))) and
  ([14, 18] | all(.[]; . as $id |
    ($responses | map(select(.id == $id))[0].result.structuredContent.operation == "requirement.show"))) and
  ([15, 19] | all(.[]; . as $id |
    ($responses | map(select(.id == $id))[0].result.structuredContent.operation == "plan.show") and
    ($responses | map(select(.id == $id))[0].result.structuredContent.status == "not-created"))) and
  ([16, 20] | all(.[]; . as $id |
    ($responses | map(select(.id == $id))[0].result.structuredContent.operation == "deliverable.list") and
    ($responses | map(select(.id == $id))[0].result.structuredContent.data.deliverables | length == 0))) and
  ($responses | map(select(.id == 21))[0].error.code == -32602)
  and
  (map(select(.id == 22))[0].result.structuredContent.preview.operation ==
    "requirement.confirm.preview") and
  (map(select(.id == 22))[0].result.structuredContent.preview_token |
    startswith("apv1_"))
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
