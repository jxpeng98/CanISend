# RC Feature-freeze Enforcement

The qualification ledger declares whether the Rust-native release line is still `planned` or has entered `frozen`.
[`release/feature-freeze-exceptions.json`](../../release/feature-freeze-exceptions.json) is the matching machine record
for post-baseline changes.

## Before the freeze

While the release remains Alpha or Beta and the ledger status is `planned`, the exception record must also be
`planned`, its baseline must be `null`, and its exception list must be empty. It cannot pre-authorize future source
changes.

## Activating the freeze

After a signed Beta is qualified:

1. finish and commit all reviewed Beta qualification evidence so the clean `HEAD` is the intended baseline;
2. preview the two-file, digest-bound activation plan;
3. apply it explicitly from the same clean `HEAD`; and
4. commit the activation files and run the source gate from a full Git checkout.

```console
git rev-parse HEAD
cargo run -p xtask --locked -- release activate-feature-freeze FULL_HEAD_COMMIT
cargo run -p xtask --locked -- release activate-feature-freeze FULL_HEAD_COMMIT --write
cargo run -p xtask --locked -- release check
```

The command accepts only a full lowercase commit equal to current `HEAD`, a Beta-stage ledger with qualified signed
Beta evidence, and canonical planned ledger/exception state. Dry-run is the default; `--write` additionally requires
a clean worktree. It updates only `release/qualification-ledger.json` and
`release/feature-freeze-exceptions.json`. It does not create a tag, publish a release, or authorize Stable.

RC and Stable workspace versions fail unless the freeze is active.

## Allowed post-baseline changes

Documentation and narrowly defined release-evidence files are accepted automatically. These include `docs/`, root
project/support Markdown files, the qualification ledger, feedback/support state, final release notes, checked-in
package candidates, committed release evidence, and the exception record itself.

Every other changed path must be listed under the exact commit that changed it. An entry contains:

- the full lowercase Git commit;
- class `release-blocker` or `release-evidence`;
- a bounded, nonempty reason; and
- the exact sorted set of non-automatic paths changed by that commit.

The verifier requires the baseline to exist and be an ancestor of `HEAD`, reconstructs every commit and changed path
in `BASELINE..HEAD`, and rejects missing, extra, reordered, duplicated, stale, or invented exception entries. A code
change therefore cannot be relabeled by adding a broad path pattern or an unrelated commit.

The mechanism records review scope; it does not itself decide whether a blocker fix is correct. Each exception still
requires normal tests, review, and the clean-tag RC matrices.
