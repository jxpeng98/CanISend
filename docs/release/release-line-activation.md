# Release-line activation

Release-line activation moves the repository's active development and qualification authority to a
new product line. It is intentionally separate from stage promotion because it archives the
previous line and resets all evidence that cannot truthfully carry over.

The checked-in policy is
[`release/release-line-policy.json`](../../release/release-line-policy.json). The only planned
activation is:

```text
cargo run -p xtask --locked -- release activate-line v1.0.0-alpha.1
```

The command is a dry run by default. Its JSON plan lists the digest before and after every
controlled file, the source commit, and the external actions it will not perform. Review that plan
before requesting a write.

## Write procedure

1. Commit all owned work and confirm a clean worktree.
2. Run the dry-run command above and review every controlled path.
3. Run the same command with `--write`.
4. Review `release/history/0.7/manifest.json`, the active release state, all exact internal
   dependency pins, and both lockfiles.
5. Run `cargo fmt --all`, focused `xtask` tests, Clippy, and
   `cargo run -p xtask --locked -- release check`.
6. Commit the activation as one isolated commit.

The write operation stages every output before replacement. If a replacement fails, the
transaction restores every path already changed and removes any newly created file. The history
manifest hashes copied 0.7 records and the retained `packaging/candidates/v0.7.0-alpha.1` tree.

This command changes repository source authority only. It does not create a tag, push a branch,
publish a release, upload an artifact, or modify an external package repository.

## Recovery

If the command fails before writing, correct the reported input or policy problem and rerun the dry
run. If the transaction reports a successful rollback, verify the clean worktree before retrying.
If rollback itself reports an error, stop and compare every path in the printed plan with Git
before making further release-state changes.
