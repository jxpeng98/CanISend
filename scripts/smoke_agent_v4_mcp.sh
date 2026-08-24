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
for command in jq; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "Agent v4 MCP smoke: required command is missing: $command" >&2
    exit 1
  fi
done
if ! command -v cygpath >/dev/null 2>&1 \
  && ! command -v mkfifo >/dev/null 2>&1; then
  echo "Agent v4 MCP smoke: required command is missing: mkfifo" >&2
  exit 1
fi

mcp_pid=""
mcp_fds_open=false
cleanup_mcp_session() {
  if [[ "$mcp_fds_open" == true ]]; then
    exec 3>&-
    exec 4<&-
    mcp_fds_open=false
  fi
  if [[ -n "$mcp_pid" ]] && kill -0 "$mcp_pid" 2>/dev/null; then
    kill "$mcp_pid" 2>/dev/null || true
    wait "$mcp_pid" 2>/dev/null || true
  fi
  mcp_pid=""
}
trap cleanup_mcp_session EXIT

MCP_RESPONSE=""
MCP_NEXT_ID=100
mcp_request() {
  local request="$1"
  printf '%s\n' "$request" >&3
  if ! IFS= read -r MCP_RESPONSE <&4; then
    echo "Agent v4 MCP smoke: server closed before a dynamic response" >&2
    if [[ -f "$smoke_root/dynamic-mcp.stderr" ]]; then
      sed -n '1,120p' "$smoke_root/dynamic-mcp.stderr" >&2
    fi
    exit 1
  fi
}

mcp_tool_call() {
  local tool="$1"
  local arguments="$2"
  MCP_NEXT_ID=$((MCP_NEXT_ID + 1))
  mcp_request "$(
    jq -nc \
      --argjson id "$MCP_NEXT_ID" \
      --arg tool "$tool" \
      --argjson arguments "$arguments" \
      '{jsonrpc: "2.0", id: $id, method: "tools/call", params: {
        name: $tool,
        arguments: $arguments
      }}'
  )"
}

assert_mcp_operation() {
  local operation="$1"
  if ! jq -e --arg operation "$operation" '
    .result.isError == false
    and (
      .result.structuredContent.operation //
      .result.structuredContent.preview.operation
    ) == $operation
  ' <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: expected successful operation $operation" >&2
    jq -c . <<< "$MCP_RESPONSE" >&2 || true
    exit 1
  fi
}

assert_mcp_failure() {
  if ! jq -e '
    (.error | type == "object") or (.result.isError == true)
  ' <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: expected a fail-closed MCP response" >&2
    jq -c . <<< "$MCP_RESPONSE" >&2 || true
    exit 1
  fi
}

capture_preview_binding() {
  MCP_PREVIEW_TOKEN="$(
    jq -er '.result.structuredContent.preview_token' <<< "$MCP_RESPONSE"
  )"
  MCP_PREVIEW_SHA256="$(
    jq -er '.result.structuredContent.preview.data.preview_sha256' <<< "$MCP_RESPONSE"
  )"
}

mkdir -p "$smoke_root/candidates"
workspace="$smoke_root/workspace"
backup="$smoke_root/workspace-backup"
restored="$smoke_root/workspace-restored"
generic_candidate="$smoke_root/candidates/generic.json"
academic_candidate="$smoke_root/candidates/academic.json"
profile_source="$smoke_root/candidates/profile.md"

generic_source_text="MCP-V4-GENERIC-PRIVATE-SENTINEL requires a project narrative."
jq -n \
  --arg source_text "$generic_source_text" \
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

academic_source_text="MCP-V4-ACADEMIC-PRIVATE-SENTINEL requires an academic CV."
jq -n \
  --arg source_text "$academic_source_text" \
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
generic_source="$(jq -ec '.data.stored.snapshot.requirements[0].source_span.content' "$smoke_root/generic-create.json")"
academic_source="$(jq -ec '.data.stored.snapshot.requirements[0].source_span.content' "$smoke_root/academic-create.json")"
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
  (map(select(.id == 2))[0].result.tools | length == 36) and
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
    "canisend_export_list",
    "canisend_export_prepare_commit",
    "canisend_export_prepare_preview",
    "canisend_export_show",
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
    "canisend_review_disposition_commit",
    "canisend_review_disposition_preview",
    "canisend_review_inspect",
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

if command -v cygpath >/dev/null 2>&1; then
  coproc MCP_SERVER {
    "$binary" --workspace "$workspace" mcp serve \
      2> "$smoke_root/dynamic-mcp.stderr"
  }
  mcp_pid="$MCP_SERVER_PID"
  exec 3>&${MCP_SERVER[1]}-
  exec 4<&${MCP_SERVER[0]}-
else
  request_fifo="$smoke_root/dynamic-mcp.requests"
  response_fifo="$smoke_root/dynamic-mcp.responses"
  mkfifo "$request_fifo" "$response_fifo"
  "$binary" --workspace "$workspace" mcp serve \
    < "$request_fifo" \
    > "$response_fifo" \
    2> "$smoke_root/dynamic-mcp.stderr" &
  mcp_pid="$!"
  exec 3> "$request_fifo"
  exec 4< "$response_fifo"
fi
mcp_fds_open=true

mcp_request "$(
  jq -nc '{
    jsonrpc: "2.0",
    id: 100,
    method: "initialize",
    params: {
      protocolVersion: "2025-11-25",
      capabilities: {},
      clientInfo: {name: "canisend-packaged-lifecycle", version: "1.0"}
    }
  }'
)"
if ! jq -e '.result.protocolVersion == "2025-11-25"' \
  <<< "$MCP_RESPONSE" >/dev/null; then
  echo "Agent v4 MCP smoke: dynamic lifecycle initialization failed" >&2
  jq -c . <<< "$MCP_RESPONSE" >&2 || true
  exit 1
fi
printf '%s\n' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' >&3

qualify_application_lifecycle() {
  local label="$1"
  local application_id="$2"
  local original_requirement_id="$3"
  local source_reference="$4"
  local source_text="$5"
  local category="$6"
  local extracted_statement="$7"
  local valid_plan="$8"
  local drafts="$9"
  local expected_deliverable_count="${10}"
  local expected_pack="${11}"
  local invalid_kind="${12}"
  local private_marker="${13}"
  local start_byte end_byte arguments
  local extracted_requirement_id preview_token preview_sha256

  start_byte="$(
    jq -nr \
      --arg source_text "$source_text" \
      --arg statement "$extracted_statement" \
      '$source_text | index($statement)'
  )"
  if [[ ! "$start_byte" =~ ^[0-9]+$ ]]; then
    echo "Agent v4 MCP smoke: extraction statement is not in $label source" >&2
    exit 1
  fi
  end_byte=$((start_byte + ${#extracted_statement}))

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --argjson source "$source_reference" \
      --arg category "$category" \
      --arg statement "$extracted_statement" \
      --argjson start_byte "$start_byte" \
      --argjson end_byte "$end_byte" \
      '{
        application_id: $application_id,
        expected_revision: 1,
        source: $source,
        requirements: [{
          category: $category,
          statement: $statement,
          priority: "recommended",
          start_byte: $start_byte,
          end_byte: $end_byte
        }],
        confirmed_private_read: false
      }'
  )"
  mcp_tool_call "canisend_requirement_extract_preview" "$arguments"
  assert_mcp_operation "requirement.extract.preview"
  capture_preview_binding
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true,
        confirmed_private_read: false
      }'
  )"
  mcp_tool_call "canisend_requirement_extract_commit" "$arguments"
  assert_mcp_operation "requirement.extract.commit"
  extracted_requirement_id="$(
    jq -er '.result.structuredContent.data.snapshot.requirements[1].id' \
      <<< "$MCP_RESPONSE"
  )"

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg original_requirement_id "$original_requirement_id" \
      --arg extracted_requirement_id "$extracted_requirement_id" \
      '{
        application_id: $application_id,
        expected_revision: 2,
        decisions: [
          {requirement_id: $original_requirement_id, decision: "confirm"},
          {requirement_id: $extracted_requirement_id, decision: "confirm"}
        ]
      }'
  )"
  mcp_tool_call "canisend_requirement_confirm_preview" "$arguments"
  assert_mcp_operation "requirement.confirm.preview"
  capture_preview_binding
  preview_token="$MCP_PREVIEW_TOKEN"
  preview_sha256="$MCP_PREVIEW_SHA256"

  if [[ "$label" == "academic" ]]; then
    arguments="$(
      jq -nc \
        --arg application_id "$application_id" \
        --arg preview_token "$preview_token" \
        --arg preview_sha256 "$preview_sha256" \
        '{
          application_id: $application_id,
          preview_token: $preview_token,
          preview_sha256: $preview_sha256,
          approved: false
        }'
    )"
    mcp_tool_call "canisend_requirement_confirm_commit" "$arguments"
    assert_mcp_failure
    mcp_tool_call "canisend_requirement_confirm_preview" "$(
      jq -nc \
        --arg application_id "$application_id" \
        --arg original_requirement_id "$original_requirement_id" \
        --arg extracted_requirement_id "$extracted_requirement_id" \
        '{
          application_id: $application_id,
          expected_revision: 2,
          decisions: [
            {requirement_id: $original_requirement_id, decision: "confirm"},
            {requirement_id: $extracted_requirement_id, decision: "confirm"}
          ]
        }'
    )"
    assert_mcp_operation "requirement.confirm.preview"
    capture_preview_binding
  else
    mcp_tool_call "canisend_requirement_confirm_preview" "$arguments"
    assert_mcp_operation "requirement.confirm.preview"
    capture_preview_binding
  fi

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true
      }'
  )"
  mcp_tool_call "canisend_requirement_confirm_commit" "$arguments"
  assert_mcp_operation "requirement.confirm.commit"
  if ! jq -e '.result.structuredContent.data.snapshot.application.revision == 3' \
    <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: $label Requirement commit has the wrong revision" >&2
    exit 1
  fi
  mcp_tool_call "canisend_requirement_confirm_commit" "$arguments"
  assert_mcp_failure

  if [[ "$label" == "generic" ]]; then
    arguments="$(
      jq -nc \
        --arg application_id "$application_id" \
        --arg preview_token "$preview_token" \
        --arg preview_sha256 "$preview_sha256" \
        '{
          application_id: $application_id,
          preview_token: $preview_token,
          preview_sha256: $preview_sha256,
          approved: true
        }'
    )"
    mcp_tool_call "canisend_requirement_confirm_commit" "$arguments"
    assert_mcp_failure
  fi

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg invalid_kind "$invalid_kind" \
      '{
        application_id: $application_id,
        expected_revision: 3,
        decision: "proceed",
        deliverables: [{
          kind: $invalid_kind,
          disposition: "required",
          rationale: "Cross-Pack kind must fail",
          constraints: [],
          execution_mode: "host-agent"
        }]
      }'
  )"
  mcp_tool_call "canisend_plan_propose_preview" "$arguments"
  assert_mcp_failure

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --argjson deliverables "$valid_plan" \
      '{
        application_id: $application_id,
        expected_revision: 3,
        decision: "proceed",
        deliverables: $deliverables
      }'
  )"
  mcp_tool_call "canisend_plan_propose_preview" "$arguments"
  assert_mcp_operation "plan.propose.preview"
  capture_preview_binding
  if [[ "$label" == "generic" ]]; then
    arguments="$(
      jq -nc \
        --arg application_id "$academic_id" \
        --arg preview_token "$MCP_PREVIEW_TOKEN" \
        --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
        '{
          application_id: $application_id,
          preview_token: $preview_token,
          preview_sha256: $preview_sha256,
          approved: true
        }'
    )"
    mcp_tool_call "canisend_plan_propose_commit" "$arguments"
    assert_mcp_failure
    arguments="$(
      jq -nc \
        --arg application_id "$application_id" \
        --argjson deliverables "$valid_plan" \
        '{
          application_id: $application_id,
          expected_revision: 3,
          decision: "proceed",
          deliverables: $deliverables
        }'
    )"
    mcp_tool_call "canisend_plan_propose_preview" "$arguments"
    assert_mcp_operation "plan.propose.preview"
    capture_preview_binding
  fi
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true
      }'
  )"
  mcp_tool_call "canisend_plan_propose_commit" "$arguments"
  assert_mcp_operation "plan.propose.commit"

  arguments="$(
    jq -nc --arg application_id "$application_id" '{
      application_id: $application_id,
      expected_revision: 4
    }'
  )"
  mcp_tool_call "canisend_plan_confirm_preview" "$arguments"
  assert_mcp_operation "plan.confirm.preview"
  capture_preview_binding
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true
      }'
  )"
  mcp_tool_call "canisend_plan_confirm_commit" "$arguments"
  assert_mcp_operation "plan.confirm.commit"

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --argjson deliverables "$drafts" \
      '{
        application_id: $application_id,
        expected_revision: 5,
        deliverables: $deliverables
      }'
  )"
  mcp_tool_call "canisend_deliverable_draft_preview" "$arguments"
  assert_mcp_operation "deliverable.draft.preview"
  capture_preview_binding
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true
      }'
  )"
  mcp_tool_call "canisend_deliverable_draft_commit" "$arguments"
  assert_mcp_operation "deliverable.draft.commit"
  if ! jq -e \
    --argjson expected_deliverable_count "$expected_deliverable_count" '
      .result.structuredContent.data.snapshot.application.revision == 6
      and (.result.structuredContent.data.snapshot.deliverables | length) ==
        $expected_deliverable_count
    ' <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: $label draft result is incomplete" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{
      application_id: $application_id,
      confirmed_private_read: false
    }'
  )"
  mcp_tool_call "canisend_deliverable_audit" "$arguments"
  assert_mcp_failure
  if [[ "$MCP_RESPONSE" == *"$private_marker"* ]]; then
    echo "Agent v4 MCP smoke: denied $label audit leaked private content" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{
      application_id: $application_id,
      confirmed_private_read: true
    }'
  )"
  mcp_tool_call "canisend_deliverable_audit" "$arguments"
  assert_mcp_operation "deliverable.audit"
  if [[ "$MCP_RESPONSE" != *"$private_marker"* ]]; then
    echo "Agent v4 MCP smoke: approved $label audit omitted private content" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{
      application_id: $application_id,
      confirmed_private_read: false
    }'
  )"
  mcp_tool_call "canisend_review_inspect" "$arguments"
  assert_mcp_failure
  if [[ "$MCP_RESPONSE" == *"$private_marker"* ]]; then
    echo "Agent v4 MCP smoke: denied $label review leaked private content" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{
      application_id: $application_id,
      confirmed_private_read: true
    }'
  )"
  mcp_tool_call "canisend_review_inspect" "$arguments"
  assert_mcp_operation "review.inspect"
  if [[ "$MCP_RESPONSE" != *"$private_marker"* ]]; then
    echo "Agent v4 MCP smoke: approved $label review omitted private content" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{
      application_id: $application_id,
      expected_revision: 6,
      confirmed_private_read: true
    }'
  )"
  mcp_tool_call "canisend_review_disposition_preview" "$arguments"
  assert_mcp_operation "review.disposition.preview"
  capture_preview_binding
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true,
        confirmed_private_read: true
      }'
  )"
  mcp_tool_call "canisend_review_disposition_commit" "$arguments"
  assert_mcp_operation "review.disposition.commit"

  local export_destination="applications/$application_id/exports/$label-smoke"
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg destination "$export_destination" \
      '{
        application_id: $application_id,
        expected_revision: 7,
        destination: $destination,
        confirmed_private_export: true
      }'
  )"
  mcp_tool_call "canisend_export_prepare_preview" "$arguments"
  assert_mcp_operation "export.prepare.preview"
  capture_preview_binding
  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg preview_token "$MCP_PREVIEW_TOKEN" \
      --arg preview_sha256 "$MCP_PREVIEW_SHA256" \
      '{
        application_id: $application_id,
        preview_token: $preview_token,
        preview_sha256: $preview_sha256,
        approved: true,
        confirmed_private_export: true
      }'
  )"
  mcp_tool_call "canisend_export_prepare_commit" "$arguments"
  assert_mcp_operation "export.prepare.commit"
  if ! jq -e '
      .result.structuredContent.data.render.submission_performed == false
      and (.result.structuredContent.data.render.documents | length) > 0
    ' <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: $label export receipt is incomplete" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{application_id: $application_id}'
  )"
  mcp_tool_call "canisend_export_list" "$arguments"
  assert_mcp_operation "export.list"
  if ! jq -e '.result.structuredContent.data.exports | length == 1' \
    <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: $label export list is incomplete" >&2
    exit 1
  fi

  arguments="$(
    jq -nc \
      --arg application_id "$application_id" \
      --arg destination "$export_destination" \
      '{application_id: $application_id, destination: $destination}'
  )"
  mcp_tool_call "canisend_export_show" "$arguments"
  assert_mcp_operation "export.show"
  if ! jq -e '
      .result.structuredContent.data.manifest.submission_performed == false
      and (.result.structuredContent.data.manifest.documents | length) > 0
    ' <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: $label export verification is incomplete" >&2
    exit 1
  fi

  arguments="$(
    jq -nc --arg application_id "$application_id" '{application_id: $application_id}'
  )"
  mcp_tool_call "canisend_application_show" "$arguments"
  assert_mcp_operation "application.show"
  if ! jq -e \
    --arg expected_pack "$expected_pack" \
    --argjson expected_deliverable_count "$expected_deliverable_count" '
      .result.structuredContent.data.snapshot.application.revision == 7
      and .result.structuredContent.data.snapshot.pack.id == $expected_pack
      and .result.structuredContent.data.snapshot.plan.state == "confirmed"
      and (.result.structuredContent.data.snapshot.requirements | length) == 2
      and (.result.structuredContent.data.snapshot.requirements |
        all(.[]; .confirmation == "confirmed"))
      and (.result.structuredContent.data.snapshot.deliverables | length) ==
        $expected_deliverable_count
    ' <<< "$MCP_RESPONSE" >/dev/null; then
    echo "Agent v4 MCP smoke: $label final snapshot failed parity assertions" >&2
    jq -c . <<< "$MCP_RESPONSE" >&2 || true
    exit 1
  fi
  printf '%s\n' "$MCP_RESPONSE" > "$smoke_root/$label-lifecycle-final.json"
}

generic_plan='[{
  "kind": "primary-document",
  "disposition": "required",
  "rationale": "Required by the reviewed generic source",
  "constraints": [],
  "execution_mode": "host-agent"
}]'
generic_drafts='[{
  "kind": "primary-document",
  "title": "Reviewed project narrative",
  "media_type": "text/markdown",
  "content": "PRIVATE-MCP-GENERIC-DELIVERABLE"
}]'
academic_plan='[
  {
    "kind": "cover-letter",
    "disposition": "required",
    "rationale": "Required by the academic Pack",
    "constraints": [],
    "execution_mode": "host-agent"
  },
  {
    "kind": "cv",
    "disposition": "required",
    "rationale": "Required by the reviewed academic source",
    "constraints": [],
    "execution_mode": "host-agent"
  }
]'
academic_drafts='[
  {
    "kind": "cover-letter",
    "title": "Reviewed academic cover letter",
    "media_type": "text/markdown",
    "content": "PRIVATE-MCP-ACADEMIC-COVER-LETTER"
  },
  {
    "kind": "cv",
    "title": "Reviewed academic CV",
    "media_type": "text/markdown",
    "content": "PRIVATE-MCP-ACADEMIC-CV"
  }
]'

qualify_application_lifecycle \
  generic \
  "$generic_id" \
  "$generic_requirement_id" \
  "$generic_source" \
  "$generic_source_text" \
  format \
  "project narrative" \
  "$generic_plan" \
  "$generic_drafts" \
  1 \
  org.canisend.generic-application \
  cv \
  PRIVATE-MCP-GENERIC
qualify_application_lifecycle \
  academic \
  "$academic_id" \
  "$academic_requirement_id" \
  "$academic_source" \
  "$academic_source_text" \
  qualification \
  "academic CV" \
  "$academic_plan" \
  "$academic_drafts" \
  2 \
  org.canisend.academic-job \
  primary-document \
  PRIVATE-MCP-ACADEMIC

exec 3>&-
if ! wait "$mcp_pid"; then
  echo "Agent v4 MCP smoke: dynamic MCP server failed" >&2
  sed -n '1,120p' "$smoke_root/dynamic-mcp.stderr" >&2
  exit 1
fi
exec 4<&-
mcp_fds_open=false
mcp_pid=""

"$binary" --workspace "$workspace" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null
"$binary" --workspace "$workspace" workspace backup "$backup" --json \
  | jq -e '.ok == true and .operation == "workspace.backup.commit"' >/dev/null
"$binary" workspace restore "$backup" "$restored" --json \
  | jq -e '
      .ok == true
      and .operation == "workspace.restore.commit"
      and .data.application_count == 2
    ' >/dev/null
"$binary" --workspace "$restored" workspace status --json \
  | jq -e '
      .ok == true
      and .data.workspace_format == "canisend.workspace/v4"
      and .data.application_count == 2
    ' >/dev/null
"$binary" --workspace "$restored" application list --json \
  | jq -e '.ok == true and (.data | length) == 2' >/dev/null
"$binary" --workspace "$restored" workspace check --json \
  | jq -e '.ok == true and .data.ok == true' >/dev/null

echo "Agent v4 MCP smoke: ok (guarded dual-Pack lifecycle, backup, restore, and reopen passed)"
