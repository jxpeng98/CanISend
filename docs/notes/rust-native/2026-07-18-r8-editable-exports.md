# R8.5 editable export lifecycle note

## Outcome

R8.5 completes the reviewable application-material pipeline. A `ready-to-export` package can now produce editable
Markdown, structured JSON, and a package-manifest projection without making those files authoritative and without
claiming that an application was submitted.

The public surface is:

```text
canisend package export --job JOB_ID --destination jobs/JOB_ID/application \
  --allow-private-export --json
canisend package exports --job JOB_ID --json
canisend package reconcile --job JOB_ID --json
canisend package replace --job JOB_ID --path jobs/JOB_ID/application/cover-letter.md --json
canisend package copy-as-new --job JOB_ID \
  --path jobs/JOB_ID/application/cover-letter.md \
  --destination jobs/JOB_ID/application/cover-letter-edited.md --json
```

The first command requires explicit `export-private-artifacts` consent and accepts only a safe destination beneath
the exact job tree. Every export receipt contains exact source artifact references, generated and observed SHA-256
digests, edit status, and `submission_performed: false`.

## Authority and edit boundary

Structured document and package artifacts remain the only authoritative application state. Markdown preserves
document identity, revision, source digest, claim identity, citation targets, and placeholder state as metadata and
comments, but editing Markdown does not silently rewrite a claim, citation, evidence record, or package decision.
Structured JSON gives an agent or user a complete inspectable projection of the same durable document.

Managed projections have four observed states:

- `current`: observed bytes match the generated digest;
- `edited`: a regular file exists with different bytes;
- `missing`: the managed file no longer exists;
- `repair-required`: the destination cannot be safely observed or written.

Export preflights every destination before writing. It refuses an existing unmanaged file and refuses to overwrite a
managed edit. `replace` is the explicit destructive choice for one projection. `copy-as-new` is the preserving choice:
it writes the edited bytes to a new, unmanaged, create-new path and then restores the managed generated form. Both
operations return a contract asserting `authoritative_changed: false`.

## Persistence and invalidation

SQLite migration 12 extends projection manifests with projection kind, generated/observed hashes, edit status, and
safe error state. It also adds `export_heads`, which bind one current export-manifest artifact to the exact current
package. The export artifact itself depends on the package and every exported document.

Profile, evidence, plan, document, review, job, or workflow changes stale package and export heads together. The
synthetic integration test proves a profile revision removes both current package readiness and the current export
receipt. Old user files remain ordinary files; stale metadata cannot make them current again.

## Agent-host integration

Codex, Claude, and generic guides now require explicit private-export approval and explain reconciliation choices.
Their self-contained 29-file packs include `projection`, `projection-reconcile`, and `package-export-manifest`
schemas. The binary exposes 38 deterministic public schemas and 48 embedded resources in total.

## Verification

Local acceptance passed:

- formatting and full-workspace Clippy with warnings denied;
- 68 Rust tests, including loopback transport tests, export consent, full package export, edit detection,
  copy-as-new preservation, replace, missing-file recovery, and upstream invalidation;
- 38 generated public-schema checks and 48 embedded-resource checks;
- locked release compilation;
- packaged host-agent smoke from the release binary with the expanded host pack.

R9 can now consume stable structured documents and managed projections without redefining their authority.
