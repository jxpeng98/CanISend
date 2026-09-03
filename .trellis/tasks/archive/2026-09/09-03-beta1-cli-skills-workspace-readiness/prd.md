# Beta.2 CLI, Skills, and Workspace readiness

## Goal

Prepare the private `v1.0.0-beta.2` source candidate as a reliable, usable product by proving the
complete supported headless journey across the native CLI, canonical Agent v4 Skills, persistent
MCP, and Workspace v4. Extend the release policy only enough to support an exact sequential Beta
transition. Prefer deterministic automation and disposable local fixtures so routine
qualification requires as little human execution as possible.

## Product value

A user can create or discover a project Workspace, install the correct host resources, complete
evidence-bound work with the App closed, recover the Workspace, and reopen it without losing or
silently changing authority. Release owners receive one repeatable gate instead of an informal
manual checklist.

## Confirmed facts

- Exact public `v1.0.0-beta.1` is already qualified from source
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`; it is immutable release evidence
  (`release/qualification-ledger.json`).
- The repository is in an active post-Beta feature freeze at
  `acf25dc483643ca9be0210320775708da116b715`. Only documentation, release-blocker, and
  release-evidence changes are allowed without changing that authority
  (`release/qualification-ledger.json`).
- The checked-in operation registry currently validates exact inventories of 31 CLI, 129 Tauri,
  and 36 MCP leaves with zero compatibility bindings
  (`crates/canisend-contracts/operation-registry-v1.json`).
- Four integrity-bound Agent v4 Skills cover the ten canonical tasks once: `canisend-workspace`,
  `canisend-intake`, `canisend-materials`, and `canisend-review-export`
  (`crates/canisend-resources/resources/skills/`).
- The supported headless design deliberately uses the CLI for initialization, discovery, host
  setup, body-free reads, and recovery, then one persistent MCP process for guarded mutations.
  Adding one-shot CLI mutation approval would introduce a second approval authority and is
  deferred beyond 1.0 (`docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md:969`).
- Workspace v2/v3 and pre-v4 host resources are intentionally unsupported and fail before
  mutation. Workspace v4 database schema migrations remain supported and are applied as one
  immediate transaction (`crates/canisend-store/src/database.rs:101`).
- Existing automated coverage already proves CLI initialization, parent-marker discovery,
  mixed-Pack Applications, project/global host resource lifecycle, private Profile Source
  handling, backup/restore/repair, MCP guarded mutation failures, and a complete dual-Pack
  packaged smoke (`crates/canisend-cli/tests/` and `scripts/smoke_agent_v4_mcp.sh`).
- The planning baseline passed on 2026-09-03: operation registry check; 12 CLI binary contract
  tests; 4 MCP protocol tests; 16 embedded-resource/Skill tests; the typed Agent facade test; and
  the dual-Pack Agent v4 MCP smoke.

## Requirements

### R1 — Derive readiness from canonical contracts

Inventory the supported CLI, MCP, Skills, Workspace, Pack, and release surfaces from their owning
registries and tests. Do not classify an intentionally unsupported legacy or one-shot mutation
surface as a missing feature.

### R2 — Prove one supported App-closed journey

The automated gate must exercise the existing supported path:

1. initialize a clean Workspace v4;
2. discover it from an explicit path and nested project directory;
3. install and inspect the canonical project-scoped Codex Skills and MCP guidance;
4. create one Generic and one Academic Application in the same Workspace;
5. import minimum reviewed Profile data without leaking private bodies in body-free reads;
6. complete the guarded Agent v4 requirement, Plan, Deliverable, review, and local-export flow
   through one persistent MCP process for both Packs;
7. read the MCP mutations through the standalone CLI while the App remains closed; and
8. check, back up, restore to a fresh path, reopen, and re-check the Workspace.

### R3 — Keep Skills installation owned and recoverable

The gate must verify all four expected Skills, their ownership manifest and MCP configuration
guidance. Setup and status are idempotent; user-modified, unmanaged, symlinked, pre-v4, or
wrong-digest resources fail without partial replacement. Removal deletes only unchanged
manifest-owned Skill files and does not silently edit host MCP configuration.

### R4 — Preserve Workspace integrity

Missing markers, unsupported formats, incomplete migration history, newer schemas, migration
failure, stale revisions, token replay, wrong Application/Pack context, and invalid recovery
destinations must fail without partial authoritative mutation. A failed pending Workspace v4
schema chain rolls back as a unit and can be retried. Workspace v2/v3 import or in-place migration
must not be introduced.

### R5 — Minimize human qualification

Reuse and extend the existing Rust tests and `scripts/smoke_agent_v4_mcp.sh`; do not create a
second test framework. Tests use synthetic disposable fixtures, no third-party account, no live
private data, and no network dependency. Human work is limited to release decisions, user consent,
and the smallest native accessibility or real-host observation that cannot be proven
deterministically.

### R6 — Fix only reproduced readiness gaps

Every product-code change must correspond to a failing automated reproduction or an explicit
P0/P1 release invariant. Preserve application-facade ownership, stable CLI JSON/error/exit
contracts, single-use approval semantics, body-free output, local-only export, and
`submission_performed: false`.

### R7 — Respect release identity and freeze

Do not rewrite public Beta.1 artifacts, its tag, source identity, or historical evidence. Bind any
publishable changed bytes to a new release identity and record an exact feature-freeze exception
only for verified release-blocker files. A validation-only result must say explicitly that no new
public release was created.

### R8 — Support one fail-closed sequential Beta transition

Permit only an exact same-release-line `beta.N -> beta.(N+1)` transition after the current Beta is
qualified. Reject skipped, repeated, downgraded, cross-line, build-metadata, malformed, or
unqualified transitions. Preserve the exact qualified Beta.1 record in append-only `beta_history`,
make `beta` the pending active Beta.2 slot, keep feature freeze active, and defer RC.1. RC and Stable
qualification must continue to bind only to the latest active qualified Beta.

### R9 — Separate source preparation from publication

This task may prepare and merge the private Beta.2 source state. It must not create or move a tag,
push a release ref, dispatch a release workflow, publish a GitHub release or package, or record
Beta.2 as qualified. Those actions require a later, separately authorized release operation.

## Acceptance criteria

- [ ] `cargo run -p xtask --locked -- operations check` reports the exact compiled CLI, Tauri, and
      MCP inventories with no undeclared leaf or compatibility binding.
- [ ] The existing CLI binary and MCP protocol suites pass, including negative no-mutation cases.
- [ ] The existing resource suite proves that all four canonical Skills cover the ten Agent v4
      tasks once and remain byte/digest bound.
- [ ] The packaged Agent v4 smoke performs the complete dual-Pack App-closed flow and additionally
      proves project-scoped host setup/status plus exact Skill ownership in the same disposable
      Workspace journey.
- [ ] Nested-directory Workspace discovery, missing/legacy refusal, Workspace v4 migration
      rollback/retry, backup, restore-to-new-path, repair, and reopen checks pass automatically.
- [ ] CLI reads observe the exact state committed through MCP without a second storage or approval
      implementation.
- [ ] Tests retain no real Application body, credential, participant identity, provider token, or
      host-global configuration change.
- [ ] `cargo run -p xtask --locked -- release check` and the repository's Fast CI pass once on the
      final PR head; native release matrices are not repeated unless packaged/runtime bytes change.
- [ ] Any remaining manual checks are listed with their non-automatable reason; routine local
      verification otherwise runs unattended.
- [ ] Public Beta.1 evidence remains unchanged, and the final result identifies the exact target
      build or states that it is validation-only.
- [ ] A new accepted release decision and the machine policy permit only exact sequential Beta
      iteration; Beta.1-to-Beta.2 succeeds and skip, downgrade, cross-line, build-metadata, and
      unqualified-source cases fail without changing files.
- [ ] The Beta.2 source transition is dry-run comparable and transactionally writes or rolls back
      the existing controlled release file set; it preserves the qualified Beta.1 record in
      `beta_history`, resets only the active `beta` slot to pending, and leaves feature freeze
      active.
- [ ] RC.1 is neither prepared nor created, and no tag, workflow dispatch, public release,
      package publication, or Beta.2 qualification record is produced by this task.

## Out of scope

- Re-publishing, moving, or rewriting `v1.0.0-beta.1`.
- Publishing or qualifying `v1.0.0-beta.2`, creating `v1.0.0-rc.1`, or updating external package
  indexes.
- Adding direct one-shot CLI preview/commit mutations or a durable approval-token broker.
- Migrating Workspace v2/v3 or pre-v4 Skills in place.
- Adding a fifth Skill, a new workflow Pack, a new operation family, or a new test framework
  without a reproduced Beta blocker.
- Electron migration, broad desktop redesign, live provider automation, portal automation, or
  Application submission.
- Treating automated fixtures as invited-user or real-provider cohort evidence.

## Key decisions

- The product owner selected private `v1.0.0-beta.2` preparation on 2026-09-03 and explicitly
  deferred RC.1. A new ADR will amend only the release-sequence rule; accepted historical ADRs
  remain unchanged.
- `release/qualification-ledger.json` keeps `beta` as the active/latest slot and gains an ordered,
  append-only `beta_history` for earlier qualified Betas. The field may be absent before the first
  sequential transition and is canonicalized by the Beta.2 transition.
- Feature freeze remains active across Beta iteration. Product and policy changes require exact
  release-blocker exceptions; Beta-entry evidence is preserved rather than regenerated.
- The work stops at a merged private source candidate. Public Beta.2 qualification, cohort
  rebasing, tagging, workflow dispatch, and publication remain separate release decisions.
