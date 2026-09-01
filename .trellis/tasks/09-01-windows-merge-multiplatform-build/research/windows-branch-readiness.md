# Windows branch readiness

**Inspected:** 2026-09-01

## Refs

- Local `main`: `aaa4e98a8e788fc34d86357efc9749e795e5bd4f`.
- `origin/main`: `0d11c456d726990ff9940f404a6aebad24cf72fc`.
- `origin/fix/windows-desktop-validation`:
  `42036258fa07fe25e9b42a358b7e64088cbd502e`.
- Merge base of the Windows branch and `origin/main`:
  `0d11c456d726990ff9940f404a6aebad24cf72fc`.

The local and Windows commits are sibling documentation-only commits over the same protected
baseline. There is no content conflict in principle, but that does not make the Windows product
follow-up complete.

## Branch contents

The Windows branch contains one commit, `docs(trellis): record Windows validation handoff`, with ten
new `.trellis/` task/journal files and no product source, tests, release metadata, or artifacts.

The handoff records five reproduced gaps:

1. `App.svelte` invokes retired `job.list` and `job.show` operations.
2. Windows Settings describes macOS `.zprofile` mutation instead of the user PATH registry value.
3. Four Unix-only imports/constants fail strict Windows Clippy.
4. Malformed pre-existing `.exe` bytes can hang CLI version inspection and replacement tests.
5. Playwright's nested `pnpm exec vite` server command is not portable to the validated Windows
   package-manager process environment.

The remote task is still `in_progress`. Its review-checkpoint evidence is complete, while every
product-follow-up acceptance item remains open.

## GitHub state

- No pull request exists for `fix/windows-desktop-validation`.
- The branch head has no commit statuses and no check runs.
- It is therefore not a protected, green integration candidate.

## Feature-freeze protected-merge audit

The freeze verifier reconstructs every commit in `BASELINE..HEAD` and currently calculates each
commit's nonautomatic paths with:

```console
git diff-tree --first-parent -m --no-commit-id --name-only -r COMMIT
```

A normal GitHub merge commit therefore repeats the PR's changed paths against protected `main`,
even though the PR's own source commits were already traversed and exception-bound. The most recent
protected merge, `0d11c456d726990ff9940f404a6aebad24cf72fc`, demonstrates the relevant shape: its tree
`43ceccaded7b5f6dbfc449ba258792d12ff298f6` exactly equals second-parent PR head
`1ca0949eec3e9a02dcf81e7392da5c83eb96c236`, while `diff-tree` still reports the PR paths on the
merge commit.

That is harmless for the current documentation-only history but would require an impossible
self-referential exception for this product PR's generated merge SHA. The narrow safe rule is to
skip duplicate exception accounting only when the merge commit's entire tree equals a non-first
parent tree. All commits on that parent remain traversed and audited. A merge tree that differs
from every non-first parent, including conflict resolution or any newly introduced content,
continues through the existing exact-path exception logic.

## Recommendation

Do not merge the branch into `main` as if the Windows work were complete. Continue the same branch
on macOS, preserve both documentation checkpoints, repair the five Windows gaps and the narrow
protected-merge audit blocker, record exact feature-freeze exceptions, and obtain macOS plus native
Windows branch evidence. Open and merge a protected PR only after its merge-ref source gate and all
required checks pass. Run the requested `cargo-xwin` and native package verification again on the
exact merged `main` identity.
