# CanISend release-pipeline optimization roadmap

**Status:** Stage 2 implemented locally; next candidate measurement remains pending

**Decision date:** 2026-07-26

**Scope:** Native Rust test, package, candidate, and tag-promotion workflows

## 1. Outcome

Reduce ordinary Alpha feedback time and eliminate duplicate release compilation without weakening
the exact-byte release boundary. The optimized system must preserve:

- one complete Rust source gate per candidate;
- native packaging and extracted-archive smoke evidence for every claimed target;
- exact source, tag, manifest, checksum, signing, and GitHub provenance binding;
- draft-download smoke tests before publication;
- fail-closed handling of expired, missing, mismatched, or partly replaced artifacts; and
- the scheduled/native gates that own extended assurance.

The optimization is ordered. A later stage starts only after the preceding stage is measured and
its integrity boundary remains green.

## 2. Measured baseline

The first `v1.0.0-alpha.1` release exposed two different costs:

| Run | Purpose | Wall time | Dominant work |
|---|---:|---:|---|
| `30178680287` | non-publishing candidate | about 29m47s | Windows native build, Intel macOS build |
| `30179583856` | tag release before recovery | about 39m34s | the same native compilation repeated, then draft smoke |
| `30180930141` | publication recovery | about 1m55s | release-state recovery only |

Representative tag-run job times were about 28m41s for the native matrix wall clock, 13m10s for
the Apple Silicon GUI package, 7m41s for source gates, and 1m42s for assembly. Rust is not the
problem by itself: the workflow deliberately compiled multiple target graphs, compiled test
harnesses on every target, built the Intel GUI, and then repeated all build work after the tag.

## 3. Performance budgets

These are engineering targets, not release-integrity exceptions:

| Milestone | Candidate target | Tag-to-public target | Rule |
|---|---:|---:|---|
| Stage 1 | no regression from baseline | 8–15 minutes | tag promotion compiles no product binaries |
| Stage 2 | 15–22 minutes | 8–15 minutes | full workspace suite runs once, native runners keep owned tests |
| Stage 3 | 12–18 minutes for Alpha | 8–15 minutes | Intel GUI evidence leaves ordinary Alpha candidates |
| Stage 4 | improve warm runs by at least 20% | no regression | shared compiler cache is additive, never authoritative |
| Stage 5 | measured after profile change | no regression | faster Alpha profile is visibly distinct from RC/Stable |

If a stage misses its budget, retain the integrity change and investigate the remaining critical
path before starting the next optimization.

## 4. Stage 1 — Build once, promote identical bytes

**Goal:** A future-tag workflow dispatch builds the complete candidate once; an annotated tag
promotes those exact bytes without compiling, signing, packaging, or attesting replacement bytes.

### Candidate contract

- `workflow_dispatch` accepts an exact future tag that matches the Cargo workspace version.
- The future tag must not exist yet.
- Source gates, signing readiness, five native CLI archives, Apple Silicon GUI packaging, Intel
  compile-only evidence, assembly, checksum verification, and attestations run once.
- The complete `canisend-TAG-release-assets` artifact is retained for 30 days.
- Candidate mode has read-only release-content permission and cannot create a GitHub release.
- Candidate and promotion runs for the same tag are serialized without canceling the in-progress
  candidate, so an early tag push waits for candidate completion instead of racing artifact
  discovery.

### Promotion contract

- A push must be an annotated tag with a version matching the workspace.
- The workflow resolves the tag's commit and queries successful `workflow_dispatch` runs from
  `release.yml` by exact `head_sha`.
- It accepts only the exact named, unexpired complete candidate artifact.
- `xtask release verify-candidate TAG COMMIT DIRECTORY` repeats the complete release verification
  and additionally requires `manifest.source.commit == tagged commit`.
- Every asset's attestation must match the repository, signer workflow, and source digest.
- Promotion may create or safely resume only a draft with no unknown asset names.
- All release assets are uploaded from the downloaded candidate directory.
- The existing six-target draft-download smoke matrix remains the publication gate.
- A body-free promotion record preserves candidate run/artifact IDs, source commit, manifest
  digest, asset count, and the explicit fact that promotion did not recompile.

### Failure and recovery

- No candidate, an expired artifact, a lightweight tag, a source mismatch, an unknown draft
  asset, an incomplete checksum set, or invalid provenance fails closed.
- A partial draft upload can be retried only when all existing draft asset names belong to the
  verified candidate; expected assets are then re-uploaded from the candidate with clobber.
- Published releases are never overwritten by candidate promotion.
- An expired candidate is not reconstructed file by file. Prepare a new future version and rerun
  the full candidate matrix before tagging it.

### Stage 1 tasks

- [x] Add exact source-commit candidate verification to `xtask`.
- [x] Add positive, mismatch, and malformed-commit regression coverage.
- [x] Split the native workflow into candidate and promotion branches.
- [x] Remove draft creation from candidate assembly.
- [x] Add exact-run discovery, unexpired-artifact selection, and cross-run download.
- [x] Reverify checksums, manifest identity, signer workflow, and source digest during promotion.
- [x] Add resumable draft upload and body-free promotion evidence.
- [x] Align post-smoke publication recovery with candidate-promotion job ownership.
- [x] Document the operator and signing sequence.
- [x] Run formatter, focused `xtask` tests, Clippy, workflow syntax checks, and `release check`.
- [x] Review the workflow diff and commit Stage 1.
- [ ] Exercise the new path with the next future version: successful candidate first, then
  annotated tag, then confirm the tag run has no product build jobs.

**Exit:** Source gates prove the contract locally. The next release proves that the candidate and
public release manifest hashes are identical and that the tag run performed no product compile.

## 5. Stage 2 — Partition tests by ownership

**Goal:** Stop compiling the full workspace test graph on all five native targets while retaining
the smallest native test that proves each platform claim.

### Test ownership

| Gate | Required work |
|---|---|
| Fast CI | formatter, affected crates, normal workspace tests, relevant Clippy |
| Candidate source gate | full locked workspace suite once on Linux, property contracts once, release contracts, dependency policy |
| Each native archive job | locked release build, package, extracted-archive version/doctor/capability smoke, native signing checks where applicable |
| Linux GNU | existing release performance and full synthetic workflow budgets |
| macOS app | app assembly, signature/layout verification, packaged CLI checks, extracted-app smoke |
| RC/Stable qualification | exact platform lifecycle, upgrade/rollback, documentation/uninstall, accessibility, package-manager, and clean-tag evidence |

### Implementation tasks

- [x] Inventory every target-sensitive test and assign it to one explicit owner.
- [x] Replace `cargo test --workspace --target TARGET` in the release matrix with the bounded
  platform-owned test or smoke set.
- [x] Keep a scheduled cross-target test workflow for regressions that are valuable but not
  release-critical.
- [x] Add source checks that reject a native target with neither a platform test nor archive smoke.
- [x] Record per-job build, target-specific validation, package, and smoke duration in a body-free
  timing artifact.
- [ ] Compare two candidate runs before accepting the new partition.

**Exit:** No claimed native behavior loses its owning test, while Windows and Intel runners no
longer compile unrelated workspace test binaries.

## 6. Stage 3 — Move Intel GUI compilation out of ordinary Alpha candidates

**Goal:** Remove the longest non-runtime GUI compilation from the fast Alpha loop without implying
Intel GUI support.

### Implementation tasks

- [ ] Change the Alpha release manifest so it does not require per-candidate Intel GUI evidence.
- [ ] Retain the truthful absence of an Intel GUI archive and runtime support claim.
- [ ] Add a scheduled Intel compile workflow for development regressions.
- [ ] Restore exact-commit Intel GUI compilation as an RC/Stable qualification gate, or earlier
  when the product begins claiming an Intel GUI.
- [ ] Update release notes, support policy, verifier, and negative tests atomically.

**Exit:** Alpha release integrity is unchanged for published artifacts; Intel compile evidence is
owned by a scheduled or stage-specific gate instead of every ordinary candidate.

## 7. Stage 4 — Add `sccache` as a non-authoritative accelerator

**Goal:** Reduce repeated dependency and code-generation work across hosted runners.

### Implementation tasks

- [ ] Pin an `sccache` installation action or exact binary version and checksum.
- [ ] Use target, Rust version, profile, and relevant feature state in cache separation.
- [ ] Keep `Cargo.lock`, source, manifest, checksums, native signatures, and attestations as the
  authority; a cache hit is never evidence.
- [ ] Capture hit rate, compile requests, cache errors, and time saved without private paths or
  source bodies.
- [ ] Fall back to normal Cargo compilation when the cache is unavailable.
- [ ] Compare cold, warm, and intentionally invalidated candidates.

**Exit:** Warm candidate critical-path time improves by at least 20%, cold builds remain correct,
and cache failure cannot block or weaken release verification.

## 8. Stage 5 — Introduce an Alpha-fast release profile

**Goal:** Avoid spending RC/Stable optimization time on frequently changing Alpha binaries.

### Proposed profile

Add a named profile inheriting from `release`, then explicitly trade some compile-time
optimization for iteration speed, initially by disabling LTO and increasing code-generation units.
The canonical `release` profile remains the RC/Stable profile.

### Implementation tasks

- [ ] Measure current Alpha archive size, startup, render, and synthetic workflow budgets.
- [ ] Add a named profile such as `release-alpha`; do not silently redefine `release`.
- [ ] Select the profile from the validated release stage in packaging scripts/workflow.
- [ ] Record the selected profile in CLI, GUI compilation, and package qualification evidence.
- [ ] Update manifest verification so profile identity cannot drift.
- [ ] Compare compile time, binary size, cold start, and runtime budgets on Apple Silicon,
  Intel macOS, Windows, GNU, and musl.
- [ ] Keep RC/Stable on the canonical optimized profile.

**Exit:** Alpha builds are materially faster, every artifact records its profile, runtime budgets
remain green, and RC/Stable optimization is unchanged.

## 9. Measurement and review protocol

For each stage:

1. run focused local verification and the source gate;
2. record workflow run IDs, runner images, cache state, job start/end times, and conclusions;
3. compare critical-path wall time rather than adding parallel job durations;
4. verify manifest digest continuity across candidate, draft, and public download boundaries;
5. confirm no new publication authority or platform claim was introduced; and
6. update this checklist before starting the next stage.

The ordinary edit loop does not run extended fuzzing, dependency assurance, or the complete native
qualification suite. Those remain owned by scheduled or exact-release workflows.

## 10. Immediate next action

Finish Stage 1 local verification. Do not trigger another expensive native candidate for the
already published `v1.0.0-alpha.1`. Exercise the candidate-to-tag path on the next sequential
version after its source version and release notes are prepared.
