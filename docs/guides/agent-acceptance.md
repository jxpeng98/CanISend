# Agent acceptance: automation first

Run the existing automated checks before asking a user to exercise a real Host. These checks use
synthetic data and synthetic MCP form responses; they do not establish human consent or Host UI
qualification. Keep the interactive Workspace separate from the automated fixture.

## Automated checks

From the repository root, build the native CLI if it is not current, then run:

```sh
cargo build -p canisend --locked
cargo test -p canisend --locked --test mcp_protocol
cargo test -p canisend-app --locked --lib approval::tests
bash scripts/smoke_agent_v4_mcp.sh target/debug/canisend dist/agent-acceptance-run
```

The `mcp_protocol` integration test is the simulated Host: it starts an isolated MCP process,
answers native form requests with scripted True/False/decline/cancel/malformed responses, and
verifies durable state and process-restart behavior. It needs no model account, interactive UI,
network access, or manual clicks. The same suite can target an extracted local candidate:

```sh
CANISEND_TEST_CLI_BINARY=/absolute/path/to/extracted/canisend \
  cargo test -p canisend --locked --test mcp_protocol
```

The override exists only in test code and must name an absolute existing file. Both the MCP
server and CLI cross-checks use it. Use a candidate built from compatible current source: the
fixture deliberately compares the complete current tool contract. The build host needs Rust to
compile the harness; the extracted consumer binary still needs no language runtime. Fast CI runs
the default simulated Host; the release workflow additionally runs it once against the extracted
Linux GNU candidate, reusing the release-profile build. Other native targets retain their archive
smokes. These scripted responses are never user approval or actual Host UI qualification.

The smoke destination must not exist. Choose a new directory for a subsequent run; do not erase
an interactive Workspace to reuse its path. These commands use the existing test suites rather
than introducing another runner. Fast CI already runs the protocol suite and the dual-Pack smoke.

| Check | Primary owner |
|---|---|
| Both Packs: extraction, Requirement confirmation, Plan proposal/confirmation, drafting, review, local export | Dual-Pack MCP smoke |
| True, False, decline, cancel, unsupported form capability and malformed responses; rejected writes preserve state | MCP protocol lifecycle test |
| Expiry, replay, process-local grants and unchanged deadlines | ApprovalBroker tests, including a manual clock |
| Exact canonical state after reopening and backup/restore; original export manifest and file verification | Dual-Pack MCP smoke |

`acceptance-summary.json` records body-free final Application identities, Pack digests, revisions,
snapshot digests and verification scope. Full fixture outputs contain synthetic data and may
contain test tokens; share the summary, not raw protocol logs. Scoped export directories are
derived output and are not restored by backup. The smoke verifies that authoritative draft/review
state survives and that restored export discovery is empty; a new export needs fresh consent.
The 120-second Host form timeout has no dedicated elapsed-time regression in these suites; do not
report that branch as tested merely because other rejection cases pass.

## Minimal real Host check

This is candidate qualification, not a prerequisite for continuing development. Run it after the
affected Host interaction has stabilized; repeat only the changed interaction or the required
candidate binding. Ordinary edits use the automated checks above and `xtask source check`.
`xtask release check` retains the full evidence gate. Never refresh an old passed record merely
to match new source digests.

Keep an interactive fixture and its backup outside system temporary directories. Read actual
current state and the complete, digest-matched Pack catalog before proposing materials. Academic
requires both `cover-letter` and `cv`, plus every additional Required kind in the confirmed Plan;
a first validation error is not a complete catalog listing.
Import a synthetic Profile Source, use `canisend_evidence_confirm_preview` with exact source
quotes and normalized byte ranges, then confirm its catalog through the native form. Associate
the confirmed Evidence with each Application through the guarded association tools before
drafting. A Profile Source alone is not confirmed Evidence; do not seed the database directly.

1. If not already qualified for the tested build and Host, observe one actual False rejection and
   one fresh True confirmation. Verify their canonical state effects. Retain existing matching
   evidence instead of asking the user to repeat the same checks.
2. In one user-controlled Host, complete the remaining dual-Pack journey. Present each exact
   proposal and use `request_confirmation: true` to request its native form. Only the user's
   actual form acceptance authorizes the change. Keep review and execution in that Host rather
   than copying preview metadata to another conversation. Refresh an expired preview and
   re-review any changed proposal. Rediscover tool schemas after upgrading: legacy `approved`
   and `confirmed_private_read/export` inputs are rejected, not aliases for the request fields.
3. Open another local Host session against the same Workspace. Read and compare the persisted
   Pack identities, revisions, Plans, Deliverable metadata, export metadata and Workspace health.
   A newly initialized Workspace is not resumption of the earlier one.

For each commit, token, digest and expiry must come from the same successful preview response in
the same MCP process. If crossing tool invocations, retain that complete response with the Host's
existing state facility and verify retrieval; never reconstruct a token from conversation text.
Clear the selected local record before its single commit attempt. Stop that attempt on denial or failure.
Loss of the record, expiry or MCP restart requires a fresh preview, not an old-token retry.
Continue authorized diagnosis and isolated automated tests without asking for process permission.
Do not repeat a denied real mutation or answer the native form for the user.

Subagents can review metadata, candidates and artifact consistency independently. They must not
receive active approval tokens, answer human forms or turn synthetic responses into real Host
acceptance evidence. Record automated checks, human observations, protected CI and exact release
artifact qualification separately. Missing audit receipt fields are not invented audit evidence.

## Profile interview before an Application

Use `canisend-workspace` and the existing `profile-evidence` task. Begin with a fresh Workspace
containing no Profile Sources or Applications. Verify the installed Skill matches the candidate;
an older executable cannot install the new interview guidance. An isolated model forward test
can check conversation behavior, but does not establish actual Host file access, native forms,
or real-user acceptance. Keep its findings separate from the interactive observations below.

Profile import is CLI-only: `canisend --workspace PATH profile-source import FILE
--sensitivity private-local --confirm-private-read --json`. The consent flag asserts the user's
actual authorization to read that file; factual review alone does not supply it. Read the
candidate's `profile-source import --help` rather than using a legacy command path.

| User situation | Expected behavior |
|---|---|
| No prepared Profile and no specific opportunity | Start with a small adaptive round, reuse goals/language already supplied, and avoid requiring an Application or inventing credentials |
| Pause after an incomplete answer, then reopen the same Workspace | Read `inputs/profile-interview/interview.md` and `profile.md`, preserve progress, and continue from the next relevant gap without repeating settled identity questions |
| Correct a proposed leadership role to assistance; year is unknown | Remove the superseded claim, preserve the correction, and leave the year uncertain rather than infer it |
| Manually edit the Profile between sessions | Preserve the edit, reconcile conflicts explicitly, and do not overwrite it with an older interview summary |
| Describe a funding application as a future plan | Keep the intention separate from factual experience; never turn it into an awarded grant |
| An organization prepares a grant Profile with an approximate participant count | Adapt the interview to the organization, clarify count/time scope, and preserve uncertainty instead of imposing academic career modules |
| Host has no file tools | Provide copyable drafts and state that progress has not been saved |
| User wants a draft saved but no import | Save only ordinary user files; retain zero canonical Profile Sources and Applications |
| User reviews a first Profile and authorizes its import | Use the supported Profile Source import, record its exact receipt, and verify one Source with zero Applications; Evidence confirmation and association remain later steps |

Use fictional facts for the functional check; the human controls the conversation and all native
forms. These cases do not count as the formal real-applicant cohort. Inspect both saved files,
not only the final chat response. Proposed facts and unresolved details belong in the interview
record; the importable Profile contains reviewed facts, self-report attribution and separately
labeled intentions. Do not leave a superseded role, guessed date or unreviewed achievement in it.
After import, correct facts through a new reviewed Source and explicit reconciliation of affected
Evidence/associations; do not claim the new Source automatically replaced earlier links.

### Prepare the interactive Workspace

Use a new persistent directory and the exact extracted candidate executable, not `target/debug`
or an older binary on `PATH`. For example, substitute actual absolute paths in these commands:

```sh
/absolute/path/to/extracted/canisend --workspace /absolute/path/to/profile-host-check workspace init --json
/absolute/path/to/extracted/canisend --workspace /absolute/path/to/profile-host-check host setup --host codex --executable /absolute/path/to/extracted/canisend --json
/absolute/path/to/extracted/canisend --workspace /absolute/path/to/profile-host-check workspace status --json
```

Setup installs project Skills and returns MCP registration guidance. Follow that guidance in the
chosen Host, open this Workspace there and rediscover tools. `host status` checks installation,
not the live connection. Ask the Host to read `canisend_workspace_status` and
`canisend_application_list` before the interview. Start with:

> I have no prepared Profile and no specific opportunity. Use canisend-workspace to interview
> me for research and teaching applications, a few questions at a time. Keep uncertain details
> separate, show me summaries to review and save my progress. Do not import or create an
> Application yet.

After a small round, reopen the same Workspace in another Host session and exercise the resume
and correction cases. Import only after reviewing the complete Profile and giving the actual
required consent. Record candidate archive/binary hashes, installed Skill hash, Host name/version,
Workspace identity, observed case outcomes and exact import receipt metadata. Omit private
bodies, conversation text and active tokens from any shared result. Leave unobserved cases pending;
a prepared checklist or an installed Skill is not a passed acceptance record.

## Whole-application Skill scenarios

Use `canisend-application-workflow` for an end-to-end request. It coordinates the existing ten
canonical tasks; the other four Skills own stage operations. Validate content behavior separately
from the protocol smoke: scripted draft inputs prove the API lifecycle, not model writing quality.
Use isolated synthetic sources for these scenarios; never supply real confirmation responses.

| User situation | Expected behavior |
|---|---|
| A new academic advert plus CV; prepare the application | Build a source-backed opportunity brief and Evidence, confirm Requirements, assess fit, plan catalog-required materials, draft purpose-specific content, review and export with actual required consent |
| A grant/tender/admission request using the generic Pack | Derive purpose and material structure from the supplied call; do not impose academic criteria or unsupported Deliverable kinds |
| A mandatory criterion has only partial support | Explain the precise gap and possible next evidence; preserve proceed/hold choice and do not invent qualifications or infer eligibility |
| Only revise one paragraph of an existing letter | Read the current draft and relevant support, preserve unaffected content, re-audit the revision; do not restart intake or ask to repeat settled choices |
| A revised advert conflicts with earlier requirements | Identify source/version conflict, clarify authority when needed, refresh affected Requirements/Plan/materials and review; do not present the old export as current |
| A new Host session finds a Submitted local candidate | Recover canonical Application and task metadata, review the exact candidate through supported private-read/preview paths, never reuse an old token or treat a lease as consent |
| Resume from backup with drafts but no scoped export files | Verify recovered state; request a fresh export when authorized rather than declaring drafts lost or copying stale exports |
| A required native form is declined during the journey | Stop that mutation; continue only independently authorized work without a substitute commit or fabricated approval |

Review outputs for source traceability, requirement coverage, audience-specific writing,
cross-document consistency and explicit gaps. If visual document inspection is unavailable,
report that limit. Neither a validated Skill manifest nor the dual-Pack smoke establishes these
model-level outcomes automatically.

### Current-surface review cases

- A new Application request uses the supported CLI creation schema; it must not wait for a
  nonexistent MCP creation preview. Missing Source replacement adapters are reported precisely.
- The first academic draft contains the catalog-required cover letter and CV together with all
  other Required kinds in the confirmed Plan, such as research and teaching statements. Once
  drafts exist, a one-document edit uses revise rather than another initial draft commit.
- Candidate reasoning happens before draft preview; `deliverable.audit` inspects persisted drafts
  after commit, not an unsaved candidate. A Plan proposal must be confirmed before drafting.
- Review disposition targets the current Application revision, not an invented finding/waiver ID.
  Source text, private bodies and exported files remain within the relevant consent scope.

These cases were checked against CLI/MCP parameter types and Store preconditions. They remain
model behavior scenarios, not claims that a live model session has passed them.

### Resume acceptance from observed state

Before requesting another test, read the newest result records and canonical Application state;
do not select the next action from an old pasted recap. A later successful True receipt and matching
read invalidate an earlier “awaiting True” message. Record the completed case once and advance.
A conflict after that success is a duplicate-operation check, not a failed native confirmation.

For a planned multi-case acceptance run, a negative case ends that mutation, not the whole run.
Continue independently authorized checks and the next explicitly authorized case; only require
new input where the outcome or consent scope is missing. Never auto-answer a form. If the user
only authorized one case, report that result and stop at that scope. Do not reset a confirmed
Requirement or silently create a new Application just to repeat a completed test.

The owning protocol regression now covers successful confirmation followed by a duplicate preview:
it returns conflict with read-state remediation, opens no new form, preserves the exact snapshot,
and allows the next Plan stage. Model continuity must also be observed in the actual Host.
