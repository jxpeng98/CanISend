# Alpha.10 dependency-assurance design

## Boundary

This task renews an existing, machine-enforced risk decision; it does not change product code,
dependency versions, accepted advisory IDs, reachability claims, or defensive controls.

Four existing authorities are kept aligned:

1. `release/dependency-advisory-exceptions.json` owns the lock-bound exception dates and facts.
2. `docs/release/dependency-assurance.md` and one dated implementation note expose the review
   evidence and unchanged product boundaries.
3. The Master Roadmap and one GitHub Issue project this P0 blocker publicly under Alpha.10.
4. The Trellis task records bounded execution and returns control to the already in-progress
   headless child after the gate passes.

No new policy format, validator, dependency, workflow, or abstraction is required.

## Review contract

For each of the existing 23 entries:

- the advisory ID and `deny.toml` reachability text remain byte-for-byte aligned;
- the third-party lock fingerprint remains
  `d2807a35172dc853ad98f7e128f1cbc4737b61aac8cb31f4ddf56c18b05ed903` with 751 packages;
- `reviewed_on` becomes `2026-08-24`;
- `review_by` and `expires_on` become `2026-09-07`;
- ownership, removal condition, and upstream tracking remain unchanged.

Any different advisory set, lock fingerprint, reachability, or input/platform exposure stops the
renewal and requires a new decision.

## Evidence flow

```text
unchanged Cargo.lock + guarded source boundaries
  -> exact cargo-deny 0.19.5 scan
  -> maintainer 14-day acceptance
  -> policy/date and public-record update
  -> dependency validator
  -> complete release source gate
  -> resume M3-HEADLESS-001
```

The policy validator continues to enforce ordering, exact field sets, matching `deny.toml` text,
date maximums, lock identity, and the declaration-only bibliography helper. The existing release
gate remains the integration owner; no duplicate test is added.

## Compatibility and security

- There is no runtime, CLI, schema, Workspace, or artifact compatibility change.
- User-authored Typst, bibliography/CSL/XML, user/system fonts, user-authored Tauri patterns, and a
  public Linux GUI remain release blockers.
- The two `quick-xml 0.38.4` vulnerabilities remain accepted only because their parser entry points
  are unreachable from CanISend input.
- A missed 2026-09-07 review fails closed without a grace interval.

## Rollout and rollback

The date change and evidence record land as the smallest prerequisite to the headless capability
source gate. If either dependency check fails, revert the renewal files and leave Alpha.10 blocked;
no tag, package, database, or public artifact is affected.
