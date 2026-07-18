# R11.2 cross-platform Beta contract freeze

**Date:** 2026-07-18

**Roadmap item:** R11.2 Beta qualification

**Status:** Portability defect reproduced and fixed locally; fixed-source CI qualification pending

## Failure evidence

The first post-signing Alpha release dry-run, GitHub Actions run `29636580836`, used exact source commit
`c7d1d4c79b5b9d0ca6f6ef4f91b14f1c354e3a03`. Source gates and signing readiness passed, as did the target workspace
tests on both macOS runners and both Linux runners. Windows job `88060396717` failed during `Test target workspace`
before release build or packaging. Its only failed test was
`tests::beta_agent_and_workspace_contracts_match_freeze`.

The frozen JSON schemas and SQL migrations were hashed as raw checkout bytes. A Windows checkout could therefore
replace LF with CRLF and report a contract drift even though the parsed schemas, migrations, names, and ordering were
unchanged. This was a portability defect in the freeze verifier, not evidence authorizing a new freeze digest.

## Corrected invariant

Frozen JSON and SQL remain exact, ordered, named contract inputs, but their text is canonicalized from CRLF to LF
before hashing. Invalid UTF-8 and bare carriage returns fail closed. `.gitattributes` also pins JSON and SQL checkout
line endings to LF, so fresh clones receive canonical source bytes while the verifier remains robust for existing
Windows worktrees.

The committed `release/beta-contract-freeze.json` was not regenerated. Real content, file-name, ordering, inventory,
or migration changes therefore still alter the digest and fail the gate.

## Local validation

- The focused `xtask` suite passes 12 tests, including LF/CRLF digest equivalence and rejection of a bare carriage
  return.
- `cargo clippy -p xtask --all-targets --locked -- -D warnings` passes.
- `cargo run -p xtask --locked -- release check` still reports 40 schemas, migrations frozen through 13, and an
  unchanged Beta contract freeze.

## Remaining qualification

Push the fix, require ordinary CI to pass at the exact commit, and run a new non-publishing Alpha release matrix.
The replacement matrix must pass the target workspace test and packaged-archive smoke on all five native runners.
The failed run remains retained as root-cause evidence and cannot qualify a release.
