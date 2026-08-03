# M0 release-truth and iteration closure

**Date:** 2026-08-03
**Roadmap items:** M0-STATE-002, M0-STATE-003, M0-REL-002, M0-REL-003,
M0-WF-001, M0-FEEDBACK-001, M0-DOC-001
**Branch:** `agent/alpha6-m0-closure`
**State:** Local implementation and focused verification complete; protected integration pending

## Scope

This slice closes the remaining machine and documentation drift that would otherwise allow a
future Alpha, RC, or Stable transition to reuse stale authority:

- `release check` now derives the active public checkpoint from the authoritative 1.0 Roadmap and
  checks the README source/public status, Roadmap header, root release guide, bug-template version,
  37-operation parity count, GPL identity, and community-signing wording. A fixture proves stale
  active text fails while archived 0.7 text remains valid history.
- Feedback validation, refresh, and Stable publication follow the bounded Markdown path declared by
  `feedback-snapshot.next_roadmap.path`; the 0.7 and 1.0 paths are both accepted fixtures, while
  traversal, absolute, non-Markdown, and out-of-registry paths fail.
- Sequential Alpha continues to invalidate readiness, contract-freeze, and feedback state. The
  Alpha-to-Beta gate now rejects Alpha.4, Alpha.5, and Alpha.6 and accepts only exact public
  Alpha.7 evidence binding source, successful run, public URL, Agent/Workspace v3, Pack v1, and both
  embedded Pack digests.
- Sequential RC preparation requires the current RC to be the latest canonical recorded matrix.
  The two-commit post-freeze procedure records the resulting stage commit without an impossible
  self-referential exception hash.
- Release, native-upgrade, and package-manager workflow-dispatch defaults now point to the active
  1.0 candidate or Beta/RC pair and are checked from the workspace release line.
- Final RC feedback must match the latest recorded RC. Preparing a later RC resets the notes review
  and leaves older feedback invalid until refreshed.
- README, CHANGELOG, root release policy, qualification guidance, limitations, and desktop guidance
  now distinguish public Alpha.5 history from unqualified Alpha.6 source and describe the generic
  Pack framework, Workspace migration, GPL boundary, 37/37 parity, and community signing without
  broadening platform support.

No tagged evidence, archived 0.7 record, published Alpha.5 artifact, or package candidate was
rewritten.

## Local evidence

The following commands pass on the branch:

```text
cargo fmt --all -- --check
cargo test -p xtask --locked
bash -n scripts/refresh_release_feedback.sh scripts/refresh_beta_readiness.sh
cargo run -p xtask --locked -- release check
cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.6
git diff --check
```

The focused `xtask` suite contains 85 passing tests. The Alpha.6 transition remains a dry run: it
reports 27 controlled files, performs no write, creates no tag, and starts no workflow.

## Remaining evidence boundary

This note is not protected-integration evidence. The seven Issues remain In progress until the
branch is reviewed through a PR, all required checks pass, and the merge commit lands on protected
`main`. Only then may their labels and the M0 exit checklist move to Verified.
