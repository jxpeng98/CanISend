# Stage 3 Workspace Migration And Recovery

This guide applies when an existing CanISend workspace is first used with `0.3.0b1` or a later compatible Stage 3
build. The upgrade is additive and fail-closed: CanISend does not rewrite private application data merely because a
new package version is installed.

## Before Upgrading

1. Keep the private workspace git-ignored and make a backup according to your own retention policy.
2. Finish or explicitly cancel any active Draft/Review task before changing versions.
3. Record `canisend agent context --workspace <workspace> --job jobs/<job> --format json` and
   `canisend stage status ... --format json` so the durable starting state is inspectable.
4. Run `canisend doctor --workspace <workspace>`.

`canisend update-workspace` refreshes packaged prompts, schemas, skills, and bridge defaults. It is not a user-data migration.
Without `--overwrite`, locally edited defaults are preserved; user-owned job YAML and workflow receipts
are never migration targets.

## Compatibility Rules

- Existing Stage 1/2 jobs, compatibility Markdown, and Typst sources remain readable.
- Legacy 1.0 Draft/Review control records remain readable. A sole legacy Cover Letter instance may be associated with
  its current stable document ID without rewriting immutable records.
- Existing `review_dispositions.yaml` defaults to Cover Letter ownership. Research Statement and aggregate package
  decisions use separate files and mutation namespaces.
- Existing user-owned YAML is preserved byte for byte when its basis becomes stale. Status reports review-required;
  it does not silently normalize, copy, or delete old decisions.
- A legacy package without `package_review_findings.json` and current `package_review_dispositions.yaml` remains
  readable, but APP-Q5 fails closed. Cover Letter readiness alone is no longer whole-package readiness.
- Optional standalone Research Statement output never becomes required merely because the files exist. The confirmed
  Required Document Plan remains the source of requiredness.

## Advancing An Existing Job

Resume from durable state rather than recreating the job:

1. Make Parse, Confirm, Evidence, Match, Decision, Brief, and the Required Document Plan current.
2. Run `documents status` and use each stable document ID for guarded Draft and deterministic Review.
3. Use `review-dispositions status|init|update` for every supported prepared document. If its Draft/Review basis
   changed, apply the explicit `reset_for_current_review` patch before deciding new findings.
4. Run deterministic `stage run --stage package_review` without a document ID.
5. Use `package-review status|init|update`. If aggregate Review changed, apply the explicit
   `reset_for_current_package_review` patch before deciding new findings.
6. Run `check-package`. APP-Q5 independently rederives aggregate readiness from exact current receipts.

Blockers cannot be accepted or waived. A correction proposal must return through a new guarded candidate for its
target document. Package `reviewed` is not rendering approval, portal readiness, submission, or proof of receipt.

## Interrupted Work

- Re-run body-free status first. Stage state is reconstructed from immutable receipts when the derived state view is
  missing or malformed.
- A cache hit reuses only an exact current stage instance; changed inputs invalidate the affected instance and true
  descendants.
- If an accepted user-owned write reports receipt/recovery pending, run `user-mutation recover` with the returned
  mutation ID and explicit recovery consent. Recovery completes that accepted mutation; it does not replay the patch.
- Output drift, unsafe aliases, conflicting mutation controls, or a changed compare-and-swap baseline require manual
  reconciliation. Do not delete immutable events merely to make status appear clean.
- An edited Typst primary remains authoritative for editing. Regeneration writes a `*.generated.typ` candidate for
  explicit reconciliation instead of overwriting it.

## Rollback

Rolling back the executable does not roll back private data. Stop active work, restore the earlier isolated package
version, and retain the workspace backup plus all new receipts. Older compatible builds ignore unfamiliar additive
files, but they cannot certify Stage 3 aggregate readiness. Do not manually transplant decisions between document or
package namespaces, and do not reuse a mutation ID or published package version.
