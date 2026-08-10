# Design: exact Alpha.8 qualification

## Source sequence

1. Land Trellis control on `main`.
2. Merge the bounded PR #174 source.
3. Land release-authority regression fixes on the resulting source.
4. Apply the reviewed Alpha.8 controlled-file transition in its own commit/PR.
5. Freeze that exact merge commit for build-once native qualification and public promotion.

No candidate may be assembled from a PR head, mixed worktree, or rebuild.

## Release-tool contracts

### Sequential Alpha documentation

`prepare-stage` accepts exactly the known canonical development form or canonical
published-current form. It renders the target development version once. Zero, duplicate, or
unrecognized matches fail before mutation.

### Beta-eligible Alpha

An eligible source is an Alpha iteration of 7 or greater. Eligibility is necessary but not
sufficient: readiness must be qualified and bind the exact active tag, source, public run/URL, v4
contracts, both Pack digests, provider record, and measured user evidence. Alpha.6 and every
mismatch fail closed.

### Refresh script

The script reads the pending active Alpha tag, validates its eligible iteration, downloads the
matching public manifest, verifies the source exists in Git, and requires the provider record to
match. It does not select "latest" by time and never guesses a tag.

## Verification ownership

- Focused `xtask` tests own source-shape and eligibility behavior.
- `bash -n` plus the existing readiness verifier owns shell wiring.
- One final release source gate owns shared contracts and documentation.
- Native build-once workflows own candidate bytes; public reverify owns published bytes.

## Rollback

Before publication, revert the bounded change or close the candidate and retain Alpha.7. After
publication, Alpha.8 is immutable; a blocker requires a new sequential Alpha.
