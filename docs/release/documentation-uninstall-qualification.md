# Native documentation and uninstall qualification

[`release/documentation-uninstall-policy.json`](../../release/documentation-uninstall-policy.json) binds the
documented quick-start, host-agent, isolated install, and uninstall checks to the exact five archives built by one
RC release run. Alpha evidence may remain `prepared-native`, but only a signed RC run already recorded in
`release_candidates` can promote the ledger field to `passed`.

## Evidence production

`scripts/smoke_release_archive.sh` performs the checks against the extracted release binary rather than a Cargo
build path. In qualification mode it writes one body-free `canisend.documentation-uninstall/v1` record only after
all of these have succeeded:

- the archive executable equals the expected signed executable;
- the complete license, notice, install, privacy, security, and release-identity bundle exists;
- `version`, `doctor`, the documented quick-start, and the packaged host-agent flow pass;
- an isolated installed copy creates a workspace; and
- uninstall removes the installed unit while preserving that workspace.

The record contains only release/target identity, the archive digest, run ID, observed version, all-true check names,
and a UTC completion time. It contains no workspace, job, profile, application, host-pack, command-output, or runner
path data.

## Verification and ledger recording

The RC assembly job runs
`release verify-documentation-evidence TAG RELEASE_ASSET_DIRECTORY EVIDENCE_DIRECTORY`. Verification first checks
the complete signed release, then rejects missing/extra records, wrong target environments, mixed run IDs, archive
digests that differ from the manifest, false or unknown checks, version mismatch, and noncanonical fields.

After the public RC run and asset attestations have been independently inspected, download its complete release
assets and five-record evidence artifact. Preview the ledger change from a clean checkout:

```console
cargo run -p xtask --locked -- release record-documentation-qualification \
  v0.7.0-rc.1 DOWNLOADED_ASSET_DIRECTORY DOWNLOADED_EVIDENCE_DIRECTORY
```

The command requires the evidence run ID to equal the `signed_matrix_run` already recorded for that exact RC tag,
so all five records come from the same RC run.
It is dry-run-only unless `--write` is supplied, and it updates only `documentation_uninstall`. A successful local
fixture, Alpha/Beta run, other RC run, or invented run number cannot close the R11.3 checklist item.
