# Stage 3 Completion And 0.3.0 Release Implementation Plan

**Status:** In progress — Release A (`0.3.0.dev2`) and Task 13 accepted; Task 14 underway

**Date:** 2026-07-14

**Branch:** `feat/evidence-backed-draft-foundation`

**Current accepted baseline:** Task 12 at `cb115f1`

**Release sequence:** `0.3.0.dev2` on TestPyPI, then `0.3.0b1` on TestPyPI and PyPI after Stage 3 acceptance

## Goal

Publish the accepted Task 12 baseline as an installable TestPyPI development checkpoint, complete the remaining
Stage 3 cross-document review and aggregate package-readiness boundaries, and publish the accepted Stage 3 result as
the `0.3.0b1` PyPI prerelease. Neither checkpoint may imply submission, portal, or semantic-truth certification.

## Fixed Decisions

- Keep Python, the private file workspace, current CLI, stage runtime, and `canisend.agent/v1` as the platform.
- Use `scripts/release.sh`; do not upload distributions or create GitHub releases manually.
- Retain `test/v0.3.0.dev1` as an immutable failed candidate. It failed remote CI before any TestPyPI upload because
  ANSI-styled option names exposed terminal-sensitive assertions.
- Publish its fixed successor, `test/v0.3.0.dev2`, to TestPyPI only. It may originate from the reviewed feature
  branch.
- Publish `v0.3.0b1` only after Tasks 13–15 are accepted. The release workflow must publish to TestPyPI and pass its
  installed-package smoke before promotion to PyPI and creation of a GitHub prerelease.
- A pushed tag or published package version is immutable. Never move or reuse `0.3.0.dev1` or `0.3.0.dev2`; any
  later retry increments `.devN`.
- Document readiness, cross-document review, application-package readiness, rendering readiness, and submission are
  separate states.
- Cross-document findings and correction proposals are Tier 2 private data. Control responses remain body-free.
- A correction proposal never mutates an authoritative Draft. Revision must reuse guarded Draft candidate validation
  and promotion for the targeted document.
- Stage 4 discovery expansion, Stage 5 runtime convergence, portal work, uploads, and stable `1.0.0` are non-goals.

## Delivery Sequence

```text
Task 12 accepted
  -> Release A: 0.3.0.dev2 TestPyPI checkpoint
  -> Task 13: cross-document consistency Review
  -> Task 14: aggregate package review decisions and readiness
  -> Task 15: Stage 3 exit, remote CI, distribution acceptance
  -> Release B: 0.3.0b1 TestPyPI -> smoke -> PyPI prerelease
```

## Release A: TestPyPI Development Checkpoint

### Preconditions

- [x] Task 12 has full local, cross-version, distribution, clean-wheel, and Typst evidence.
- [x] The working tree is clean and the canonical/workspace skill trees match.
- [x] `test/v0.3.0.dev1` is retained at `0aede4f`; run 29364176031 failed before upload.
- [x] The ANSI-sensitive CLI assertions found by that run are reproducible and fixed without changing CLI behavior.
- [x] Before release, `test/v0.3.0.dev2` was absent locally, remotely, and on TestPyPI.
- [x] The remote feature branch exists and can advance to the fixed reviewed candidate.
- [x] GitHub authentication and the tag-triggered release workflow are available.
- [x] The Stage 3 completion/release plan is committed with the release candidate.

### Execution

1. Run `scripts/release.sh test --version 0.3.0.dev2` without skipping local checks.
2. Let the script update `pyproject.toml`, `src/canisend/__init__.py`, `.codex-plugin/plugin.json`, README version
   examples/badge, and `uv.lock`.
3. Require the script to pass mirror, full pytest, build, Twine, packaged-resource, and clean-wheel Decision Spine
   smoke checks before it commits the version bump.
4. Let the script push the exact feature-branch candidate, verify the remote head, and push
   `test/v0.3.0.dev2`.
5. Monitor `.github/workflows/release.yml` through TestPyPI publication and TestPyPI installation smoke.
6. Independently install `canisend==0.3.0.dev2` from TestPyPI with PyPI as dependency fallback and verify version,
   CLI help, workspace initialization, resource availability, and the packaged Decision Spine smoke.

### Failure And Recovery

- Before a tag is pushed, fix the local failure and rerun the release script from a clean tree.
- After the tag is pushed but before a TestPyPI upload, fix the release infrastructure and rerun the failed workflow
  only when the immutable candidate remains identical.
- After any successful TestPyPI upload, do not reuse the version. Record the failure, fix forward, and use the next
  `.devN` version.
- Do not create a `v*` tag during Release A; the test channel must never publish to PyPI.

### Release A Validation Snapshot

Release A was accepted on 2026-07-14:

- `test/v0.3.0.dev1` remains an immutable failed candidate at `0aede4f`; GitHub Actions run
  [29364176031](https://github.com/jxpeng98/CanISend/actions/runs/29364176031) failed during tests before any
  TestPyPI upload. The failure exposed terminal-colour-sensitive assertions and did not change CLI behavior.
- The fixed `test/v0.3.0.dev2` candidate is `ec42ca6876f2fd74fc1224e08348b8e58a078c56`.
- The unskipped local gate passed 1,072 tests on Python 3.14.2, package build, Twine metadata validation, packaged
  resource validation, and a clean-wheel Decision Spine smoke with 10 successful stages and 18 mutation receipts.
- GitHub Actions run [29365962138](https://github.com/jxpeng98/CanISend/actions/runs/29365962138) passed the Python
  3.12 build/test job, TestPyPI Trusted Publishing, and installed-package TestPyPI smoke. PyPI publication and the
  GitHub Release jobs were correctly skipped for the test channel.
- An independent fresh Python 3.12.12 environment installed `canisend==0.3.0.dev2` without cache from TestPyPI with
  PyPI as the dependency fallback. Version metadata, packaged schemas, CLI help, workspace initialization, Doctor,
  Agent capabilities, and the 10-stage/18-receipt Decision Spine smoke all passed.
- TestPyPI wheel: `canisend-0.3.0.dev2-py3-none-any.whl`, SHA-256
  `1dbb3619380d04b794f4d431e9f1fbe2f4707fe331f40fdb573c022f5c140e4a`.
- TestPyPI source distribution: `canisend-0.3.0.dev2.tar.gz`, SHA-256
  `fd132bd55376062c4eba9b8c043c19a10ba359e3bf8a330158a0bfd2c980f0ad`.
- GitHub reported that several JavaScript actions still target Node 20 and are being forced onto Node 24. This is a
  non-blocking release-infrastructure follow-up for Task 15, not a package or publication failure.

## Task 13: Cross-Document Consistency Review

### Objective

Create one deterministic, reviewable package-level consistency result over exact current document instances without
rewriting Cover Letter, Research Statement, Brief, Evidence, or their user-owned decisions.

### Contract And Ownership

- Accept ADR-021 for aggregate Review identity, privacy, input selection, correction proposals, and invalidation.
- Add a strict versioned package-consistency model/schema and one core-owned Tier 2 output, provisionally
  `package_review_findings.json`.
- Bind the result to the exact Required Document Plan, Application Brief, Parsed Job, current Draft/Review/readiness
  hashes for every selected document, and the normalized document IDs/kinds.
- Keep finding messages, compared Claim bodies, and correction proposal bodies out of TaskSpec, state, receipts,
  errors, and AgentResponse.
- Make correction proposals document- and Claim-scoped. Applying one requires a new guarded Draft candidate for the
  target document; no aggregate reviewer writes an authoritative document.

### Deterministic Review Rules

- Detect exact contradictory factual values that can be proven from structured Claim/job/Brief references.
- Detect violations of confirmed Brief exclusions and document-specific required-section omissions.
- Detect duplicate strong factual Claims whose Evidence receipts disagree or have become unavailable.
- Surface semantic proportionality, tone, narrative alignment, and non-exact conflicts as explicit human-review
  findings rather than pretending deterministic certainty.
- Treat unavailable, stale, blocked, omitted-required, or unimplemented required documents as package blockers.

### Runtime And Agent Surface

- Register one deterministic package-review stage with declared read/write ownership and reconstructable state.
- Expose body-free status, counts, reason codes, exact input/output hashes, consents, and next actions.
- Invalidate only the package-review instance when a bound document, Brief, plan, or readiness receipt changes.
- Preserve per-document Draft/Review state and existing compatibility views byte for byte on rejected or stale runs.

### Acceptance

- Model/schema parity, current-input, output-drift, stale/tamper, recovery, cache, and private-body tests pass.
- Dual-document fixtures prove exact conflict detection and safe semantic-review deferral.
- A changed Cover Letter invalidates package Review without invalidating a still-current Research Statement Review.
- Supported Python versions, packaged resources, source smoke, and clean-wheel smoke pass.

### Task 13 Validation Snapshot

Task 13 was locally accepted on 2026-07-14:

- ADR-021, the strict `PackageReviewFindingsV1` contract/schema, deterministic `package_review` stage, dynamic
  document fan-in, and body-free AgentResponse surface are implemented.
- The development-interpreter full suite passed 1,079 tests; the final aggregate Review and release-smoke focused
  suite passed 24 tests after adding output-drift and immutable-receipt reconstruction coverage.
- Dual-document fixtures proved exact repeated-assertion/Evidence-receipt conflict detection, document/Claim-scoped
  correction proposals, explicit semantic-review deferral, cache reuse, and selective aggregate invalidation.
- Source and clean-wheel Decision Spine smokes both passed with 11 successful stages and 18 immutable user-mutation
  receipts. The aggregate stage preserved unsupported required-document blockers in the packaged fixture.
- `uv build`, Twine metadata validation, packaged-resource validation, generated-schema parity, Python bytecode
  compilation, `git diff --check`, and canonical/workspace skill-mirror validation passed.
- Cross-version reruns and the final release gate remain Task 15 work; Task 13 does not claim application-package,
  rendering, or submission readiness.

## Task 14: Aggregate Package Review Decisions And Readiness

### Objective

Add guarded user decisions for package-level findings and derive application-package readiness only from exact
current required-document and consistency-review receipts.

### Contract And Ownership

- Accept ADR-022 for user-owned package finding decisions, non-waivable blockers, aggregate readiness, and APP-Q
  integration.
- Add strict `package_review_dispositions.yaml` with explicit-consent revision/hash compare-and-swap, immutable
  claims/receipts, interrupted-publication recovery, and explicit reset against a changed package Review.
- Add `ApplicationPackageReadinessV1` with body-free states such as `blocked`, `review_required`,
  `revision_required`, and `reviewed`.
- Bind readiness to the Required Document Plan, every required document readiness hash, package Review hash, and
  package disposition hash. Optional documents may be reported but cannot silently become required.

### Gate Semantics

- A required omitted, unavailable, stale, blocked, unreviewed, or executor-unavailable document prevents package
  readiness.
- Every current non-blocker package finding requires an explicit decision; blockers cannot be accepted or waived.
- Per-document `reviewed` is necessary but insufficient for application-package `reviewed`.
- Application-package `reviewed` is necessary but insufficient for rendering, manual submission, or proof of
  submission.
- Extend `check-package` to rederive the aggregate contract and bind exact inputs without absorbing optional
  standalone documents that are outside the plan.

### CLI And Agent Surface

- Add body-free `package-review status|init|update` operations or an equivalently explicit aggregate namespace.
- Require explicit user-owned-write consent, current revision/hash, one strict patch, and deterministic next actions.
- Keep Claim, finding, rationale, and correction bodies out of ordinary CLI/AgentResponse output.

### Acceptance

- Cross-document CAS isolation, stale basis, non-waivable blocker, partial decisions, revision requests, reset,
  recovery, and privacy tests pass.
- `check-package` cannot pass from Cover Letter readiness alone or from a stale aggregate receipt.
- Existing single-document package behavior remains readable and fails closed until aggregate prerequisites exist.
- Supported Python versions, installed-wheel smoke, Typst protection, and Git tracking remain compatible.

## Task 15: Stage 3 Exit And `0.3.0b1` Candidate

### Objective

Close Stage 3 with end-to-end evidence that reviewed documents, aggregate Review, package decisions, compatibility
views, and release artifacts remain consistent across fresh agent hosts and installed distributions.

### Completion Work

- Finish ADR, schema, canonical skill, compatibility mirror, README, changelog, file-contract, privacy, recovery, and
  migration documentation for Tasks 13–14.
- Add a synthetic dual-document fixture that reaches aggregate package `reviewed` without claiming submission.
- Prove restart, cache, selective invalidation, stale/tamper, interrupted mutation, edited Typst, and legacy workspace
  behavior.
- Run full Python 3.11–3.13 plus development-interpreter tests, build, Twine, packaged-resource checks, source smoke,
  clean-wheel smoke, and real Typst compilation.
- Push the reviewed Stage 3 candidate and require successful remote CI before creating the prerelease tag.

### Stage 3 Exit Criteria

- Every strong factual Claim resolves to current Evidence.
- Unsupported or contradictory facts and missing required documents are executable blockers.
- The same confirmed Brief produces inspectably consistent constraints across documents.
- No worker/reviewer writes an authoritative, user-owned, compatibility, profile, or run target outside its declared
  guarded boundary.
- Aggregate readiness is reproducible from exact receipts in a fresh supported host.
- All local and remote release gates pass with no private bodies in logs, reports, or release metadata.

## Release B: `0.3.0b1` PyPI Prerelease

### Preconditions

- [ ] Tasks 13, 14, and 15 are locally accepted and recorded in the Stage 3 implementation plan.
- [ ] The candidate branch is reviewed, clean, pushed, and passes remote CI.
- [ ] `v0.3.0b1` is absent locally, on GitHub, TestPyPI, and PyPI.
- [ ] TestPyPI and PyPI Trusted Publishing environments remain configured.

### Execution

1. Run `scripts/release.sh beta --version 0.3.0b1` without skipping local checks.
2. Require the tag workflow to publish to TestPyPI first and pass installed-package smoke.
3. Allow promotion to PyPI only after the TestPyPI job succeeds.
4. Require a GitHub prerelease attached to the exact `v0.3.0b1` candidate.
5. Independently install from PyPI and run the packaged Stage 3 smoke in a fresh Python 3.12 environment.
6. Record immutable tag, commit, workflow run, TestPyPI/PyPI project links, hashes, and test evidence in the plan.

## Explicit Non-Goals

- stable `0.3.0` or `1.0.0` publication;
- discovery-source expansion, Lead v2, Greenhouse, Lever, email import, or ranking;
- complete Stage 5 runtime/legacy convergence;
- automatic correction application, portal navigation, upload, submission, or sensitive declarations;
- claiming semantic truth, employer acceptance, or successful submission from package readiness.
