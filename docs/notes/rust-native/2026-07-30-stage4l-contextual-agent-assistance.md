# Stage 4L contextual Agent assistance evidence

Date: 2026-07-30

Release line: `1.0.0-alpha.5` development tree

Scope: source implementation only; no tag, package qualification, push, or release

## Outcome

Stage 4L connects the selected application, its body-free Dossier, and sanitized Content Catalog
relationships to one contextual Agent starting point. CanISend now recommends the smallest
applicable installed workflow skill and the exact authoritative next action. Codex, Claude, or
another external host remains the primary conversation, reasoning, search, plugin, and session
authority.

The additive `agent.assistance` read model is available through:

- the Rust application facade;
- `canisend --workspace PATH agent assist --job JOB_ID --json`;
- the macOS Tauri command and typed TypeScript bridge; and
- the Svelte Agent workspace.

No generated contract schema changed and the MCP surface remains frozen at 13 tools.

## Data and privacy boundary

The assistance packet contains:

- the selected body-free Dossier and Agent Context;
- at most 64 sanitized content identities;
- artifact kind, ID, revision, SHA-256, status, privacy, provenance, and relationships;
- the recommended skill, application section, reason, and exact next action;
- five proposal targets and their current/upstream artifact identities; and
- explicit external-host/session/state ownership.

It omits content bodies and provenance locators. CanISend does not persist an Agent transcript.
The in-App runtime remains optional and read-only.

The malformed-input regression uses a private source whose filename and body contain sentinels.
Serialized assistance and handoff output contain neither sentinel nor a `locator` field.

## Proposal review boundary

Criteria, evidence, and plan candidates now use a two-step desktop review:

1. parse and lock the exact candidate in the current UI session;
2. show a bounded JSON-pointer diff against the loaded candidate, embedded artifact revisions,
   validation rules, and intended state change;
3. require a separate confirmation action; and
4. run the existing Rust schema, semantic, source-scope, and current-revision validation again
   during commit.

Evidence matches and draft results retain their stronger existing task boundary: Rust validates the
completion first, stores a bounded single-use preview token, and revalidates the task lease,
declared input revisions/hashes, candidate schema, and source spans during commit. The Svelte
review now exposes those input identities and the intended mutation before the commit button.

Successful commits refresh the selected Job, Dossier, and Content Catalog and invalidate any
previous handoff or assistance packet for that application.

## UX integration

- Agent guidance persists across host and Tab switches within the same workspace/job scope.
- Changing the workspace or application clears stale assistance.
- Returning to Agent Integration reloads guidance for the current Dossier revision.
- The UI distinguishes deterministic CanISend guidance from AI-authored content.
- English and Simplified Chinese copy cover loading, empty, stale/proposed/current states, diff,
  provenance, validation, and confirmation.
- Mature Lucide icons, semantic headings, visible focus controls, 44-pixel actions, and
  reduced-motion-safe loading indicators remain in use.

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p canisend-app -p canisend-cli -p canisend-gui --locked` | passed: app 66 + CLI 31 + GUI 35; expected release-only/network tests ignored |
| affected all-target strict Clippy | passed with warnings denied |
| `pnpm --dir apps/canisend-desktop check` | passed: 0 errors, 0 warnings |
| `pnpm --dir apps/canisend-desktop test` | passed: 7 files, 35 tests |
| `pnpm --dir apps/canisend-desktop build` | passed in 3.06 seconds |
| `cargo run -p xtask --locked -- release check` | passed: 40 schemas, 37/37 CLI/GUI and Svelte parity |
| `git diff --check` | passed |

The macOS linker emitted its existing debug-only compact-unwind size warning while linking the
large CLI test binary. It did not fail compilation or tests and is outside this source change.

## Next

Stage 4M owns catalog reopen/rebuild, stale and concurrent-edit regressions, broader accessibility
and navigation-continuity coverage, bounded large-fixture latency checks, and recovery/rollback
documentation. Native package qualification remains release-checkpoint-only.
