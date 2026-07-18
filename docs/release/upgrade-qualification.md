# Native Beta-to-RC upgrade qualification

[`release/upgrade-qualification-policy.json`](../../release/upgrade-qualification-policy.json) defines five native
records for the exact signed Beta/RC archive pair: both supported macOS architectures, Linux GNU and musl, and
Windows MSVC. It complements package-manager lifecycle evidence; neither substitutes for the other.

## Lifecycle proved by every record

The runner consumes assets downloaded once and fully verified in a nonpublishing preflight. For its exact target it
then proves:

1. Beta and RC archive/manifest hashes match the verified pair;
2. Beta `version` and `doctor` identify a standalone native runtime;
3. Beta creates and checks an external synthetic workspace, then makes a verified backup;
4. the installed unit advances to RC, whose `version`, `doctor`, workspace open, and check pass;
5. the Beta binary either accepts the unchanged schema or rejects an advanced schema with stable
   `workspace.conflict`, without mutating the workspace;
6. Beta restores its pre-upgrade backup into a new path and checks the restored workspace;
7. RC regenerates a Codex host pack;
8. uninstall removes the installed binary/notices while the workspace, backup, and restored workspace remain; and
9. no release or package repository is changed.

Evidence contains only release identity, digests, schema numbers, tool behavior, booleans, the public Actions run ID,
and a UTC completion time. The workflow uses synthetic workspaces and must not upload their bodies, databases,
backups, paths, host packs, or command output.

## Execution and review

The manual `native-upgrade-qualification` workflow accepts one public signed Beta tag and one public signed RC tag.
Its preflight downloads both complete releases, runs the release verifier, and independently verifies GitHub
attestations with `gh attestation verify` before sharing the exact bytes with the five native jobs. A final job runs
`release verify-upgrade-evidence FROM_TAG TO_TAG EVIDENCE_DIRECTORY` and rejects missing/extra records,
mixed runs or manifests, false/unknown checks, wrong environments, wrong versions, unsafe old-binary behavior, and
noncanonical fields.

This workflow is read-only with respect to public release and package channels. A successful preparation bundle is
still external evidence that must be inspected before the qualification ledger may change from `pending` to
`passed`; checked-in policy or local fixtures cannot close R11.3. After independently inspecting the public run and
its attestations, use a clean checkout and preview the only permitted ledger mutation:

```console
cargo run -p xtask --locked -- release record-upgrade-qualification \
  v0.7.0-beta.1 v0.7.0-rc.1 DOWNLOADED_EVIDENCE_DIRECTORY
```

Only after the dry-run hashes and exact five-record evidence agree may the maintainer repeat the command with
`--write`, review the diff, and commit it. The command requires a qualified signed Beta, frozen baseline, and a
successful signed matrix for the exact RC tag; it cannot qualify documentation/uninstall or package-manager fields.
