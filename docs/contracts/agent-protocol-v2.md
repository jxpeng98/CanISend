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
- actor and execution mode;
- exact input artifact revisions;
- one allowed output artifact kind;
- candidate schema ID and semantic version;
- required consent scopes;
- the exact private-read scope.

Task completion repeats expected input revisions and carries the candidate JSON. The runtime validates the generated
Draft 2020-12 schema first, then Rust deserialization and semantic rules. A failed validation cannot be treated as an
authoritative state transition.

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
canisend agent capabilities --json
canisend agent context --json
canisend schema list --json
canisend resource list --json
```

`agent capabilities` distinguishes `available` work from `planned` work and publishes the error-code registry.
`schema list` reports canonical schema IDs, versions, resource IDs, sizes, and SHA-256 digests. `resource list`
reports all compiled host guides, examples, prompts, schemas, and templates. The embedded Codex, Claude, and generic
guides instruct hosts never to edit `.canisend/` state directly.

Job intake is available through `job create`, `job import JOB_ID --file PATH`, `job import JOB_ID --url URL`,
`job list`, `job show`, and `job archive`. Import success returns source and artifact references without returning the
private source body. A URL flag is an explicit user-requested fetch; redirects remain subject to the same public
address policy as the initial URL.

## Contract generation

Rust types in `canisend-contracts` are authoritative. Eighteen public schemas are generated with canonical IDs under
`https://schemas.canisend.dev/v2/`, semantic version `2.0.0`, deterministic key ordering, and a final newline.
`cargo run -p xtask -- schemas check` rejects byte drift, missing files, and additional schema files.

All compiled resources are declared in one manifest. The build fails for a missing, duplicate, undeclared, unsafe,
or symlinked resource. The generated resource catalog embeds each file and records its typed ID, kind, version, byte
size, and SHA-256 digest.
