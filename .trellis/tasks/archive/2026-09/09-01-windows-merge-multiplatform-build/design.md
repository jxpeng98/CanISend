# Design: Windows completion and freeze-safe integration

## Design objectives

1. Fix each reproduced Windows failure once at its existing owning boundary.
2. Preserve clean Workspace v4 and release-integrity fail-closed behavior.
3. Keep the diff dependency-free, workflow-free, and limited to demonstrated blockers.
4. Preserve exact evidence history and distinguish compilation, emulated/native execution, and
   protected CI evidence.
5. Keep the edit loop macOS-first and focused; broad suites and cross-compilation occur only at
   explicit integration boundaries.

## Branch and commit topology

The two existing evidence commits are siblings over the same protected base and remain immutable:

```text
origin/main@0d11c456
├── aaa4e98a  macOS validation evidence
└── 42036258  Windows validation handoff
```

Execution continues from `42036258`, merges local `main` to retain `aaa4e98a`, and then adds
reviewable commits in this order:

1. automatic Trellis planning/history integration;
2. `fix(release)`: tree-identical protected-merge accounting plus focused tests/docs;
3. `fix(windows)`: the five product blockers plus owning regressions;
4. `chore(release)`: exact ordered `release-blocker` entries for commits 2 and 3;
5. automatic Trellis/native evidence updates as needed before the PR is merged.

No rebase, squash, or cherry-pick may change the two existing evidence identities. Immediately
before opening or merging the PR, `origin/main` must be fetched and remain an ancestor of the branch
with no unreviewed nonautomatic drift.

## 1. Active Application selection

### Existing flow

```text
Workspace activation
  -> listJobs (retired, fails closed)
  -> listApplicationDossiers (supported)
  -> showJob (retired, fails closed)
  -> getApplicationDossier (supported)
  -> selected shell/navigation/application state
```

The bridge is correct to reject `job.list` and `job.show`. The shell incorrectly invokes both old
and new reads in the same path.

### Repaired flow

```text
Workspace activation
  -> listApplicationDossiers
  -> jobs = dossiers.map(dossier => dossier.job)
  -> remembered selection resolved in that list
  -> selected dossier + existing content catalog
  -> shell/navigation/application state
```

- `App.svelte` removes `listJobs` and `showJob` imports and all three active call sites.
- The selected object is the existing `ApplicationDossierReadModel`; refresh/select uses
  `getApplicationDossier` and replaces the matching list entry.
- Existing consumers narrow from legacy `JobDetailReadModel` to the job/dossier fields they
  actually use.
- The Application view uses `dossier.source_count` and the existing content catalog for source
  presentation; it does not fabricate `SourceRecord[]` merely to satisfy an obsolete type.
- `bridge.ts` remains unchanged and fail closed. Static contract coverage proves both the absence
  of old callers and the presence of the rejection stubs.

Expected frontend owners are `App.svelte`, `WorkspaceContextBar.svelte`,
`ApplicationsView.svelte`, `workflow-navigation.ts`, and their existing Vitest contract tests.

## 2. Platform-aware PATH guidance

The backend already exposes `ProductSummary.target_os` and `CliInstallStatus` already reports the
actual configuration location. `App.svelte` passes `product?.target_os` to the lazy Settings view.

The existing i18n message shape gains aligned English and Simplified Chinese variants:

- Windows: persistent current-user `HKCU\\Environment\\Path` and new-terminal activation advice;
- macOS: the existing managed `.zprofile` block wording;
- neutral: persistent user PATH without naming a registry key or shell profile.

`SettingsView` selects copy from the backend value, not browser user-agent inference. The location
label is also platform-accurate so a registry value is never described as a shell profile. The
existing static frontend contract test covers every language/platform key and selection branch;
no component-test framework is added.

## 3. Windows-safe CLI version probing

`probe_cli_version` is a read-only input check used before replacement and downgrade decisions.
The current child loop has a 750 ms limit, but joining the stdout reader can outlive that limit.

The repaired boundary has two defenses:

1. Under `#[cfg(windows)]`, read only the two-byte prefix and reject a candidate without `MZ`
   before `Command::spawn`.
2. Send the bounded stdout read result over `std::sync::mpsc`; receive it only within the remaining
   deadline. Timeout/wait error kills and reaps the child, discards output, and returns `None`
   without an unconditional reader join.

Output remains capped by `MAX_VERSION_OUTPUT_BYTES`. Unknown/malformed output cannot produce a
newer-version claim, cannot authorize a downgrade, and cannot mutate the destination before the
existing replacement decision. The existing malformed-byte replacement/restore test is the
end-to-end regression; valid human/JSON version tests and no-downgrade tests remain unchanged.

`Write`, `MAX_SHELL_PROFILE_BYTES`, `PATH_BLOCK_START`, and `PATH_BLOCK_END` receive `#[cfg(unix)]`
at their declarations/imports. No dependency or new process abstraction is introduced.

## 4. Portable Playwright startup

The package lifecycle already places `node_modules/.bin` on `PATH`. The committed
`webServer.command` changes only from nested `pnpm exec vite ...` to `vite ...`; host, port,
strict-port, timeout, URL, and teardown policy remain identical. The existing
`pnpm test:accessibility` script remains the regression in protected CI and on native Windows.

## 5. Freeze-safe protected merges

The verifier must not broadly ignore merge commits. It may suppress duplicate accounting only
when all of these hold:

1. the commit has at least two parents;
2. its full tree hash equals one non-first parent's full tree hash; and
3. the non-first parent's commits still appear in `BASELINE..HEAD` and pass the ordinary exact
   exception audit.

This describes an up-to-date GitHub PR merge that introduces no tree content beyond the audited PR
head. A merge tree unequal to every non-first parent is processed by the existing path audit and
therefore fails without an exact exception. Focused fixtures prove a tree-identical merge needs no
duplicate entry and a merge with independent resolution content remains controlled.

The rule is implemented inside the existing feature-freeze history validator and documented in
`docs/release/feature-freeze.md`; no general Git policy or release class changes.

## 6. Feature-freeze exception sequence

After each nonautomatic implementation commit:

1. resolve the full commit SHA;
2. obtain its exact sorted paths with the same first-parent `diff-tree` command used by policy;
3. append one ordered `release-blocker` entry with a bounded reason;
4. commit only `release/feature-freeze-exceptions.json`; and
5. run the source gate on the resulting complete branch and its PR merge ref.

The ledger commit is automatic and never records itself. Any amend/rebase after ledger creation
invalidates the hashes and must be corrected before push.

## 7. Verification cadence and test budget

### macOS implementation loop

Related changes are batched before checking them. The implementation loop does not run a full
workspace/frontend suite or Windows cross-build after each numbered design section.

- Freeze merge accounting extends one existing Git-history regression with the positive and
  negative cases, then runs that exact test once.
- The dossier selection, PATH copy, and Playwright command changes reuse the existing frontend
  contract/navigation tests; one focused Vitest invocation and one Svelte check run after the
  frontend batch is coherent.
- CLI probing reuses the existing replacement/restore and version-decision tests; one exact
  owning-test invocation runs after the Rust batch is coherent.
- Formatting and affected-package Clippy run once when the complete macOS implementation is ready
  to commit. One `xtask release check` closes that branch boundary.

No new test file or framework is added unless the existing owning test cannot express the
invariant. A focused failure expands only to the smallest sibling scope needed to diagnose it.

### Protected and post-merge boundaries

- The six protected Fast CI jobs own the full frontend/Rust source suite on the final PR head and
  again on the required merge-commit identity. Those repository-required runs are inspected, not
  duplicated locally.
- Native Windows runs only the platform-specific regressions and smoke paths that macOS cannot
  prove; it does not repeat every portable test.
- `cargo-xwin` runs once after merge on exact `main`: compile the workspace test graph without
  executing it, build the applicable release executables, then inspect PE headers and hashes.
- The existing desktop-platform qualification workflow owns post-merge native Windows/Linux
  package builds and runtime smoke. No new matrix is introduced.

## 8. Evidence boundaries

| Evidence | Proves | Does not prove |
| --- | --- | --- |
| macOS focused/source checks | changed portable logic and macOS host behavior | full matrix or Windows registry/process/runtime behavior |
| `cargo-xwin test --no-run` | Windows x64 MSVC test graph compiles | tests executed on Windows |
| `cargo-xwin build` + PE/hash inspection | exact cross-built architecture and bytes | native runtime, packaging, signing |
| native Windows branch checks | malformed `.exe`, registry, Playwright, Clippy, WebView2 behavior | public release support |
| protected Fast CI | required source integration on the PR/merge commit | release candidate qualification |
| desktop-platform qualification | nonpublishing native Windows/Linux package/runtime candidates | publication or native x64 hardware certification |

If Parallels becomes available, its result is reported as Windows 11 Arm with x64 emulation. The
current hosted Windows evidence is stronger for native x64 runner behavior and remains separately
attributed.

## Expected changed files

- `apps/canisend-desktop/src/App.svelte`
- `apps/canisend-desktop/src/lib/components/WorkspaceContextBar.svelte`
- `apps/canisend-desktop/src/lib/views/ApplicationsView.svelte`
- `apps/canisend-desktop/src/lib/views/SettingsView.svelte`
- `apps/canisend-desktop/src/lib/workflow-navigation.ts`
- existing frontend tests, principally `accessibility-contract.test.ts` and
  `workflow-navigation.test.ts`
- `apps/canisend-desktop/src/lib/i18n.ts`
- `apps/canisend-desktop/playwright.config.ts`
- `crates/canisend-app/src/cli_install.rs`
- `xtask/src/main.rs`
- `docs/release/feature-freeze.md`
- `release/feature-freeze-exceptions.json`
- task-local Trellis planning/evidence files

The list may shrink after type-checking. It may not expand into public support, packaging,
dependency, or workflow authorities without renewed review.

## Failure and rollback behavior

- Before merge, fix or revert only the bounded branch commits; never rewrite the two evidence
  commits after publication.
- A failed native Windows gate blocks merge.
- A failed or licence-blocked post-merge `cargo-xwin` run is recorded as a cross-toolchain blocker
  and never converted into a runtime claim or hidden by extra unrelated tests.
- If protected `main` changes, re-fetch and re-evaluate exact history before touching the ledger.
- After merge, rollback uses a protected `git revert` PR for the exact merge; no reset, force push,
  tag mutation, release publication, or control weakening.
