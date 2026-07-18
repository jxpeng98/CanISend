# R11.3 feature-freeze enforcement preparation

**Date:** 2026-07-18

**Roadmap item:** R11.3 feature freeze

**Status:** Enforcement implemented in planned mode; activation waits for signed Beta qualification

## Gap

The qualification ledger already named an eventual baseline and allowed change classes, but it did not reconstruct
Git history after that baseline. A `frozen` string alone could not prove that feature work had stopped or that a code
change was reviewed as a release blocker.

## Enforcement

The new exception authority is empty and non-authorizing while the ledger remains `planned`. Once activated, the
release checker verifies the baseline commit and ancestry, enumerates every commit through `HEAD`, classifies only a
narrow documentation/release-evidence allowlist automatically, and requires exact per-commit path records for all
other changes. Exception commits, reasons, classes, paths, ordering, and canonical fields fail closed.

Activation is also machine-controlled. `xtask release activate-feature-freeze FULL_HEAD_COMMIT` previews the exact
two file hashes without writing. Explicit `--write` requires a clean Beta worktree, qualified signed Beta evidence,
canonical planned state, and a full baseline equal to current `HEAD`; it then keeps the ledger and exception
authority synchronized. A temporary Git repository test commits the automatic activation change and proves that
post-baseline history validation accepts it without inventing an exception.

## Boundary

This preparation does not freeze the current Alpha branch and does not close the R11.3 checkbox. Activation requires
a credential-qualified signed Beta and a separately reviewed baseline commit. Subsequent exception entries record
scope but do not replace tests or code review.

## Exact verification

Ordinary CI run `29641002363` passed all eight jobs at exact commit
`79afb488eb1f994ddb32a562b2e51d5ff47c9eb6`. This included the temporary Git-history regression fixture, complete
Rust and generated-property suites, release source gate, dependency policy, three operating-system recovery jobs,
and native render/documentation jobs. Windows also parsed both release-signing and package-lifecycle PowerShell
verifiers. This proves planned-mode enforcement is portable; it does not supply the future signed-Beta baseline.
