# Rust-native completion and external-gate audit

**Date:** 2026-07-18

**Roadmap scope:** Remaining R11 and Definition of Done items

**Status:** Repository implementation prepared; external signed-release sequence blocked on identity provisioning

## Repository state audited

The `rewrite/rust-native` branch is pushed at Stable channel implementation commit `f46f45c`; all eight jobs passed
in exact ordinary CI run `29644993877`. R0 through R11.1 are complete, every current Rust source gate
passes, and the repository contains guarded recorders and verification paths for Beta signing, feature freeze, two
clean RC matrices, five-target upgrade/documentation/uninstall evidence, four-record native package qualification,
final RC release-note review, measured feedback publication, Stable support policy, and Stable channel assets.

The current Alpha-to-Beta transition remains read-only. Its fresh preview contains exactly ten controlled files,
reports `writes_performed: false`, and preserves the public Alpha readiness/freeze/feedback/candidate history. No
workspace version or qualification state was changed during this audit.

## Live GitHub state

The public repository currently reports:

- default branch `main` and no open public issue;
- public native release `v0.7.0-alpha.1`, but no `0.7` Beta, RC, or Stable release;
- zero configured Actions secret names and zero Actions variable names;
- only `ci`, `release`, `rust-r0-spikes`, and the dependency graph registered from the default branch;
- no pull request for `rewrite/rust-native`, so `scheduled-fuzz` is not yet registered on the default branch.

The name-only signing audit therefore reports all 14 required names missing: three Apple secret names, four Apple
variable names, and seven Azure/Windows variable names. It did not request or expose any secret value.

## Remaining checklist dependency chain

The 11 open roadmap/Definition of Done checkboxes are not 11 independent implementation tasks. They form these
evidence gates:

1. **Identity provisioning:** Apple Developer ID/notary credentials plus Azure Artifact Signing Public Trust and
   least-privilege GitHub OIDC federation.
2. **Signed Beta:** refresh readiness, write the reviewed Beta transition, run the nonpublishing signed matrix,
   inspect evidence, publish the clean tag, and record the downloaded public assets.
3. **RC qualification:** activate the feature freeze; publish and independently qualify RC.1 and RC.2; run the
   five-target upgrade, documentation/uninstall, and four-record package-manager matrices; review final notes.
4. **Stable publication:** refresh and review public feedback; apply the qualified Stable transition; publish signed
   archives, support policy, release notes, next roadmap, and the six Stable package-channel assets.
5. **Default-branch fuzz:** after reviewed cutover to `main`, dispatch the registered three-target scheduled workflow
   and resolve any reproducible crash before closing the final fuzz checkbox.

The source gates intentionally prevent these items from being checked early. Invented run IDs, locally assembled
evidence, unsigned replacements, or a downgraded signing policy cannot substitute for public external evidence.

## Required maintainer input

The next executable step is not another code change. A maintainer must provision the identities described in
`docs/release/signing-operations.md` and configure the 14 repository setting names. After that external state change,
rerun the name-only audit, refresh Beta readiness, and execute the documented dry-run-first transition and signed
matrix. Until then the correct product state remains `0.7.0-alpha.1` and every dependent checkbox remains open.
