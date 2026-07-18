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

1. choose the exact reviewed feature baseline commit;
2. set the ledger feature-freeze status to `frozen` and record that full 40-character commit;
3. set the exception record to the same status and baseline; and
4. run `cargo run -p xtask --locked -- release check` from a full Git checkout.

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
