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

"$binary" agent capabilities --json >/dev/null
"$binary" agent assets export --host codex --destination "$pack" --json >/dev/null
test -f "$pack/AGENTS.md"
test -f "$pack/canisend-agent-pack.json"

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
