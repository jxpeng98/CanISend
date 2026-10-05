# Academic workflow review and Stable 1.0 repairs

Reviewed and repaired the working tree based on
`7f20e0d3de7ecb7ea897ad2568ecf6317a54c219` on 2026-10-03. The owner requested
a structural and output review, current Typst templates, and implementation of
the resulting improvements. The priority is an academic profile producing a CV,
cover letter, research statement and teaching statement. This record owns the
scope, actual checks and remaining work for that request. A follow-up authorized
conversational Profile construction when the user has no prepared materials.

**Assessment: the three reproduced output blockers are repaired and locally
verified; formal Stable qualification remains incomplete.** The supported guarded
workflow now requires the complete confirmed document plan, carries reviewed
structured fields into PDFs, and enforces the Pack's declared readiness checks.
The version remains `1.0.0-beta.10`; these results do not authorize a Stable tag.

## Structure and supported process

Contracts owns public data, Core owns storage-independent rules, Resources owns
verified Packs and templates, IO owns bounded intake and rendering, Store owns
persistence and recovery, and App composes the use cases. CLI and MCP use the App
facade. The repairs follow those boundaries and add no academic-only branch to
the shared production kernel. The reviewed domain inventory records the new
contract documentation and academic integration test explicitly.

The accepted [CLI-first decision](../../architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md)
keeps a persistent Host/MCP connection for guarded work. The Host prepares
content from authorized sources; the kernel controls evidence, consent, review,
export and recovery. The process is: import the opportunity and profile,
explicitly associate private sources, confirm sourced evidence and requirements,
confirm the document plan, prepare drafts, inspect and approve them, then export
local files. No identity is guessed from unrelated Workspace sources.

## Initial findings and completed repairs

The initial isolated probes reproduced all three P1 findings below through the
shared Store functions used by guarded v4 review/export. Their successful exports
describe the **pre-repair behavior**, not the current implementation. Historical
probe sources, PDFs and logs remain in ignored `dist/stable-workflow-review/`.

### Required statements could be omitted

A confirmed Plan required all four academic document kinds, but the application
approved and exported only a CV and cover letter. Pack count limits allowed the
statements to be absent. Snapshot validation of existing Deliverables did not
detect missing ones.

Core now validates every required Plan item separately from Pack count limits.
Composition and export reject incomplete or stale materializations. Optional
items may be absent; omitted items cannot be materialized. The owning academic
journey verifies that the incomplete draft set is rejected without changing the
Application snapshot.

### Profile fields and document structure were dropped

The active Deliverable projection provided an empty field map and one unnamed
text section. Exported PDFs lost the fictional candidate's identity and email,
used the document title as the masthead and displayed Markdown headings literally.
Direct structured-template previews retained the same profile fields.

The new [Deliverable document contract](../../contracts/deliverable-document-v3.md)
defines bounded fields, ordered sections, text roles, exact evidence and
requirement citations, and unresolved field identifiers. Store validates the
typed content and projects it as JSON; IO maps fields and sections into the
existing restricted Typst renderer. Text remains literal data, including strings
that resemble Typst commands. Draft creation, revision and Host confirmation
retain the same reviewed content. MCP confirmation presents fields, paragraphs,
roles and citations readably without changing the preview or approval digest.

Legacy text and Markdown drafts remain readable, but must be explicitly revised
to structured content before approval/export under Packs declaring these
validators. They are not silently assigned factual classifications.

### Declared readiness validators were not enforced

An initial probe exported four PDFs with no evidence inputs and a visible TODO.
The paths checked Blob integrity and review state but did not dispatch the Pack's
traceability, unsupported-claim, placeholder, citation-integrity and review checks.

Core now dispatches each document kind's exact declared bindings; unsupported
validators or parameters block approval/export. Store checks current confirmed,
associated, non-excluded evidence, exact revisions and hashes, verified source
Blobs, current requirement revisions, resolved fields, reserved placeholder
markers and review state. It repeats export validation after rendering and before
publishing files. Owning regressions cover evidence invalidated during rendering
and Application revision changes, with no export files or export audit published.

Academic Pack `1.0.3` requires evidence-bound `candidate-name` and `email` fields.
The kernel verifies provenance and the declared text roles; it does not establish
natural-language entailment. The Host and human reviewer still need to inspect
whether the cited sources support the text and its factual classification.

## Templates, resources and packaging

- Updated ModernPro CV `2.1.1` to `2.1.2`, released commit
  [`e59cd872`](https://github.com/jxpeng98/Typst-CV-Resume/tree/2.1.2), and cover
  letter `1.0.2` to `1.0.3`, released commit
  [`e93ca659`](https://github.com/jxpeng98/typst-coverletter/tree/1.0.3).
  These releases improve adaptive headers and long contact wrapping. Template
  source pins, licenses and hashes were verified using the existing offline sync.
- Academic Pack now binds `1.0.3`, digest
  `ef6a94be3d971ed0b44666863c28be0167fc4c058c7deef254ba7afae0f69bca`.
  Exact `1.0.0` and `1.0.1` historical bundles match the original baseline; the
  intermediate `1.0.2` bundle is also archived. Existing Applications keep their
  exact Pack bindings.
- Registered the eighth Application schema and refreshed the 92-entry resource
  and package contracts. Template synchronization now updates the resource count
  as well as its hash; its isolated regression checks a deliberately stale count.
- Updated the materials and review/export Host Skills and all five shipped generic
  examples to the structured contract. The exact-archive MCP smoke fixture now
  confirms real source-spanned evidence from its fictional profile. Synthetic
  approvals are restricted to isolated tests and automated smoke fixtures.
- Corrected the [limitations guide](../../guides/known-limitations.md) to separate
  the qualified Beta.1 checkpoint from Beta.10 registry testing releases and to
  document structured authoring and the remaining human review responsibility.

## Output repair milestone validation

These checks refer to the repaired output workflow, not the initial probes. This
milestone predates the Profile interview follow-up below; its archive hash binds
that earlier exact artifact. Logs and machine records are in ignored
`dist/stable-workflow-repair/`.

- `cargo test -p canisend-contracts -p canisend-core -p canisend-store -p canisend-io
  -p canisend-app -p canisend-mcp -p canisend --locked`: **370 passed, zero failed**.
  Three existing tests remained ignored: release-only performance, the public
  GitHub release endpoint and the scheduled large-fixture latency baseline.
- The final resource suite added **17 passed, zero failed**, for **387 passing
  Rust tests** across those suites. The Python template synchronization regression
  and current-template sync check passed.
- Formatting and relevant all-target Clippy with warnings denied passed. The
  final `cargo run -p xtask --locked -- source check` passed, including schemas,
  resources, dependency boundaries, operation contracts and package metadata.
- The clean-v4 academic regression imports and explicitly associates a fictional
  profile, confirms source-spanned evidence, requires all four document kinds,
  rejects missing documents, missing citations, unresolved fields, TODO markers,
  incorrect hashes, stale requirements and absent identity fields, then uses
  guarded v4 preview/commit review and export. All four PDFs retain candidate
  identity, email and ordered sections; each has one page and zero warnings.
- The optimized `release-alpha` CLI build passed. Packaging and smoke checks of
  the **exact extracted archive** passed on Linux GNU ARM64. Checks included an
  isolated consumer with an empty executable search path, binary identity and
  notices, documentation setup, project/global Host Skill lifecycle, legacy
  refusal, guarded dual-Pack MCP workflows, reopen, backup/restore, export
  verification, uninstall and Workspace retention.
- Local package SHA-256:
  `b8da367438bea6e9d98b2bbcbfcd8ad61fe0968357011ffcc2c5500330a3f099`.
  This is one unsigned candidate from the dirty working tree, not qualification
  of the other native targets. The local compiler was Rust `1.99.0`; the pinned
  `1.97` compiler was not independently exercised in this environment.
- Strict `xtask release check` passed source and existing dependency assurance,
  then rejected **a stale provider-dogfood Agent v4 contract binding**. No Host
  evidence or dependency exception was renewed by these automated checks.

The final fictional PDF set and its export manifest are in
`dist/stable-workflow-repair/academic-pdfs/`; a checksum-verified review copy is
`dist/stable-workflow-repair/academic-package-example.zip`. This demonstrates the
guarded workflow and renderer with a small synthetic profile. It is not a real
applicant's finished application or real-user acceptance.

## Profile interview follow-up

Implemented the owner's follow-up through the existing `profile-evidence` task,
shared Host guidance and application handoff prompt. The workflow can begin before
an opportunity or Application exists; it does not add another task model, database,
public operation or domain-specific kernel rule.

Workspace Skill `4.0.7` now guides small adaptive interview rounds, factual summary
review, self-report attribution, separate future intentions, uncertain answers,
conflict resolution and incremental completion. Editable `interview.md` keeps
progress and unanswered questions; `profile.md` holds the reviewed source. Resume
reads those files and preserves manual changes. Host file tools own these ordinary
user files, outside managed projections. If those tools are unavailable, the Host
provides copyable content and does not claim durable progress.

The workflow routes through supported Profile Source import, then exact source
association and source-spanned Evidence confirmation when an Application is chosen.
Imported corrections need reconciliation with earlier links and Evidence; a new
source does not silently invalidate old facts. Unimported interview files are not
part of canonical Workspace backup. These limits are documented in the
[Profile interview guide](../../guides/agent-integration.md#build-a-profile-from-conversation).
Application-workflow Skill `4.0.2`, all three Host guides `4.0.4`, Codex metadata
`4.0.2` and the quick start consistently expose this entry point.

Fresh validation for this follow-up:

- Both changed Skills passed the skill-authoring metadata/scaffold validator;
  Codex UI metadata and the 92-entry resource/package binding agree.
- All 17 owning resource tests passed, including exact exported Host pack bytes,
  installed links, versioned install/update, edit-safe refusal and uninstall.
- Three existing application handoff regressions passed. Formatting and relevant
  all-target App/Resources Clippy with warnings denied passed.
- The final `cargo run -p xtask --locked -- source check` passed. Its initial
  inventory failure exposed one additional match: Workspace Skill's explanatory
  phrase “job advert”. Inspection classified this as shared Host guidance, with
  no production kernel rule moved; the reviewed inventory now records 192 files.
- The optimized CLI build, package creation and exact extracted archive smoke
  passed on Linux GNU ARM64, including the existing guarded dual-Pack MCP lifecycle.
  Fresh archive SHA-256:
  `9d8127f772e7188755c0b667f92e945fd5fd2df9705051c37a897bef0c47c0c2`.
- A separate fictional fixture used that exact extracted archive with zero Profile
  Sources and zero Applications. Installed Workspace Skill bytes matched the current
  source. The reviewed Markdown imported as one Profile Source without creating an
  Application; draft files remained editable, uncertain details stayed outside the
  imported text, and Workspace health passed. This fixture represents a prepared
  synthetic interview summary, not a real Host conversation or human acceptance.

Follow-up logs, receipts and machine records are in ignored
`dist/profile-interview/`. The original output-repair artifact records are retained.
Existing Workspaces need the new executable followed by `workspace upgrade --json`
and a Host reconnect to load the updated Skills; upgrading with an older executable
cannot supply these new resources. Real-Host interview behavior, remote CI and
real-user acceptance were not exercised. This follow-up does not renew release
qualification or authorize Stable.

## Interview behavior and first-use follow-up, 2026-10-04

An independent model forward pass exercised Workspace Skill `4.0.7` on three
isolated fictional requests: a first academic interview without an opportunity,
resumption with corrected teaching responsibilities and manual Profile edits,
and an organization's grant Profile with an approximate participation count.
Inspection of all nine response/interview/Profile files passed the three cases:
small relevant question rounds, no invented degree or achievement, preserved
manual edits, removed superseded leadership/year claims, separate funding plans,
and uncertain counts kept outside the importable Profile. No external Host client,
CanISend mutation or native consent form was used in this model test. File-tool
fallback and human import were not exercised by it.

Preparing the exact-candidate Host Workspace then exposed an additional blocker:
the shipped Skill, Workspace README and integration guide advertised the legacy
`profile source import` path. The current CLI correctly rejected that surface
with `compatibility.unavailable`. The prepared synthetic import in the earlier
milestone had used the supported `profile-source import` operation, so it did not
validate this command example. Corrected all three active examples; Workspace
Skill is now `4.0.8`. Its conversational rules are unchanged from the forward pass.
The new CLI regression exports the actual Host pack through the App facade and
checks the advertised import command with the real legacy preflight and Clap
parser, including its private-read flags. It failed before the correction and
passed afterward; no legacy surface was restored.

Workspace README `1.0.1` now exposes the interview entry point for first use,
editable draft locations, resumption, review/import and unfinished-draft backup.
The [Agent acceptance guide](../../guides/agent-acceptance.md#profile-interview-before-an-application)
now covers nine Profile situations and confirms that initial academic drafts
include every Required kind in the confirmed Plan, including required statements.

Fresh checks for the final guidance repair passed: all six CLI library tests,
17 resource integration tests and the resource rollback unit test; formatting,
relevant all-target CLI Clippy with warnings denied, source check, documentation
links/configuration and diff checks. The optimized build and exact extracted
archive installation/lifecycle smoke passed on Linux GNU ARM64. Final local
archive SHA-256:
`bc2b09d60fa10282a5330e14a44ec8f69e4f585561ec00bb545713ede7fcd0bc`.
This remains an unsigned local artifact built from the working tree based on
`7f20e0d3de7ecb7ea897ad2568ecf6317a54c219`, not a qualified RC.

A separate fixture imported the model-generated fictional Profile through this
exact archive's corrected CLI. It went from zero Sources/Applications to one
Source and zero Applications, preserved both editable drafts, matched the imported
original bytes and passed Workspace health. Its consent flag was synthetic and
restricted to that isolated fixture; it is not human acceptance. Final logs and
machine records are in ignored `dist/profile-interview-cli-guidance/`; the original
forward inputs/results are in `dist/profile-interview-forward/model-fixture/`.
Earlier artifact records and the failed first preparation receipts are retained.

The separate human-controlled Workspace in `dist/profile-host-acceptance-final/`
was initialized from the same exact archive with zero Sources and Applications,
healthy state, and digest-matched Codex/Claude project Skills. `START-HERE.md`
contains exact paths, candidate hashes, returned MCP registration guidance,
conversation/resumption cases and result-recording instructions. Its nine human
observations remain pending. No global MCP registration or interactive Host session
was started, and no real user's form was answered. This prepares the remaining
Host check without claiming it or the formal applicant cohort has passed.

## Release boundary and remaining work

The [qualification ledger](../../../release/qualification-ledger.json) remains
`beta-qualifying`, with no qualified RC and `stable_authorized: false`. Continue
through the existing [1.0 roadmap](../../superpowers/plans/2026-07-25-1.0-release-roadmap.md):
run remote Fast CI and current exact-candidate Host evidence, perform the
consented real-user cohort acceptance, and qualify two distinct RCs with the
required artifact evidence before authorizing Stable. The cohort gate remains at
least eight real users and twenty complete workflows.

Remote CI for this working tree, fresh real-Host qualification and real-user
acceptance were not run. GUI work and full native qualification remain paused.
No tag, push, publication, qualification-ledger amendment or dependency-exception
renewal is part of this change. Local source checks, local artifact smoke tests,
remote CI and real-user acceptance remain separate facts.

## RC publication request and CI repair, 2026-10-04

The owner requested an RC publication. Implementation commit
`a7f5b4ab12c41d7baa8410ca460e60c0fa6b0bc5` was pushed to
`feat/evidence-bound-profile-workflows` and opened as
[PR #246](https://github.com/jxpeng98/CanISend/pull/246). Protected `main` requires
PR integration and six named Fast CI contexts; protections were not changed.

The initial [Fast CI run](https://github.com/jxpeng98/CanISend/actions/runs/37236952074)
exposed a stale synthetic contract-freeze test: current schema generation now
contains eight Application schemas and 56 total schemas, but the assertion still
expected seven and 55. The owning test reproduced locally before correcting those
two expectations. Historical contract-freeze and qualification records were not
regenerated. The initial
[dependency run](https://github.com/jxpeng98/CanISend/actions/runs/37236952068)
also rejected the exception review that became overdue after 2026-10-03.

A fresh `cargo-deny 0.19.7` scan against RustSec database commit
`ef6173cbc5c50ec8166f9a5b28f07834144373ee` passed advisories, bans, licenses and
sources. Re-reviewed all 23 existing entries and their current input boundaries;
the 751-package lock fingerprint, exception IDs, reachability, removal conditions
and hard expiry remain unchanged. Review dates are now 2026-10-04/2026-10-18,
with hard expiry still 2026-10-19. The
[dependency guide](../../release/dependency-assurance.md) records the exact scan
and the unused `lopdf` font-creation path checked during review.

Fresh local repair validation passed: all 92 xtask tests, formatting, relevant
all-target Clippy with warnings denied, dependency policy, source check and diff
checks. Remote CI for the repair is pending at this commit; the PR's current
checks and exact run records own subsequent integration facts. The legacy
`macos-quality` and `macos-tests` contexts run common checks on Linux; skipped GUI
checks are not native macOS qualification.

The read-only `release prepare-stage v1.0.0-rc.1` preflight exited 1:
`RC transition requires a qualified signed Beta and active feature freeze`.
`release status --json` still reports active Beta qualification pending, feature
freeze frozen, zero RCs and Stable authorization false. Source remains Beta.10.
Formal RC also needs current real-Host and consented cohort evidence, and exact
local macOS artifact/evidence handoff before full publication. These requirements
remain open; the model fixtures and unsigned local archive do not satisfy them.

The existing Beta.11 registry-testing transition preview passes without writes;
it is an available alternative, not an RC publication, and was not applied.
No tag or release was created. Request receipts and the unpublished RC notes draft
are retained in ignored `dist/rc1-publication-request/`. Source integration and
formal release completion remain separate outcomes.

## Registry Beta.11 publication, 2026-10-05

The owner changed the publication target to a new registry-testing Beta while preserving the
formal RC gates. The transactional stage tool prepared 27 controlled version surfaces and exact
internal pins in `a1c651c0259a5d386b5265ec97469539c5ad6fbf`; a second commit,
`a7fb733cc77d18bfcc9a8f007942107815b31249`, records its 21 nonautomatic freeze-exception paths.
[PR #247](https://github.com/jxpeng98/CanISend/pull/247) merged normally as `64c3bbd5f74c2ee2f436b764a427cf676841d136`.
No branch protections were changed. The merged tree equals the passing PR tree.

Fresh preparation checks passed: eight stage regressions, six version regressions, three npm
packaging tests, formatting, 129 local document links, dependency policy and the Cargo source check.
The note-body stage-neutral regression first rejected a newly added version token; removing the
token fixed the notes without changing the test policy. Protected
[PR Fast CI](https://github.com/jxpeng98/CanISend/actions/runs/37332307979) and
[PR dependency assurance](https://github.com/jxpeng98/CanISend/actions/runs/37332307827) passed.
Merged-main [Fast CI](https://github.com/jxpeng98/CanISend/actions/runs/37333167983) and
[dependency assurance](https://github.com/jxpeng98/CanISend/actions/runs/37333167972) also passed.
All four enabled Fast CI jobs passed; both GUI jobs were skipped. The legacy macOS-named shared
checks ran on Linux and supply no native macOS qualification.

A clean-tree GNU ARM64 `release-alpha` archive from the pre-merge PR head passed exact extraction,
installation/uninstall, documented dual-Pack workflow, backup/restore, Host resources and Agent v4
MCP lifecycle checks. Its SHA-256 is `cbae4fed0a30bd2eefa8b5830f6a349c62fcf6518257f42519ef0a8def6cf20d`.
That local archive is separate from the registry candidates built from the protected merge source.

[Registry run 37333356088](https://github.com/jxpeng98/CanISend/actions/runs/37333356088) completed all 13 enabled build, publish and installation jobs.
Cargo published eight packages at `1.0.0-beta.11`; its exact-archive checks, locked registry install
and nine packaged MCP tests passed. npm published `canisend@1.0.0-beta.11` on `next`, preserving
`latest` at Beta.3. PyPI published three native `1.0.0b11` wheels. npm and PyPI cover macOS ARM64
and Linux GNU x86_64/arm64. Independent readback matched the npm tarball and all three wheels to
retained CI candidates, checked public hashes, and verified all eight Cargo VCS identities against
the exact source above. Isolated Linux ARM64 installs and MCP checks passed; the installed PyPI
candidate is byte-identical to the public wheel. npm registry signature and attestation checks
also passed, and the installed source archive exactly matches `git archive` of the published commit.
The first independent per-version npm read returned 404 during index propagation; the consumer
index subsequently exposed the exact version and matched the retained package. The publication
workflow's bounded readback passed. See the [release record](../../../RELEASE.md)
for exact public digests and ignored `dist/beta11-registry-publication/verification.json` for receipts.

The requested registry-testing publication is complete. Historical qualification, provider dogfood,
contract freeze and cohort records remain unchanged. The latest qualified GitHub checkpoint is
still Beta.1. No tag or GitHub Release was created; full native/macOS handoff, current real-Host v4
acceptance and the consented cohort remain open before formal RC or Stable authorization.
