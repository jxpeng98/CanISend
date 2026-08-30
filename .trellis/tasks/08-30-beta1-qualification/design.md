# Beta.1 qualification design

## Boundary

This task performs one fail-closed release-ledger transition over immutable public bytes. It keeps
the existing `xtask release record-beta-qualification` command as the only writer.

## Owning mismatch

~~~text
initial_alpha_qualification_ledger
  -> beta = { status: pending }
prepare-stage Alpha -> Beta
  -> preserves beta object
checked-in Beta ledger
  -> beta = { status: pending }
beta_qualified_ledger (current)
  -> expects status plus four null/empty placeholder fields
  -> rejects supported transition output
~~~

The minimum repair changes the recorder's pending equality check and its existing regression to the
status-only object already owned by the generator. The qualified output remains unchanged and
continues to add exact run, tag, source, and signing targets.

## Identity and mutation chain

~~~text
public release R
  -> fresh download D
  -> release/checksum/signing verification V
  -> 20 exact provenance checks A
  -> dry-run report P
  -> clean-worktree write W
  -> one ledger diff L
  -> protected source gate and PR M
~~~

`R/D/V/A/P/W/L/M` must agree on tag `v1.0.0-beta.1`, source
`6e1397b79031cad54e794ccdc9edca2153f23b3e`, candidate run `33281162734`, and manifest SHA-256
`2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`.

## Ledger result

Only `beta` changes from the canonical pending object to:

~~~json
{
  "signed_matrix_run": 33281162734,
  "signing_evidence_targets": [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc"
  ],
  "source_commit": "6e1397b79031cad54e794ccdc9edca2153f23b3e",
  "status": "qualified",
  "tag": "v1.0.0-beta.1"
}
~~~

Top-level stage/status remain Beta / `beta-qualifying`; freeze remains planned and baseline-free.

## Failure and rollback

- Public identity or provenance mismatch: stop before preview and do not edit the repository.
- Preview mismatch: stop before write; retain no report in the repository.
- Dirty worktree or changed public bytes: write fails closed.
- Unexpected write path or report mismatch: restore the single uncommitted ledger file and fix the
  owning code or evidence; never edit the generated qualified fields manually.
- Protected CI failure: fix the owning regression or projection and rerun; never alter the tag or
  public release.

## Compatibility

None. The canonical pending shape is current generated state, not a legacy input. No compatibility
branch is retained.
