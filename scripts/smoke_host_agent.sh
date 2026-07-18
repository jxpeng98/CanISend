#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/canisend}"
smoke_root="${2:-${TMPDIR:-/tmp}/canisend-host-agent-smoke}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"

case "$binary" in
  /*) ;;
  *) binary="$PWD/$binary" ;;
esac

if [[ ! -x "$binary" ]]; then
  echo "host-agent smoke: binary is not executable: $binary" >&2
  exit 1
fi
if [[ -e "$smoke_root" ]]; then
  echo "host-agent smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi

mkdir -p "$smoke_root"
workspace="$smoke_root/workspace"
agent_work="$smoke_root/agent-work"
pack="$smoke_root/codex-pack"
mkdir -p "$agent_work"

first_uuid_id() {
  printf '%s' "$1" \
    | grep -oE '"id":"[0-9a-fA-F-]{36}"' \
    | sed -n '1p' \
    | cut -d'"' -f4
}

task_lease_id() {
  printf '%s' "$1" \
    | sed -E 's/^.*"lease":\{"expires_at":"[^"]+","id":"([0-9a-fA-F-]{36})"\}.*$/\1/'
}

task_job_revision() {
  printf '%s' "$1" \
    | grep -oE '"job_revision":[0-9]+' \
    | sed -n '1p' \
    | cut -d: -f2
}

task_profile_revision() {
  printf '%s' "$1" \
    | grep -oE '"profile_revision":[0-9]+' \
    | sed -n '1p' \
    | cut -d: -f2
}

task_expected_inputs() {
  printf '%s' "$1" \
    | sed -E 's/^.*"input_artifacts":(\[[^]]*\]),"job_id".*$/\1/' \
    | sed -E 's/"id":/"artifact_id":/g; s/,"kind":"[^"]+"//g'
}

write_completion() {
  local task_json="$1"
  local requirement="$2"
  local output="$3"
  local task_id lease_id job_revision expected_inputs source_artifact
  local source_id source_revision source_sha256
  task_id="$(first_uuid_id "$task_json")"
  lease_id="$(task_lease_id "$task_json")"
  job_revision="$(task_job_revision "$task_json")"
  expected_inputs="$(task_expected_inputs "$task_json")"
  source_id="$(printf '%s' "$expected_inputs" | grep -oE '"artifact_id":"[^"]+"' | cut -d'"' -f4)"
  source_revision="$(printf '%s' "$expected_inputs" | grep -oE '"revision":[0-9]+' | cut -d: -f2)"
  source_sha256="$(printf '%s' "$expected_inputs" | grep -oE '"sha256":"[0-9a-f]+"' | cut -d'"' -f4)"
  source_artifact="{\"kind\":\"source-normalized-text\",\"id\":\"$source_id\",\"revision\":$source_revision,\"sha256\":\"$source_sha256\"}"
  printf '%s\n' \
    "{\"task_id\":\"$task_id\",\"lease_id\":\"$lease_id\",\"expected_job_revision\":$job_revision,\"expected_inputs\":$expected_inputs,\"candidate\":{\"id\":\"019f2f55-7c00-7000-8000-000000000201\",\"job_id\":\"$job_id\",\"title\":\"Lecturer in Economics\",\"institution\":\"Northbridge University\",\"summary\":\"Teaching and research role in economics.\",\"responsibilities\":[\"Teach economics\",\"Maintain an active research programme\"],\"criteria\":[{\"id\":\"019f2f55-7c00-7000-8000-000000000202\",\"job_id\":\"$job_id\",\"kind\":\"teaching\",\"requirement\":\"$requirement\",\"importance\":\"essential\",\"source_quote\":\"$source_quote\",\"source_span\":{\"source\":$source_artifact,\"start_byte\":$source_start,\"end_byte\":$source_end},\"confidence_milli\":950,\"confirmed\":false,\"revision\":1}],\"revision\":1}}" \
    > "$output"
}

write_evidence_completion() {
  local task_json="$1"
  local output="$2"
  local task_id lease_id job_revision profile_revision expected_inputs source_artifact
  local source_id source_revision source_sha256
  task_id="$(first_uuid_id "$task_json")"
  lease_id="$(task_lease_id "$task_json")"
  job_revision="$(task_job_revision "$task_json")"
  profile_revision="$(task_profile_revision "$task_json")"
  expected_inputs="$(task_expected_inputs "$task_json")"
  source_id="$(printf '%s' "$expected_inputs" | grep -oE '"artifact_id":"[^"]+"' | cut -d'"' -f4)"
  source_revision="$(printf '%s' "$expected_inputs" | grep -oE '"revision":[0-9]+' | cut -d: -f2)"
  source_sha256="$(printf '%s' "$expected_inputs" | grep -oE '"sha256":"[0-9a-f]+"' | cut -d'"' -f4)"
  source_artifact="{\"kind\":\"source-normalized-text\",\"id\":\"$source_id\",\"revision\":$source_revision,\"sha256\":\"$source_sha256\"}"
  printf '%s\n' \
    "{\"task_id\":\"$task_id\",\"lease_id\":\"$lease_id\",\"expected_job_revision\":$job_revision,\"expected_inputs\":$expected_inputs,\"candidate\":{\"profile_revision\":$profile_revision,\"proposals\":[{\"kind\":\"qualification\",\"summary\":\"Doctorate in Economics\",\"source_quote\":\"$evidence_quote\",\"source_span\":{\"source\":$source_artifact,\"start_byte\":$evidence_start,\"end_byte\":$evidence_end},\"sensitivity\":\"private-local\"}]}}" \
    > "$output"
}

task_input_artifact() {
  local task_json="$1"
  local kind="$2"
  printf '%s' "$task_json" \
    | grep -oE "\\{\"id\":\"[0-9a-fA-F-]{36}\",\"kind\":\"$kind\",\"revision\":[0-9]+,\"sha256\":\"[0-9a-f]+\"\\}" \
    | sed -n '1p'
}

write_match_completion() {
  local task_json="$1"
  local output="$2"
  local task_id lease_id job_revision expected_inputs criteria_artifact evidence_artifact
  task_id="$(first_uuid_id "$task_json")"
  lease_id="$(task_lease_id "$task_json")"
  job_revision="$(task_job_revision "$task_json")"
  expected_inputs="$(task_expected_inputs "$task_json")"
  criteria_artifact="$(task_input_artifact "$task_json" criteria)"
  evidence_artifact="$(task_input_artifact "$task_json" evidence-catalog)"
  test -n "$criteria_artifact"
  test -n "$evidence_artifact"
  printf '%s\n' \
    "{\"task_id\":\"$task_id\",\"lease_id\":\"$lease_id\",\"expected_job_revision\":$job_revision,\"expected_inputs\":$expected_inputs,\"candidate\":{\"job_id\":\"$job_id\",\"criteria_artifact\":$criteria_artifact,\"evidence_artifact\":$evidence_artifact,\"proposals\":[{\"criterion\":{\"id\":\"019f2f55-7c00-7000-8000-000000000202\",\"revision\":1},\"evidence\":[{\"id\":\"$evidence_item_id\",\"revision\":1}],\"strength\":\"partial\",\"rationale\":\"The doctorate supports subject expertise but does not establish teaching effectiveness.\",\"gap\":\"No confirmed evidence of effective university-level teaching.\",\"prohibited_claims\":[\"Do not claim the doctorate proves teaching effectiveness.\"]}]}}" \
    > "$output"
}

"$binary" agent capabilities --json >/dev/null
"$binary" agent assets export --host codex --destination "$pack" --json >/dev/null
test -f "$pack/AGENTS.md"
test -f "$pack/canisend-agent-pack.json"
test -f "$pack/prompts/evidence-normalize.md"
test -f "$pack/prompts/evidence-match.md"
test -f "$pack/schemas/v2/evidence-proposals.schema.json"
test -f "$pack/schemas/v2/evidence-match-proposals.schema.json"
test -f "$pack/schemas/v2/application-plan-candidate.schema.json"
test -f "$pack/schemas/v2/document-candidate.schema.json"

"$binary" --workspace "$workspace" workspace init --json >/dev/null
job_json="$(
  "$binary" --workspace "$workspace" job create \
    --title "Lecturer in Economics" \
    --institution "University X" \
    --json
)"
job_id="$(first_uuid_id "$job_json")"
"$binary" --workspace "$workspace" job import "$job_id" \
  --file "$repo_root/fixtures/v2-spec/job-advert.md" --json >/dev/null
"$binary" --workspace "$workspace" workflow start --job "$job_id" --json \
  >"$agent_work/workflow-start.json"
"$binary" --workspace "$workspace" workflow status --job "$job_id" --json \
  >"$agent_work/workflow-status.json"
grep -q '"stage":"intake","status":"complete"' "$agent_work/workflow-status.json"
grep -q '"stage":"parse","status":"ready"' "$agent_work/workflow-status.json"

task_json="$(
  "$binary" --workspace "$workspace" task prepare \
    --job "$job_id" --operation job-parse --json
)"
task_id="$(first_uuid_id "$task_json")"

set +e
"$binary" --workspace "$workspace" task inputs "$task_id" \
  --destination "$agent_work/inputs" --json >"$agent_work/consent-required.json"
consent_status=$?
set -e
test "$consent_status" -eq 3
grep -q '"code":"consent.required"' "$agent_work/consent-required.json"
test ! -e "$agent_work/inputs"

"$binary" --workspace "$workspace" task inputs "$task_id" \
  --destination "$agent_work/inputs" --allow-private-read --json \
  >"$agent_work/input-export.json"
test -f "$agent_work/inputs/canisend-task-inputs.json"
grep -q "Lecturer in Economics" "$agent_work"/inputs/inputs/*.txt
source_quote="Evidence of effective university-level teaching."
input_file="$(find "$agent_work/inputs/inputs" -type f -name '*.txt' -print -quit)"
source_start="$(LC_ALL=C grep -boF "$source_quote" "$input_file" | sed -n '1s/:.*//p')"
test -n "$source_start"
source_end=$((source_start + ${#source_quote}))

write_completion "$task_json" " " "$agent_work/invalid-completion.json"
set +e
"$binary" --workspace "$workspace" task complete \
  --file "$agent_work/invalid-completion.json" --json \
  >"$agent_work/validation-failure.json"
validation_status=$?
set -e
test "$validation_status" -eq 3
grep -q '"code":"candidate.semantic_invalid"' "$agent_work/validation-failure.json"
grep -q '"remediation"' "$agent_work/validation-failure.json"

write_completion "$task_json" "Evidence of effective university-level teaching" \
  "$agent_work/completion.json"
"$binary" --workspace "$workspace" task complete \
  --file "$agent_work/completion.json" --json >"$agent_work/committed.json"
grep -q '"idempotent":false' "$agent_work/committed.json"
"$binary" --workspace "$workspace" task complete \
  --file "$agent_work/completion.json" --json >"$agent_work/replayed.json"
grep -q '"idempotent":true' "$agent_work/replayed.json"

"$binary" --workspace "$workspace" criteria export --job "$job_id" \
  --destination "$agent_work/criteria.json" --json >"$agent_work/criteria-export.json"
"$binary" --workspace "$workspace" criteria confirm --job "$job_id" \
  --file "$agent_work/criteria.json" --json >"$agent_work/criteria-confirm.json"
grep -q '"status":"confirmed"' "$agent_work/criteria-confirm.json"
grep -q '"confirmed":true' "$agent_work/criteria-confirm.json"

"$binary" --workspace "$workspace" profile source add \
  --file "$repo_root/fixtures/v2-spec/profile-evidence.json" --json \
  >"$agent_work/profile-source.json"
evidence_task_json="$(
  "$binary" --workspace "$workspace" task prepare \
    --job "$job_id" --operation evidence-normalize --json
)"
evidence_task_id="$(first_uuid_id "$evidence_task_json")"
"$binary" --workspace "$workspace" task inputs "$evidence_task_id" \
  --destination "$agent_work/evidence-inputs" --allow-private-read --json \
  >"$agent_work/evidence-input-export.json"
evidence_quote="PhD in Economics awarded by Example University in 2024"
evidence_input_file="$(find "$agent_work/evidence-inputs/inputs" -type f -name '*.txt' -print -quit)"
evidence_start="$(LC_ALL=C grep -boF "$evidence_quote" "$evidence_input_file" | sed -n '1s/:.*//p')"
test -n "$evidence_start"
evidence_end=$((evidence_start + ${#evidence_quote}))
write_evidence_completion "$evidence_task_json" "$agent_work/evidence-completion.json"
"$binary" --workspace "$workspace" task complete \
  --file "$agent_work/evidence-completion.json" --json \
  >"$agent_work/evidence-committed.json"
"$binary" --workspace "$workspace" profile evidence proposed --job "$job_id" --json \
  >"$agent_work/evidence-proposed.json"
grep -q '"confirmed":false' "$agent_work/evidence-proposed.json"
"$binary" --workspace "$workspace" profile evidence export --job "$job_id" \
  --destination "$agent_work/evidence-decision.json" --json \
  >"$agent_work/evidence-export.json"
"$binary" --workspace "$workspace" profile evidence confirm --job "$job_id" \
  --file "$agent_work/evidence-decision.json" --json \
  >"$agent_work/evidence-confirm.json"
grep -q '"status":"confirmed"' "$agent_work/evidence-confirm.json"
grep -q '"confirmed":true' "$agent_work/evidence-confirm.json"
"$binary" --workspace "$workspace" profile evidence show --job "$job_id" --json \
  >"$agent_work/evidence-show.json"
evidence_item_id="$(
  grep -oE '"items":\[\{"confirmed":true,"excluded":false,"id":"[0-9a-fA-F-]{36}"' \
    "$agent_work/evidence-confirm.json" | cut -d'"' -f10
)"
test -n "$evidence_item_id"

match_task_json="$(
  "$binary" --workspace "$workspace" task prepare \
    --job "$job_id" --operation evidence-match --json
)"
match_task_id="$(first_uuid_id "$match_task_json")"
"$binary" --workspace "$workspace" task inputs "$match_task_id" \
  --destination "$agent_work/match-inputs" --allow-private-read --json \
  >"$agent_work/match-input-export.json"
write_match_completion "$match_task_json" "$agent_work/match-completion.json"
"$binary" --workspace "$workspace" task complete \
  --file "$agent_work/match-completion.json" --json \
  >"$agent_work/match-committed.json"
"$binary" --workspace "$workspace" match show --job "$job_id" --json \
  >"$agent_work/match-show.json"
grep -q '"strength":"partial"' "$agent_work/match-show.json"
grep -q '"prohibited_claims":\[' "$agent_work/match-show.json"
"$binary" --workspace "$workspace" workflow status --job "$job_id" --json \
  >"$agent_work/workflow-after-match.json"
grep -q '"stage":"plan","status":"ready"' "$agent_work/workflow-after-match.json"

"$binary" --workspace "$workspace" plan export --job "$job_id" \
  --destination "$agent_work/application-plan.json" --json \
  >"$agent_work/plan-export.json"
grep -q '"decision": "hold"' "$agent_work/application-plan.json"
grep -q '"severity": "blocking"' "$agent_work/application-plan.json"
"$binary" --workspace "$workspace" plan confirm --job "$job_id" \
  --file "$agent_work/application-plan.json" --json >"$agent_work/plan-confirm.json"
grep -q '"status":"confirmed"' "$agent_work/plan-confirm.json"
"$binary" --workspace "$workspace" plan show --job "$job_id" --json \
  >"$agent_work/plan-show.json"
grep -q '"decision":"hold"' "$agent_work/plan-show.json"
"$binary" --workspace "$workspace" workflow status --job "$job_id" --json \
  >"$agent_work/workflow-after-plan.json"
grep -q '"stage":"plan","status":"complete"' "$agent_work/workflow-after-plan.json"
grep -q '"stage":"draft","status":"blocked"' "$agent_work/workflow-after-plan.json"
grep -q '"code":"workflow.plan_blocked"' "$agent_work/workflow-after-plan.json"

"$binary" --workspace "$workspace" workflow rerun --job "$job_id" \
  --stage parse --json >"$agent_work/parse-rerun.json"

stale_task_json="$(
  "$binary" --workspace "$workspace" task prepare \
    --job "$job_id" --operation job-parse --json
)"
stale_task_id="$(first_uuid_id "$stale_task_json")"
write_completion "$stale_task_json" "Evidence of effective university-level teaching" \
  "$agent_work/stale-completion.json"
"$binary" --workspace "$workspace" job import "$job_id" \
  --file "$repo_root/fixtures/v2-spec/job-advert.md" --json >/dev/null
set +e
"$binary" --workspace "$workspace" task complete \
  --file "$agent_work/stale-completion.json" --json \
  >"$agent_work/stale.json"
stale_status=$?
set -e
test "$stale_status" -eq 4
grep -q '"code":"task.stale"' "$agent_work/stale.json"
"$binary" --workspace "$workspace" task show "$stale_task_id" --json \
  >"$agent_work/stale-task.json"
grep -q '"status":"stale"' "$agent_work/stale-task.json"

"$binary" --workspace "$workspace" workspace check --json >"$agent_work/workspace-check.json"
grep -q '"ok":true' "$agent_work/workspace-check.json"

echo "host-agent smoke: ok"
