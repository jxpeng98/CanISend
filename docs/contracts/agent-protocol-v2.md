# CanISend Agent Protocol v2

**Protocol identifier:** `canisend.agent/v2`

**Schema version:** `2.0.0`

**Transport:** local command invocation with JSON on stdout

## Output boundary

Commands invoked with `--json` emit exactly one JSON object followed by one newline on stdout. When stdout is not a
terminal, agent-capable commands select JSON automatically. Diagnostics and progress belong on stderr and never share
the JSON stream. Successful JSON commands do not write stderr.

The response envelope has this stable shape:

```json
{
  "protocol": "canisend.agent/v2",
  "operation": "agent.context",
  "ok": true,
  "status": "available",
  "data": {},
  "artifacts": [],
  "required_consents": [],
  "warnings": [],
  "next_actions": [],
  "error": null
}
```

An error sets `ok` to `false`, sets `data` to `null`, and provides a safe `error` object. Errors must not copy job
advert bodies, evidence text, drafts, credentials, or provider payloads into the envelope.

## Safe references and tasks

Artifact references contain only an artifact kind, UUIDv7 identity, positive revision, and lowercase SHA-256 digest.
They contain no filesystem path or private body. Task descriptors declare:

- task identity and lease expiry;
- subject job identity and exact job revision;
- actor and execution mode;
- exact input artifact revisions;
- one allowed output artifact kind;
- candidate schema ID and semantic version;
- required consent scopes;
- the exact private-read scope.

Task completion repeats the expected job revision and every expected input revision/hash, then carries the candidate
JSON. The runtime validates the generated Draft 2020-12 schema first, then Rust deserialization and semantic rules.
A failed validation cannot be treated as an authoritative state transition. Completion rechecks the lease, job
revision, and artifact heads in an immediate SQLite transaction; a matching replay returns the same artifact, while
changed context becomes `task.stale`.

The available task lifecycle is:

```text
canisend --workspace WORKSPACE task prepare --job JOB_ID --operation job-parse --json
canisend --workspace WORKSPACE task show TASK_ID --json
canisend --workspace WORKSPACE task inputs TASK_ID --destination DIRECTORY \
  --allow-private-read --json
canisend --workspace WORKSPACE task complete --file COMPLETION.json --json
canisend --workspace WORKSPACE task complete --stdin --json
canisend --workspace WORKSPACE task cancel TASK_ID --json
```

`task inputs` requires explicit `read-private-inputs` confirmation and exports only the descriptor's declared scope
to a new or empty external directory. Candidate files are capped at 4 MiB; file input must be a regular, non-symlink
`.json` file. Hosts never need filesystem access to `.canisend/`.

The `job.parse` task returns an unconfirmed `canisend.parsed-job/v2` candidate. Every proposed criterion carries an
exact normalized-source artifact reference, UTF-8 byte span, quote, and confidence. Host-agent and
configured-provider modes share the same validator; provider mode additionally requires explicit
`send-to-configured-provider` consent. User correction and confirmation remain a separate boundary:

```text
canisend --workspace WORKSPACE criteria proposed --job JOB_ID --json
canisend --workspace WORKSPACE criteria export --job JOB_ID --destination criteria.json --json
canisend --workspace WORKSPACE criteria confirm --job JOB_ID --file criteria.json --json
canisend --workspace WORKSPACE criteria show --job JOB_ID --json
```

## Package and projection lifecycle

The deterministic Package stage freezes exact plan, match, evidence, profile, document-set, member-document, and
review revisions. Readiness is body-free and can be `blocked`, `needs-review`, or `ready-to-export`; it is never a
submission signal. The export lifecycle is:

```text
canisend --workspace WORKSPACE package check --job JOB_ID --json
canisend --workspace WORKSPACE package show --job JOB_ID --json
canisend --workspace WORKSPACE package export --job JOB_ID \
  --destination jobs/JOB_ID/application --allow-private-export --json
canisend --workspace WORKSPACE package exports --job JOB_ID --json
canisend --workspace WORKSPACE package reconcile --job JOB_ID --json
canisend --workspace WORKSPACE package replace --job JOB_ID \
  --path jobs/JOB_ID/application/cover-letter.md --json
canisend --workspace WORKSPACE package copy-as-new --job JOB_ID \
  --path jobs/JOB_ID/application/cover-letter.md \
  --destination jobs/JOB_ID/application/cover-letter-edited.md --json
```

`package export` requires explicit `export-private-artifacts` confirmation before opening a workspace. It writes
editable Markdown, structured JSON, and self-contained escaped Typst source for every current document plus
`package-manifest.json`, all under the exact job projection tree. The export receipt binds the current package and
every projection to generated and observed SHA-256 hashes and always records `submission_performed: false`.

Typst sources are generated only from validated structured documents and the embedded application-document template.
Quotes, backslashes, line controls, Unicode, and Typst-like input text remain string data. Any unresolved document
placeholder stops source generation. The `.typ` file is an editable projection and never becomes an authoritative
document merely because a user or agent edits it.

Structured artifact blobs remain authoritative. `package reconcile` records a managed projection as `current`,
`edited`, `missing`, or `repair-required` without importing file edits. Re-export refuses both edited managed files
and unmanaged destination collisions. `package replace` explicitly discards one edit; `package copy-as-new` first
preserves edited bytes at a new unmanaged safe path and then restores the generated form. Neither operation changes
the authoritative structured document.

## Data and consent types

Data classifications are `public`, `private-local`, `provider-bound`, and `secret`. Provider-bound use is limited to
the artifact revisions explicitly approved for an operation. Defined consent scopes are:

- `read-private-inputs`
- `send-to-configured-provider`
- `fetch-user-supplied-url`
- `export-private-artifacts`
- `use-system-fonts`

The protocol never treats readiness as evidence that an application was submitted.

## Error registry

Error codes are stable within protocol v2 even when human messages improve:

| Code | Exit group |
|---|---:|
| `input.invalid` | 3 |
| `input.path_rejected` | 3 |
| `workspace.not_found` | 4 |
| `workspace.conflict` | 4 |
| `job.not_found` | 4 |
| `job.archived` | 4 |
| `profile.source_not_found` | 4 |
| `discovery.source_not_found` | 4 |
| `discovery.lead_not_found` | 4 |
| `discovery.conflict` | 4 |
| `pdf.encrypted` | 3 |
| `pdf.malformed` | 3 |
| `pdf_text_unavailable` | 3 |
| `resource.not_found` | 3 |
| `resources.integrity_failed` | 6 |
| `schema.not_found` | 3 |
| `candidate.schema_invalid` | 3 |
| `candidate.semantic_invalid` | 3 |
| `candidate.unknown_evidence` | 3 |
| `task.not_found` | 4 |
| `task.stale` | 4 |
| `task.conflict` | 4 |
| `consent.required` | 3 |
| `external.io_failed` | 5 |
| `provider.failed` | 5 |
| `internal.invariant_failed` | 6 |

Process exit groups are `0` for success, `2` for CLI usage, `3` for validation or workflow blockers, `4` for state
conflicts, `5` for external I/O or providers, and `6` for an internal invariant failure.

## Discovery commands

An agent host should begin with:

```text
canisend doctor --json
canisend agent capabilities --json
canisend agent context --json
canisend schema list --json
canisend resource list --json
```

`agent capabilities` distinguishes `available` work from `planned` work and publishes the error-code registry.
`doctor` performs a body-free, workspace-free render of the embedded Cover Letter template and reports whether the
in-process Typst compiler, embedded default fonts, resource manifest, and schemas are usable. Its default renderer
self-check does not scan system fonts or enable runtime Typst package downloads.
`schema list` reports canonical schema IDs, versions, resource IDs, sizes, and SHA-256 digests. `resource list`
reports all compiled host guides, examples, prompts, schemas, and templates. The embedded Codex, Claude, and generic
guides instruct hosts never to edit `.canisend/` state directly.

Self-contained host packs are exported without a workspace:

```text
canisend agent assets export --host codex --destination DIRECTORY --json
canisend agent assets export --host claude --destination DIRECTORY --json
canisend agent assets export --host generic --destination DIRECTORY --json
```

Each 29-file pack contains its host entrypoint, task prompts and examples, and the contracts required through package
export, including the projection, reconciliation, and export-manifest schemas. `canisend-agent-pack.json` records
pack, product, protocol, and resource versions plus every resource ID, path, size, and SHA-256 digest.

Job intake is available through `job create`, `job import JOB_ID --file PATH`, `job import JOB_ID --url URL`,
`job list`, `job show`, and `job archive`. Import success returns source and artifact references without returning the
private source body. A URL flag is an explicit user-requested fetch; redirects remain subject to the same public
address policy as the initial URL.

Discovery is available through:

```text
canisend discovery adapters --json
canisend discovery import --file BATCH.csv --source-name NAME --dry-run --json
canisend --workspace WORKSPACE discovery import --file BATCH.json --json
canisend --workspace WORKSPACE discovery import --file AGENT.json --host-agent --json
canisend --workspace WORKSPACE discovery refresh --adapter ADAPTER --endpoint URL --source-name NAME --json
canisend --workspace WORKSPACE discovery sources --json
canisend --workspace WORKSPACE discovery list --include-history --json
canisend --workspace WORKSPACE discovery show LEAD_ID --json
canisend --workspace WORKSPACE discovery suggest LEAD_ID --limit 5 --json
canisend --workspace WORKSPACE discovery promote LEAD_ID --json
```

CSV mapping requires `title`, `organization`, and `url`; it accepts explicit optional fields and bounded `meta.*`
extensions. JSON and host-agent imports use `canisend.discovery-batch/v2`. `--dry-run` performs no workspace access.
Network refresh is user-invoked and adapter-bound; it is not a crawler. Promotion creates a job record and returns a
next action for `job import JOB_ID --url URL`, keeping advert retrieval inside the direct-intake consent boundary.

## Contract generation

Rust types in `canisend-contracts` are authoritative. Thirty-eight public schemas are generated with canonical IDs under
`https://schemas.canisend.dev/v2/`, semantic version `2.0.0`, deterministic key ordering, and a final newline.
`cargo run -p xtask -- schemas check` rejects byte drift, missing files, and additional schema files.

All compiled resources are declared in one manifest. The build fails for a missing, duplicate, undeclared, unsafe,
or symlinked resource. The generated resource catalog embeds each file and records its typed ID, kind, version, byte
size, and SHA-256 digest.
