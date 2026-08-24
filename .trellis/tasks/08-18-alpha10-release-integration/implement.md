# Alpha.10 execution plan

## 1. Governance and task split

- [ ] Add `M3-HEADLESS-001` and `M3-ALPHA10-001` to the Master Roadmap before the invited cohort.
- [ ] Create one Alpha.10 milestone and one GitHub Issue per new Roadmap ID; leave historical
      Alpha.7–Alpha.9 Issues closed and immutable.
- [ ] Retain `M3-EVID-003` / Issue #68 as the affected-scenario rerun and keep real-user Issue #70
      blocked until an exact Alpha.10 checkpoint exists.
- [ ] Create and plan the two Trellis children below; implement the headless child first.

## 2. Child A — headless capability closure

- [ ] Add a CLI `project|global` value enum and `--scope` to host setup/status/remove, defaulting to
      project.
- [ ] Pass the selected scope through existing `AgentSkillsInstallRequest` calls and include it in
      structured/human receipts.
- [ ] Add the smallest CLI regression covering parse/default/global routing and missing-home
      failure at the application-facade owner.
- [ ] Extend the existing Agent v4 packaged/headless smoke rather than create a second harness:
      - clean Workspace initialization and starter checks;
      - project and isolated-global Skill install/status/remove/drift behavior;
      - exact MCP registration guidance;
      - mixed academic/generic Application lifecycle with App closed;
      - denial/replay/stale/wrong-context/host-restart no-mutation cases;
      - review/export, backup, restore, and reopened status.
- [ ] Update canonical Skills and quick-start guidance only where the executable workflow changed.
- [ ] Run focused CLI/App/resource tests, formatting, relevant Clippy, operation/semantic checks,
      and one final source gate on the PR head.
- [ ] Merge through protected Fast CI; do not bypass checks.

## 3. Child B — exact Alpha.10 qualification

- [ ] Reconcile the new protected capability source into the Roadmap and release notes.
- [ ] Preview `release prepare-stage v1.0.0-alpha.10`, inspect every controlled digest, then apply
      write mode from a clean branch.
- [ ] Run `cargo run -p xtask --locked -- release check` and Fast CI on the metadata PR; merge
      through branch protection.
- [ ] Dispatch one nonpublishing build-once candidate from the exact protected source.
- [ ] Verify the five CLI targets, supported Apple Silicon App packages, lifecycle, accessibility,
      integrity, SBOM, provenance, and signing evidence owned by the native workflows.
- [ ] Run the affected headless scenarios and exact-host synthetic dogfood against candidate bytes;
      record only body-free identities and outcomes.
- [ ] If all candidate gates pass, create the protected annotated tag and promote the same cached
      candidate without recompilation.
- [ ] Download every public asset independently and verify manifest, checksums, provenance,
      package contract, executable identity, starter resources, Skills digests, MCP inventory, and
      App-closed smoke.
- [ ] Merge the evidence record, mark the new Roadmap/Issue/milestone Verified, and rebind Issue #70
      to exact public Alpha.10. Do not claim invited-user or Beta evidence.

## Validation commands

Use the smallest owning checks during edits. The final headless PR head runs:

```text
cargo fmt --all -- --check
cargo clippy -p canisend-cli -p canisend-app -p canisend-resources --all-targets --all-features -- -D warnings
cargo test -p canisend-cli --locked
cargo test -p canisend-app --locked agent
cargo run -p xtask --locked -- operations check
cargo run -p xtask --locked -- semantics check
cargo run -p xtask --locked -- release check
git diff --check
```

Fast CI owns the complete workspace suite. Native release workflows own the exact packaged matrix;
do not duplicate that matrix locally.

## Review gates

- [ ] PRD, design, and this plan receive explicit user approval before the first child starts.
- [ ] No product code and version transition share one PR.
- [ ] No tag exists before candidate qualification.
- [ ] No public-release claim exists before independent downloaded-byte verification.
- [ ] No cohort/Beta claim is inferred from synthetic host dogfood.

## Rollback points

- Before Child A merge: close/revise the implementation PR; public Alpha.9 is unaffected.
- Before version write: leave Alpha.10 unprepared and keep Alpha.9 as cohort baseline.
- Before tag: discard a failed candidate and fix protected source; never promote it.
- After tag but before verified publication claim: stop reconciliation, diagnose exact public
  bytes, and do not rewrite the tag or historical evidence.
