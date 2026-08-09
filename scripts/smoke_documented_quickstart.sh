#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/canisend}"
smoke_root="${2:-${TMPDIR:-/tmp}/canisend-documentation-smoke}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
source "$script_dir/lib/native_paths.sh"
binary="$(canisend_absolute_path "$binary")"
smoke_root="$(canisend_absolute_path "$smoke_root")"

if [[ ! -x "$binary" || -L "$binary" ]]; then
  echo "documentation smoke: binary is not an executable regular file: $binary" >&2
  exit 1
fi
if [[ -e "$smoke_root" || -L "$smoke_root" ]]; then
  echo "documentation smoke: destination must not exist: $smoke_root" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "documentation smoke: required command is missing: jq" >&2
  exit 1
fi

mkdir -p "$smoke_root/candidates"
workspace="$smoke_root/applications"
backup="$smoke_root/applications-backup"
restored="$smoke_root/applications-restored"

"$binary" version --json > "$smoke_root/version.json"
jq -e '.ok == true and .operation == "product.version"' \
  "$smoke_root/version.json" >/dev/null
"$binary" doctor --json > "$smoke_root/doctor.json"
jq -e '
  .ok == true
  and .data.python_required == false
  and .data.embedded_typst == "verified"
  and .data.runtime_package_downloads == false
' "$smoke_root/doctor.json" >/dev/null

"$binary" --help > "$smoke_root/help.txt"
grep -q 'Evidence-backed application preparation' "$smoke_root/help.txt"
"$binary" application create --help > "$smoke_root/application-create-help.txt"
grep -q 'Pack-bound Application' "$smoke_root/application-create-help.txt"
"$binary" profile-source import --help > "$smoke_root/profile-source-import-help.txt"
grep -q 'Workspace v4 authority' "$smoke_root/profile-source-import-help.txt"
"$binary" workspace repair --help > "$smoke_root/workspace-repair-help.txt"
grep -q 'projection' "$smoke_root/workspace-repair-help.txt"

"$binary" --workspace "$workspace" workspace init --json \
  > "$smoke_root/workspace-init.json"
jq -e '
  .ok == true
  and .data.workspace_format == "canisend.workspace/v4"
  and .data.application_count == 0
' "$smoke_root/workspace-init.json" >/dev/null

generic_source='Applicants must provide a reviewed project narrative.'
jq -n \
  --arg source_text "$generic_source" \
  '{
    title: "Synthetic programme application",
    opportunity_metadata: {
      organization: {type: "short-text", value: "Example Foundation"}
    },
    application_metadata: {},
    source_text: $source_text,
    requirements: [{
      category: "format",
      statement: $source_text,
      priority: "mandatory",
      start_byte: 0,
      end_byte: ($source_text | utf8bytelength)
    }]
  }' > "$smoke_root/candidates/generic.json"

academic_source='Applicants must provide an academic CV.'
jq -n \
  --arg source_text "$academic_source" \
  '{
    title: "Synthetic research fellowship",
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
  }' > "$smoke_root/candidates/academic.json"

"$binary" --workspace "$workspace" application create \
  --pack org.canisend.generic-application \
  --candidate "$smoke_root/candidates/generic.json" \
  --json > "$smoke_root/generic-create.json"
"$binary" --workspace "$workspace" application create \
  --pack org.canisend.academic-job \
  --candidate "$smoke_root/candidates/academic.json" \
  --json > "$smoke_root/academic-create.json"

generic_id="$(jq -er '.data.stored.snapshot.application.id' "$smoke_root/generic-create.json")"
academic_id="$(jq -er '.data.stored.snapshot.application.id' "$smoke_root/academic-create.json")"
"$binary" --workspace "$workspace" application list --json \
  > "$smoke_root/application-list.json"
jq -e '
  .ok == true
  and (.data | length) == 2
  and ([.data[].snapshot.pack.id] | sort) == [
    "org.canisend.academic-job",
    "org.canisend.generic-application"
  ]
' "$smoke_root/application-list.json" >/dev/null
"$binary" --workspace "$workspace" application show \
  --application "$generic_id" \
  --json > "$smoke_root/generic-show.json"
"$binary" --workspace "$workspace" application show \
  --application "$academic_id" \
  --json > "$smoke_root/academic-show.json"
jq -e '.data.snapshot.pack.id == "org.canisend.generic-application"' \
  "$smoke_root/generic-show.json" >/dev/null
jq -e '.data.snapshot.pack.id == "org.canisend.academic-job"' \
  "$smoke_root/academic-show.json" >/dev/null

printf '%s\n' \
  '# Synthetic Profile Source' \
  '' \
  'Managed a reviewed cross-domain programme.' \
  > "$smoke_root/candidates/profile.md"
"$binary" --workspace "$workspace" profile-source import \
  "$smoke_root/candidates/profile.md" \
  --sensitivity private-local \
  --confirm-private-read \
  --json > "$smoke_root/profile-source-import.json"
"$binary" --workspace "$workspace" profile-source list --json \
  > "$smoke_root/profile-source-list.json"
jq -e '
  .ok == true
  and (.data.sources | length) == 1
  and .data.sources[0].sensitivity == "private-local"
' "$smoke_root/profile-source-list.json" >/dev/null
if grep -q 'Managed a reviewed cross-domain programme' \
  "$smoke_root/profile-source-list.json"; then
  echo "documentation smoke: Profile Source listing leaked a private body" >&2
  exit 1
fi

"$binary" --workspace "$workspace" workspace check --json \
  > "$smoke_root/workspace-check.json"
jq -e '.ok == true and .data.ok == true' \
  "$smoke_root/workspace-check.json" >/dev/null
"$binary" --workspace "$workspace" workspace backup "$backup" --json \
  > "$smoke_root/workspace-backup.json"
"$binary" workspace restore "$backup" "$restored" --json \
  > "$smoke_root/workspace-restore.json"
"$binary" --workspace "$restored" workspace repair --json \
  > "$smoke_root/workspace-repair.json"
"$binary" --workspace "$restored" workspace check --json \
  > "$smoke_root/restored-check.json"
jq -e '.ok == true and .data.ok == true' \
  "$smoke_root/restored-check.json" >/dev/null
"$binary" --workspace "$restored" application list --json \
  | jq -e '.ok == true and (.data | length) == 2' >/dev/null

echo "documentation smoke: ok (one Workspace, two Packs, basic data, backup, and restore)"
