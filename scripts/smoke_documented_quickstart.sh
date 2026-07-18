#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/canisend}"
smoke_root="${2:-${TMPDIR:-/tmp}/canisend-documentation-smoke}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"

case "$binary" in
  /*) ;;
  *) binary="$PWD/$binary" ;;
esac

if [[ ! -x "$binary" ]]; then
  echo "documentation smoke: binary is not executable: $binary" >&2
  exit 1
fi
if [[ -e "$smoke_root" ]]; then
  echo "documentation smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi

mkdir -p "$smoke_root"
workspace="$smoke_root/applications"
backup="$smoke_root/applications-backup"
restored="$smoke_root/applications-restored"
host_pack="$smoke_root/canisend-codex-pack"
cd "$smoke_root"

"$binary" version --json >version.json
grep -q '"workspace_format":"canisend.workspace/v2"' version.json
"$binary" doctor --json >doctor.json
grep -q '"python_required":false' doctor.json
grep -q '"embedded_typst":"verified"' doctor.json
grep -q '"runtime_package_downloads":false' doctor.json

"$binary" --help >help.txt
grep -q 'Evidence-backed application preparation' help.txt
"$binary" job import --help >job-import-help.txt
grep -q 'user-supplied public URL' job-import-help.txt
"$binary" workspace repair --help >workspace-repair-help.txt
grep -q 'preserving user edits' workspace-repair-help.txt

"$binary" agent assets export --host codex --destination "$host_pack" --json >host-pack.json
test -f "$host_pack/AGENTS.md"
test -f "$host_pack/canisend-agent-pack.json"

"$binary" --workspace "$workspace" workspace init --json >workspace-init.json
job_json="$(
  "$binary" --workspace "$workspace" job create \
    --title "Lecturer in Economics" \
    --institution "University X" \
    --json
)"
job_id="$(printf '%s' "$job_json" | sed -E 's/.*"id":"([0-9a-fA-F-]{36})".*/\1/')"
test -n "$job_id"

"$binary" --workspace "$workspace" job import "$job_id" \
  --file "$repo_root/fixtures/v2-spec/job-advert.md" --json >job-import.json
"$binary" --workspace "$workspace" job show "$job_id" --json >job-show.json
"$binary" --workspace "$workspace" workflow start --job "$job_id" --json >workflow-start.json
"$binary" --workspace "$workspace" workflow status --job "$job_id" --json >workflow-status.json
grep -q '"stage":"parse","status":"ready"' workflow-status.json

"$binary" --workspace "$workspace" workspace check --json >workspace-check.json
grep -q '"ok":true' workspace-check.json
"$binary" --workspace "$workspace" workspace backup "$backup" --json >workspace-backup.json
"$binary" workspace restore "$backup" "$restored" --json >workspace-restore.json
"$binary" --workspace "$restored" workspace repair --json >workspace-repair.json
"$binary" --workspace "$restored" workspace check --json >restored-check.json
grep -q '"ok":true' restored-check.json

echo "documentation smoke: ok"
