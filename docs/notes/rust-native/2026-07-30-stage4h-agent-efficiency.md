# Stage 4H external-agent efficiency evidence

**Date:** 2026-07-30

**Source baseline:** post-`v1.0.0-alpha.5` working source; no version change, tag, push, package, or
public release is authorized by this record.

## Outcome

CanISend remains the local-first control plane and now prepares a tighter external Codex/Claude
workflow. One desktop action installs or safely upgrades host-discoverable project skills and
generates a body-free one-step launch command. The host still owns its conversation, search,
plugins, connectors, and transcript.

The former duplicated host-wide workflow is split into:

- `canisend-application` for context-first routing;
- `canisend-job-intake` for link/PDF/local intake, parse, and criteria;
- `canisend-application-materials` for evidence, matching, decisions, and drafts; and
- `canisend-application-review` for review, package, render, and export.

Codex installation writes eight managed files under `.agents/skills`, including generated
`agents/openai.yaml` metadata. Claude installation writes four `SKILL.md` files under
`.claude/skills`. A host-specific digest manifest makes reinstall a no-op, allows an unchanged
managed file to upgrade, and rejects local modifications or symlinked destinations.

The handoff contract retains the raw launch command, manual starting message, capability and
context fallbacks, and body-free context. It adds the recommended skill and a safely quoted
one-step command using the host's supported initial-prompt argument. The Svelte view makes that
command primary and shows skill status plus the current CanISend next action.

## Verification

- All four `SKILL.md` files passed the skill-creator structural validator.
- Generated Codex metadata satisfies the documented display, description, brand, and
  `$skill-name` default-prompt constraints.
- Svelte check completed with zero errors and zero warnings.
- Five frontend test files completed with 26 passing tests.
- The final production Svelte build completed in 2.63 seconds; the lazy Agent chunk remained
  separate.
- Resource tests passed five tests, including fresh install, idempotent reinstall, managed
  upgrade, user-edit refusal, and Codex/Claude layout coverage.
- Application tests passed 60 tests with one intentionally ignored public-endpoint test.
- Desktop Rust tests passed 32 tests.
- CLI binary contracts passed 20 tests, including real `agent assets install` execution and
  39-file Codex pack verification.
- The complete locked Rust workspace suite passed.
- Strict all-workspace, all-target, all-feature Clippy passed with warnings denied.
- The release source check passed 40 schemas, embedded resources, documentation, release policy,
  35 CLI/GUI operations, and 35/35 Svelte parity.
- `cargo fmt --all -- --check` and `git diff --check` passed.

The macOS linker emitted its existing debug-only compact-unwind size warning while linking the
large CLI test binary; it did not fail tests or strict Clippy. This change does not run an extended
native release matrix or package qualification because it is source-stage work.

## Remaining release evidence

- Run separately consented Codex and Claude real-provider sessions from a disposable
  CanISend workspace.
- Confirm project skill discovery, MCP preference with CLI fallback, host-owned session
  continuity, and a user-approval pause at a guarded mutation.
- Treat any next prerelease version, tag, package, push, or publication as a separate explicit
  release decision.
