# macOS continuation handoff

## Purpose

Resume `fix/windows-desktop-validation` on a macOS host after the review-only Windows checkpoint.
The Windows checkpoint contains evidence and planning only; none of the five observed defects is
fixed by that commit.

## Checkout contract

1. Fetch `origin` and switch to `fix/windows-desktop-validation`.
2. Confirm the branch contains the review checkpoint based on
   `0d11c456d726990ff9940f404a6aebad24cf72fc`.
3. Do not silently rebase onto a different release baseline. If `main` has advanced, inspect and
   record the delta before deciding whether the task should be replanned.
4. Read, in order: `prd.md`, `design.md`, `implement.md`, `research/windows-validation.md`, and this
   handoff.
5. Record `uname -a`, macOS version, architecture, Rust version, Node.js version, pnpm version, and
   exact Git head before changing code.

## Planned implementation order

1. Replace active `job.list`/`job.show` callers with supported Application dossier projections,
   keeping the bridge stubs fail closed.
2. Make Settings PATH guidance branch on the backend-reported target OS, with aligned English and
   Simplified Chinese Windows/macOS/neutral variants.
3. Gate Unix-only Rust imports/constants correctly and make unmanaged CLI version probing bounded.
4. Replace Playwright's nested `pnpm exec vite` server command with the package-local Vite command.
5. Add the owning-layer regressions before running broader gates.

## macOS runnable gates

Use the repository-pinned toolchains and the exact commands from `implement.md`. At minimum:

```text
cargo fmt --all -- --check
cargo test -p canisend-app --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
pnpm format:check
pnpm check
pnpm test
pnpm build
pnpm test:accessibility
cargo run -p xtask --locked -- release check
```

Run a rebuilt macOS GUI smoke for workspace creation/opening, Applications, Settings PATH copy,
diagnostics, language switching, and cleanup. Attribute the result to the exact macOS architecture
(Apple Silicon or Intel); do not generalize across both macOS targets without both native results.

## Evidence macOS cannot replace

The following remain pending even if every macOS gate passes:

- execution of Windows-only malformed `.exe` and replacement/restore tests;
- strict Clippy/test execution under native Windows semantics;
- real `HKCU\\Environment\\Path` mutation and activation behavior;
- Playwright/Vite child-process launch under Windows command resolution;
- a rebuilt WebView2 GUI smoke on Windows.

A Windows-target `cargo check` or frontend unit test is useful static evidence, but it is not native
runtime qualification. Record native CI URLs/results later rather than describing a macOS pass as
complete Windows parity.

## Cleanup and delivery

- Use isolated temporary workspaces and remove their registry entries after GUI smoke.
- Do not publish/sign/notarize packages or update support claims in this task.
- Keep product fixes in a separate commit after the review checkpoint.
- Push follow-up commits to the same remote branch unless the maintainer explicitly requests a new
  branch or the baseline has materially diverged.
