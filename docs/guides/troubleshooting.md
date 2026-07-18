# Troubleshooting

Start with the native and workspace checks:

```console
canisend doctor
canisend --workspace ./applications workspace check
```

Add `--json` to the failing command when reporting a problem. Preserve the stable error code, status, exit code,
retryable flag, and remediation, but remove private bodies and local paths before sharing logs.

## Common errors

### `workspace.not_found`

Pass the directory containing `canisend.toml`, or initialize a new directory:

```console
canisend --workspace ./applications workspace init
```

### `pdf_text_unavailable`

The PDF is probably scanned/image-only or has no extractable text. OCR is deliberately not embedded in the first
native release. Obtain a text-based PDF, or run a trusted OCR tool separately, review the output, save it as UTF-8
Markdown/plain text, and import that reviewed file. Do not treat unreviewed OCR as authoritative job criteria.

### `pdf.encrypted` or `pdf.malformed`

Request an unencrypted valid advert. CanISend never accepts or stores PDF passwords. A page-limit, size-limit, or
time-budget rejection is also reported as a bounded PDF/input error; splitting content must not omit requirements.

### `consent.required`

Read the response's exact scopes and artifacts. Use `--allow-private-read` only after approving the declared task
inputs, `--allow-provider-send` only for a configured-provider task, and `--allow-private-export` only for a local
package/PDF destination. A flag applies to that command, not future work.

### `candidate.schema_invalid` or `candidate.semantic_invalid`

Use the returned violation codes and JSON pointers, correct the candidate, and retry the same lease. Validation
failure does not commit task state. Never remove citations, change expected revisions, or invent IDs merely to make a
candidate pass.

### `task.stale`

A lease expired or a job/profile/input revision changed. Prepare the operation again and generate a candidate from
the new descriptor. Do not replay the old private input bundle.

### `workflow.conflict`

Inspect `workflow status --job JOB_ID --json`. Complete the named prerequisite or follow its `next_actions`. If an
upstream revision intentionally changed, use the scoped `workflow rerun` command rather than modifying SQLite.

### projection edited, missing, or unmanaged conflict

Run `package reconcile` to inspect generated versus observed hashes. Use `package replace` only to discard an edit
explicitly, or `package copy-as-new` to preserve it separately. `workspace repair` rebuilds missing deterministic
files but never overwrites an edited projection silently.

### `blob.reference_invalid` or database integrity failure

Stop writing. Preserve the workspace for diagnosis and restore a separately stored verified backup into a new path.
Repair is not content recovery.

### URL/provider fetch failure

Confirm the URL is public HTTP(S), contains no credentials, and uses the selected adapter's documented host. Private,
loopback, link-local, documentation-only, DNS-rebinding, and cross-provider redirect targets are rejected by design.
Retry transient external I/O only when the response says `retryable: true`.

## What to include in a bug report

- `canisend version --json` and `canisend doctor --json`;
- operating system and architecture;
- stable operation/status/error code and exit code;
- whether the failure reproduces in a new synthetic workspace;
- redacted steps using public fixtures where possible.

Never attach a real workspace, backup, CV, job source, task input directory, draft, or provider credential to a public
issue.
