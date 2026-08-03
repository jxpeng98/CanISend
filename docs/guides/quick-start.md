# Quick start: choose one exact Pack

This guide describes the post-`v1.0.0-alpha.5` source contract. A downloaded Alpha.5 binary may not
contain every command below; verify its version and release notes first. The two paths use separate
Workspaces because Pack identity is an authority boundary, not a preference.

## 1. Verify the binary

```console
canisend version --json
canisend doctor --json
```

Keep every Workspace in a user-controlled private directory. Do not initialize one inside a public
repository or a directory synchronized without appropriate encryption and access control.

## 2. Choose a path before initialization

Use the Generic Pack for grants, admissions, tenders, professional roles, and other evidence-bound
Applications:

```console
canisend --workspace ./generic-applications workspace init \
  --pack generic-application --json
```

This creates a Workspace v3 authority bound to
`org.canisend.generic-application`. `--pack generic-application` is the default, but keeping it in
scripts makes the decision auditable.

Use the academic reference journey when you need the established job-source, Profile, matching,
four-document, and workflow controls:

```console
canisend --workspace ./academic-applications workspace init \
  --pack academic-job --json
```

This creates the academic Workspace v2 compatibility authority bound to
`org.canisend.academic-job`. Generic v3 operations fail closed in it; Academic `job`, `profile`,
`task`, and workflow mutations fail closed in a Generic Workspace.

## 3A. Complete a Generic v3 Application

The current generic CLI accepts a user-reviewed bounded JSON request. Direct URL, PDF, and local
document normalization into that request is not yet implemented. Save the following as
`create.json`:

```json
{
  "title": "Synthetic application",
  "opportunity_metadata": {},
  "application_metadata": {},
  "source_text": "Narrative required.",
  "requirements": [
    {
      "category": "format",
      "statement": "Narrative required.",
      "priority": "mandatory",
      "start_byte": 0,
      "end_byte": 19
    }
  ]
}
```

Create the Application and copy its UUID and current revision from the JSON response:

```console
canisend --workspace ./generic-applications application generic-create \
  --candidate ./create.json --json
canisend --workspace ./generic-applications application v3-list --json
canisend --workspace ./generic-applications application v3-show \
  --application APPLICATION_ID --json
```

Review every Requirement against the source span. Save a Plan candidate as `plan.json`, replacing
`expected_revision` with the current revision returned above:

```json
{
  "expected_revision": 1,
  "decision": "proceed",
  "deliverables": [
    {
      "kind": "primary-document",
      "disposition": "required",
      "rationale": "Required by the reviewed source",
      "constraints": ["Use confirmed local evidence only"],
      "execution_mode": "manual-import"
    }
  ]
}
```

```console
canisend --workspace ./generic-applications application generic-plan \
  --application APPLICATION_ID --candidate ./plan.json --json
```

Save reviewed material as `compose.json`, again using the revision returned by the preceding
mutation:

```json
{
  "expected_revision": 2,
  "deliverables": [
    {
      "kind": "primary-document",
      "title": "Application narrative",
      "media_type": "text/markdown",
      "content": "# Application narrative\n\nReviewed content."
    }
  ]
}
```

```console
canisend --workspace ./generic-applications application generic-compose \
  --application APPLICATION_ID --candidate ./compose.json --json
canisend --workspace ./generic-applications application v3-show \
  --application APPLICATION_ID --json
```

Read the private Deliverable bodies and approve them only after human review. The CLI approval is
revision-bound; an Agent v3/MCP approval additionally consumes a single-use review token:

```console
canisend --workspace ./generic-applications application generic-approve \
  --application APPLICATION_ID --expected-revision CURRENT_REVISION --json
canisend --workspace ./generic-applications application generic-export \
  --application APPLICATION_ID --expected-revision CURRENT_REVISION \
  --destination applications/APPLICATION_ID/exports/first \
  --allow-private-export --json
```

Use the revision returned by `generic-approve` for export. The destination must be a safe relative
path within the Workspace. A successful receipt always reports `submission_performed: false`.

Four complete fictional Pack examples are described in
[Generic Application synthetic examples](../testing/generic-application-examples.md).

## 3B. Complete the Academic reference intake

Create a job and copy its ID:

```console
canisend --workspace ./academic-applications job create \
  --title "Lecturer in Economics" --institution "University X" --json
```

Import reviewed Markdown, UTF-8 text, or a text-based PDF:

```console
canisend --workspace ./academic-applications job import JOB_ID \
  --file ./job-advert.md --json
canisend --workspace ./academic-applications job import JOB_ID \
  --file ./person-specification.pdf --json
```

A user-supplied public HTTP(S) URL is also supported. Fetches reject credentials, non-public
destinations, unsafe redirects, misleading content types, and oversized responses:

```console
canisend --workspace ./academic-applications job import JOB_ID \
  --url https://jobs.example.edu/vacancy/123 --json
```

Scanned or image-only PDFs are unsupported. Review output from a separately trusted OCR tool and
import it as text; never treat unreviewed OCR as authoritative evidence.

```console
canisend --workspace ./academic-applications job show JOB_ID --json
canisend --workspace ./academic-applications workflow start --job JOB_ID --json
canisend --workspace ./academic-applications workflow status --job JOB_ID --json
```

Continue through criteria confirmation, Evidence, matching, the apply/hold/skip decision,
materials, review, rendering, and local export. CanISend never performs portal submission.

## 4. Existing Workspace v2: preview migration before mutation

Do not run a Generic command against an existing Academic Workspace to “upgrade” it. First stop all
writers, check it, and request the body-free migration plan:

```console
canisend --workspace ./academic-applications workspace check --json
canisend --workspace ./academic-applications workspace migration-preview --json
```

Review the exact Pack binding, counts, required backup size, and `migration_plan_sha256`. Then use
that digest and a new backup destination:

```console
canisend --workspace ./academic-applications workspace migrate \
  --expected-plan-sha256 MIGRATION_PLAN_SHA256 \
  --backup-destination ./backups/academic-before-v3 --json
```

Migration creates and verifies the backup before commit, rejects a stale plan, preserves legacy
compatibility authority, and keeps `org.canisend.academic-job`. It does not turn academic records
into `org.canisend.generic-application` records. To start a Generic journey, create a separate
Generic Workspace.

## 5. Finish every session safely

```console
canisend --workspace ./generic-applications workspace check --json
canisend --workspace ./generic-applications workspace backup \
  ./generic-applications-backup --json
```

Use a new or empty backup destination. See [Backup and recovery](backup-and-recovery.md) before
moving, restoring, or repairing an important Workspace.
