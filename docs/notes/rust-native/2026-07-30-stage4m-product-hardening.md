# Stage 4M qualification and product-hardening evidence

Date: 2026-07-30

Release line: `1.0.0-alpha.5` development tree

Scope: source implementation only; no tag, native package qualification, push, or release

## Outcome

Stage 4M hardens the integrated Application Workspace without creating another persistence layer.
Application Dossiers, the Content Catalog, metadata search, contextual Agent assistance, and
handoff context continue to rebuild from current SQLite rows and immutable artifact identities.
The optional private full-text index exists only inside one explicitly consented, bounded call.

The qualification slice adds:

- deterministic Catalog results across independent workspace opens and four concurrent readers;
- proof that a private full-text index is discarded before a later metadata-only search;
- control-character and unbounded-result malformed-input rejection;
- a revision race showing that an old intake preview cannot commit after a competing edit;
- assistance and handoff refresh from the new authoritative job revision;
- one combined serialization proof covering Dossier, registry, diagnostics, Catalog, routine
  search, Agent context, assistance, and handoff;
- desktop contracts for keyboard landmarks, semantic controls, live state, reduced motion,
  44-pixel critical targets, English/Simplified Chinese state coverage, bounded navigation memory,
  and application-scoped conversation continuity; and
- a separately invoked 128-application/256-entry read-model latency tripwire.

## Data, privacy, and recovery boundary

SQLite and referenced immutable blobs remain the only authority for this feature set. Catalog,
Dossier, guidance, and search results are computed read models. They add no database migration,
catalog table, private-body cache, backup entry, managed projection, or downgrade transform.

The private-body sentinel is discoverable only through a call that includes explicit
`PrivateReadConsent`. A subsequent metadata-only search returns zero sentinel matches, reports
zero private-body entries/bytes, and returns no snippet. A broader serialization test proves the
same sentinel is absent from routine coordination and diagnostic read models.

Recovery therefore means reopening or refreshing the selected application. If authoritative
integrity fails, the existing verified backup/restore procedure applies. Read models are never
copied into a backup or treated as rollback authority.

## Desktop continuity and accessibility

- The skip link and main landmark remain keyboard reachable.
- Primary application context controls and recovery actions have at least 44-pixel targets,
  visible focus rings, current-state semantics, and a labelled progressbar.
- Loading, success, error, conversation, and guidance changes expose status/alert/live-region
  semantics and reduced-motion-safe animation.
- Primary surfaces use native buttons, selects, and details; static regression coverage rejects
  click or key handlers on non-semantic containers.
- English and Simplified Chinese retain exact key parity and non-empty loading, recovery, empty,
  guidance, and conversation states.
- Malformed or oversized navigation memory falls back safely; an invalid last action is discarded
  without losing the valid workspace/application selection.
- Agent rendered state is isolated by runtime and application. Switching runtime for the same
  application preserves current guidance; changing application invalidates guidance while
  restoring only that scope's local rendered conversation.

CanISend still does not persist a parallel Agent transcript. Codex and Claude remain the
conversation, search, plugin, and session authorities.

## Bounded performance baseline

The explicit local benchmark creates 128 applications, each with one normalized source, producing
256 Catalog entries. Five warmed in-process samples on Apple Silicon macOS measured:

| Metric | Median | Maximum | Budget |
| --- | ---: | ---: | ---: |
| Dossier list | 29 ms | 33 ms | 2,000 ms |
| deterministic metadata-index search | 23 ms | 37 ms | 1,000 ms |

Fixture construction, compilation, linking, network access, and private-body indexing are outside
these measurements. The test is ignored by ordinary `cargo test` and runs only when explicitly
selected.

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p canisend-app --test stage4m_hardening` | passed: 3 |
| explicit `read_model_performance` | passed: 1; 128 jobs and 256 entries |
| `cargo test --workspace --locked` | passed: 269; ignored by policy: 5 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | passed |
| `pnpm --dir apps/canisend-desktop check` | passed: 0 errors, 0 warnings |
| `pnpm --dir apps/canisend-desktop test` | passed: 8 files, 42 tests |
| `pnpm --dir apps/canisend-desktop build` | passed in 2.52 seconds |
| `cargo run -p xtask --locked -- release check` | passed: 40 schemas, migrations through 13, Svelte parity 37/37 |
| `cargo fmt --all -- --check` and `git diff --check` | passed |

The production Svelte build kept Content Library at 16.04 kB, Workflow at 35.58 kB, Delivery at
20.17 kB, Agent at 47.65 kB, and the main minified JavaScript chunk at 476.32 kB, with no size
warning.

The macOS linker emitted its existing debug-only compact-unwind size warning for the large CLI
test binary. `mise` also could not write two optional cache files outside the sandbox. Neither
condition failed compilation, tests, or strict Clippy.

## Release boundary

No macOS App, DMG, CLI archive, tag, push, or public release is part of Stage 4M source
qualification. Exact native package and accessibility qualification remains an explicit release
checkpoint.
