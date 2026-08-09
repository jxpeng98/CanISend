# Quick start: one Workspace, independently Pack-bound Applications

This guide describes the checked-in `1.0.0-alpha.7` clean Workspace v4 and Agent v4 development
contract. The latest public Alpha.6 remains the historical Workspace v3 checkpoint and does not
contain this flow. Treat Alpha.7 as unpublished until exact release artifacts pass qualification.

CanISend is local-first. Keep every Workspace in a user-controlled private directory. It never
logs in, uploads, or submits an Application; every successful export receipt keeps
`submission_performed: false`.

## 1. Verify the binary and initialize once

```console
canisend version --json
canisend doctor --json
canisend --workspace ./my-applications workspace init --json
canisend --workspace ./my-applications workspace status --json
```

Initialization creates a neutral `canisend.workspace/v4` authority. It creates no Profile,
private body, Application, or academic/generic mode. A Workspace may contain any number of
Applications, but each Application owns one exact Pack ID, version, and content digest.

## 2. Create a generic Application

Save a reviewed, bounded candidate as `generic.json`:

```json
{
  "title": "Community programme application",
  "opportunity_metadata": {
    "organization": { "type": "short-text", "value": "Example Foundation" }
  },
  "application_metadata": {},
  "source_text": "Applicants must provide a reviewed project narrative.",
  "requirements": [
    {
      "category": "format",
      "statement": "Applicants must provide a reviewed project narrative.",
      "priority": "mandatory",
      "start_byte": 0,
      "end_byte": 53
    }
  ]
}
```

Create it with the exact built-in Pack:

```console
canisend --workspace ./my-applications application create \
  --pack org.canisend.generic-application \
  --candidate ./generic.json --json
```

The Generic Pack supports professional roles, grants and fellowships, admissions, tenders and
proposals, internal dossiers, and other evidence-bound submissions. Pack metadata and
Deliverable kinds describe the domain; the kernel remains neutral.

## 3. Create an academic Application in the same Workspace

Save an academic candidate as `academic.json`:

```json
{
  "title": "Research fellowship",
  "opportunity_metadata": {
    "institution": { "type": "short-text", "value": "Example University" }
  },
  "application_metadata": {},
  "source_text": "Applicants must provide an academic CV.",
  "requirements": [
    {
      "category": "qualification",
      "statement": "Applicants must provide an academic CV.",
      "priority": "mandatory",
      "start_byte": 0,
      "end_byte": 39
    }
  ]
}
```

```console
canisend --workspace ./my-applications application create \
  --pack org.canisend.academic-job \
  --candidate ./academic.json --json
canisend --workspace ./my-applications application list --json
```

The list must contain both exact Pack identities. Creating, archiving, or changing either
Application does not relabel or clear the other.

## 4. Add minimum shared data

Import a reviewed Profile Source. Private-local files require explicit read consent:

```console
canisend --workspace ./my-applications profile-source import ./profile.md \
  --sensitivity private-local --confirm-private-read --json
canisend --workspace ./my-applications profile-source list --json
```

The list receipt is body-free. A Profile Source or Evidence record is not visible to an
Application merely because it exists in the same Workspace. Use the Application-scoped
association lists, then review and commit an exact association through one MCP session:

```console
canisend --workspace ./my-applications profile association list \
  --application APPLICATION_ID --json
canisend --workspace ./my-applications evidence association list \
  --application APPLICATION_ID --json
```

## 5. Install new Agent v4 host resources

Codex and Claude Code use the same generated Skills and MCP operation registry:

```console
canisend --workspace ./my-applications host setup --host codex --json
canisend --workspace ./my-applications host status --host codex --json
canisend --workspace ./my-applications host setup --host claude --json
canisend --workspace ./my-applications host status --host claude --json
```

Setup installs only manifest-owned v4 Skills and returns deterministic MCP registration guidance.
It does not silently edit the host's global configuration. See [Agent integration](agent-integration.md)
for the exact registration snippets and tool sequence.

## 6. Continue through one guarded MCP session

The standalone CLI provides initialization, recovery, host management, basic data, and body-free
Application reads. Independent CLI processes do not exchange in-memory approval tokens. Keep one
MCP stdio process alive for guarded mutations:

```console
canisend --workspace ./my-applications mcp serve
```

For each selected Application, the canonical flow is:

```text
orient -> propose -> preview -> approve -> commit -> verify
```

Use `requirement.extract`, `requirement.confirm`, `plan.propose`, `plan.confirm`,
`deliverable.draft`, `deliverable.audit`, review, and export operations with the exact Workspace,
Application, Pack, revision, snapshot digest, preview token, and required consent. A denied,
expired, replayed, stale, or wrong-context token fails without mutation. The App may be closed;
reopening it reads the same SQLite authority and receipts.

## 7. Check, back up, and restore

```console
canisend --workspace ./my-applications workspace check --json
canisend --workspace ./my-applications workspace backup \
  ./my-applications-backup --json
canisend workspace restore ./my-applications-backup \
  ./my-applications-restored --json
canisend --workspace ./my-applications-restored workspace check --json
```

Backup and restore preserve both Application Pack bindings, immutable Blobs, explicit
associations, audit history, and revisions. Restore always targets a new or empty directory.

## Unsupported legacy boundary

Alpha.7 does not support Alpha.6-or-earlier Skills, Agent v2/v3 requests, job-specific aliases,
host-resource layouts, or Workspace v2/v3 migration. Unsupported inputs fail before mutation and
direct the user to initialize a clean Workspace v4. Historical tagged release documentation
remains available for reproducing the corresponding public checkpoint.
