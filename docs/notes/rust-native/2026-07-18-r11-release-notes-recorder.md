# R11 final RC release-notes review recorder

**Date:** 2026-07-18

**Roadmap item:** R11.3 release notes and rollback guidance

**Status:** Recorder implemented; real final RC review pending

## Gap found

The qualification ledger previously required `release_notes.status` to become `stable-final`, but the Stable stage
transition itself set that status. The policy said that a human final-RC content review was mandatory, yet no
machine-bound evidence distinguished that review from the mechanical version transition.

## Control added

`xtask release record-release-notes-qualification` is a dry-run-first, ledger-only recorder. It accepts only the
current and latest successfully recorded RC, fully verifies its downloaded release assets, requires the published
and checked-in release notes to be byte-identical, and binds all of the following:

- the RC tag, signed matrix run, manifest source commit, and release-manifest digest;
- the stage-neutral release-note body digest and rollback-guide digest;
- a syntactically valid public GitHub reviewer login;
- canonical evidence text covering public issues, assets, limitations, and package-channel state.

Write mode requires a clean worktree and changes only `release/qualification-ledger.json`. Stable qualification
rejects missing, stale, earlier-RC, anonymous, or hand-extended evidence. Sequential RC iteration resets an existing
review because the newly published candidate must be reviewed independently.

## Verification and remaining boundary

The xtask suite includes negative coverage for earlier-RC reuse, invalid reviewer identity, unknown evidence fields,
and RC review invalidation. The current Alpha ledger explicitly records `review: null` and still passes the source
gate. The R11.3 checkbox remains open: a future signed final RC must be published and independently reviewed before
the recorder can truthfully write evidence. A local fixture or invented reviewer cannot close the gate.
