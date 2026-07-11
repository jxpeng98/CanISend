# Host-Neutral Agent Handoff

This fake-data contract example demonstrates how two shell-capable hosts can resume the same CanISend application
workflow from durable workspace state. It does not copy a prompt, transcript, or provider session between hosts.

Host A and host B both request the current context with the same command:

```bash
canisend agent context \
  --workspace <workspace> \
  --job jobs/<job-id> \
  --format json
```

The response contains versioned protocol metadata, a safe job summary, a derived workflow snapshot, relative or
opaque artifact references, missing fields, consent requirements, blockers, and next actions. It does not contain the
full advert, profile, or package body.

`expected_capabilities.json` records the stable agent capability subset. `expected_context_shape.json` records the
required response shape and private body fields that must never appear. Dynamic values such as `request_id`, package
version, job ID, hashes, and derived readiness are intentionally not frozen in the fixtures.

The accepted shell contract now covers durable preparation, guarded candidate submission, application, cancellation,
promotion, recovery, and status for Parse, Confirm, Evidence, and Match. Evidence and Match are deterministic-only;
they do not need a platform API, MCP transport, hosted service, or configured provider. A fresh host resumes with the
same `agent context` and `stage status` commands, then may run:

```bash
canisend extract-profile-evidence --workspace <workspace>
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage evidence --mode deterministic --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage parse --mode deterministic --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage confirm --mode deterministic --format json
canisend corrections status --workspace <workspace> --job jobs/<job-id> --format json
canisend corrections init --workspace <workspace> --job jobs/<job-id> --confirm-user-owned-write --format json
canisend corrections status --workspace <workspace> --job jobs/<job-id> --format json
canisend corrections update --workspace <workspace> --job jobs/<job-id> --patch-file <strict-patch> \
  --expected-revision <status-revision> --expected-sha256 <status-sha256> \
  --confirm-user-owned-write --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage confirm --mode deterministic --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage match --mode deterministic --format json
canisend decision status --workspace <workspace> --job jobs/<job-id> --format json
canisend decision init --workspace <workspace> --job jobs/<job-id> --confirm-user-owned-write --format json
canisend decision update --workspace <workspace> --job jobs/<job-id> --patch-file <strict-patch> \
  --expected-revision <status-revision> --expected-sha256 <status-sha256> \
  --confirm-user-owned-write --format json
```

The example placeholders are deliberately not shell-parsed dynamic hashes. A fresh host takes the artifact SHA-256
and `canisend.user_artifact_revision` from the immediately preceding status response, writes one bounded strict patch
in safe scratch space, and sends it to the operation. It never writes a user YAML file directly. Parse and Confirm
must be current for each corrections patch; empty initialization is fingerprint-neutral, while every semantic
correction requires a Confirm rerun. Initialization alone does not mean `confirmed_empty`, and undecided is not
apply/hold/skip.

If a Decision basis later changes, the YAML/value remains present and status derives review-required. Reconfirm with
a new patch and fresh baseline. If an accepted mutation reports receipt pending, a new host may run
`user-mutation recover` with its opaque mutation ID and explicit consent; it must not replay the private patch.

The Evidence run snapshot, candidate, and catalog may duplicate private profile bodies and are retained until the user
removes the run or job. Agent context and workflow control records expose only privacy-safe artifact references,
hashes, IDs, reason codes, and counts. Match output is body-minimized but remains Tier 2, so an agent asks before
reading it. Match classifications remain `review_state=proposed`; this example records Decision only through the
separate user-owned operation. Brief and required-document planning remain open.
Private user YAML/candidates/corrected Criteria are Tier 2, while immutable body-free mutation receipts are Tier 1.
CAS coordinates cooperative CanISend writers in a stable local job directory, so hosts serialize mutations and avoid
concurrent manual editor saves. Reset/clear/withdraw does not erase historical corrections or private-mode candidates (0600 on POSIX);
the ignored job stays private until the user separately removes retained events or the whole job. Automatic secure
erasure, including from backups/snapshots, is not claimed.
