# Host-Neutral Agent Handoff

This fake-data contract example demonstrates how two shell-capable hosts can resume the same CanISend application
workflow from durable workspace state. It does not copy a prompt, transcript, or provider session between hosts.

Host A and host B both request the current context with the same command:

```bash
canisend agent context \
  --workspace <workspace> \
  --job jobs/<job-id> \
  --format json
canisend stage status --workspace <workspace> --job jobs/<job-id> --format json
```

The response contains versioned protocol metadata, a safe job summary, a derived workflow snapshot, relative or
opaque artifact references, missing fields, consent requirements, blockers, and next actions. It does not contain the
full advert, profile, or package body.

`expected_capabilities.json` records the stable agent capability subset. `expected_context_shape.json` records the
required response shape and private body fields that must never appear. Dynamic values such as `request_id`, package
version, job ID, hashes, and derived readiness are intentionally not frozen in the fixtures.

The accepted shell contract covers durable preparation, guarded candidate submission, application, cancellation,
promotion, recovery, and status through the first structured Cover Letter Draft and independent Review slice.
Evidence, Match, Brief planning, and Review are deterministic-only; Draft uses the current host agent through the
same files and CLI rather than a platform API, MCP transport, hosted service, network, or configured provider. A
fresh host resumes with the same `agent context` and `stage status` commands, then may run:

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
canisend brief status --workspace <workspace> --job jobs/<job-id> --format json
canisend brief init --workspace <workspace> --job jobs/<job-id> --confirm-user-owned-write --format json
canisend brief update --workspace <workspace> --job jobs/<job-id> --patch-file <strict-patch> \
  --expected-revision <status-revision> --expected-sha256 <status-sha256> \
  --confirm-user-owned-write --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage brief --mode deterministic --format json
canisend stage prepare --workspace <workspace> --job jobs/<job-id> --stage draft --mode host-agent --format json
canisend stage submit --workspace <workspace> --job jobs/<job-id> --task <task-path> \
  --candidate-file <private-scratch-candidate.json> --format json
canisend stage apply --workspace <workspace> --job jobs/<job-id> --task <task-path> \
  --result <result-path> --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage review --mode deterministic --format json
canisend run --workspace <workspace> --job jobs/<job-id>
```

The example placeholders are deliberately not shell-parsed dynamic hashes. A fresh host takes the artifact SHA-256
and `canisend.user_artifact_revision` from the immediately preceding status response, writes one bounded strict patch
in safe scratch space, and sends it to the operation. It never writes a user YAML file directly. Parse and Confirm
must be current for each corrections patch; empty initialization is fingerprint-neutral, while every semantic
correction requires a Confirm rerun. Corrections initialization alone does not mean `confirmed_empty`, undecided is
not apply/hold/skip, and an empty Parsed Job document list is not a `confirmed_empty` document requirement set.

If a Decision basis later changes, the YAML/value remains present and status derives review-required. Reconfirm with
a new patch and fresh baseline. If an accepted mutation reports receipt pending, a new host may run
`user-mutation recover` with its opaque mutation ID and explicit consent; it must not replay the private patch.
Brief initialization and changes also require a current confirmed apply Decision. A changed Decision basis preserves
the Brief but requires explicit reconfirmation.

Draft preparation returns `read-private-draft-inputs`; the user approves that consent before the host reads the seven
declared Tier 2 inputs. The host writes only schema-valid scratch JSON, then uses the returned job-relative task and
result paths with `stage submit` and `stage apply`. It never writes run paths or `cover_letter_draft.json` directly.
Review is then regenerated independently and exposes body-free counts/codes before any Claim or finding body is read.

The Evidence run snapshot, candidate, and catalog may duplicate private profile bodies and are retained until the user
removes the run or job. Agent context and workflow control records expose only privacy-safe artifact references,
hashes, IDs, reason codes, and counts. Match output is body-minimized but remains Tier 2, so an agent asks before
reading it. Match classifications remain `review_state=proposed`; this example records Decision and Brief only
through their separate user-owned operations. `application_brief.yaml` and `required_document_plan.json` are both
Tier 2 ask-first bodies; status exposes only safe hashes, IDs, states, reasons, counts, and blockers. Unconfirmed,
`required + omit`, missing-action, and orphaned-choice states block later Draft/Verify work.
Private user YAML/candidates/corrected Criteria/plan bodies are Tier 2, while immutable body-free mutation receipts
are Tier 1.
CAS coordinates cooperative CanISend writers in a stable local job directory, so hosts serialize mutations and avoid
concurrent manual editor saves. Reset/clear/withdraw does not erase historical corrections or private-mode candidates (0600 on POSIX);
the ignored job stays private until the user separately removes retained events or the whole job. Automatic secure
erasure, including from backups/snapshots, is not claimed.

The three YAML files remain manual user-owned Tier 2 ask-first inputs: `confirmed_corrections.yaml`,
`application_decision.yaml`, and `application_brief.yaml`. Users may edit them directly. Agents use body-free status,
one bounded private patch, the latest revision/hash CAS baseline, and explicit consent; they do not replace a whole
YAML file. CAS does not make concurrent manual editor saves safe.

With current deterministic Match and the workspace-configured profile, the final `canisend run` above projects the
same proposed graph into fit/checklist/HR-review package views. A current validated Draft plus current Review with no
blocker findings also projects every Cover Letter Claim once into `03_cover_letter_draft.md`, content JSON, and both
Typst views. Stale or drifted/tampered structured state, a blocked/missing Review, a mismatching parsed view, a profile
override, or direct library use safely falls back to legacy deterministic generation; `--llm-drafts` keeps provider
output. Edited Typst is preserved and receives a `*.generated.typ` candidate. Every classification, Draft, finding,
and compatibility projection remains proposed review work, not a Decision or package-readiness result.
