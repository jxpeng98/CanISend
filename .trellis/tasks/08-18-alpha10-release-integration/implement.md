# Alpha.10 execution plan

## 1. Governance and task split

- [x] Add `M3-HEADLESS-001` and `M3-ALPHA10-001` to the Master Roadmap before the invited cohort.
- [x] Create one Alpha.10 milestone and one GitHub Issue per new Roadmap ID; leave historical
      Alpha.7–Alpha.9 Issues closed and immutable.
- [ ] Close Issue #68 as not applicable after protected Codex-first reconciliation; rebind
      real-user Issue #70 to public Beta.1 validation before RC.
- [x] Create and plan the two Trellis children below; implement the headless child first.

## 2. Child A — headless capability closure

- [x] Add a CLI `project|global` value enum and `--scope` to host setup/status/remove, defaulting to
      project.
- [x] Pass the selected scope through existing `AgentSkillsInstallRequest` calls and include it in
      structured/human receipts.
- [x] Add the smallest CLI regression covering parse/default/global routing and missing-home
      failure at the application-facade owner.
- [x] Extend the existing Agent v4 packaged/headless smoke rather than create a second harness:
      - clean Workspace initialization and starter checks;
      - project and isolated-global Skill install/status/remove/drift behavior;
      - exact MCP registration guidance;
      - mixed academic/generic Application lifecycle with App closed;
      - denial/replay/stale/wrong-context/host-restart no-mutation cases;
      - review/export, backup, restore, and reopened status.
- [x] Update canonical Skills and quick-start guidance only where the executable workflow changed.
- [x] Run focused CLI/App/resource tests, formatting, relevant Clippy, operation/semantic checks,
      and one final source gate on the PR head.
- [x] Merge through protected Fast CI; do not bypass checks.

## 3. Child B — exact Alpha.10 qualification

- [x] Reconcile the new protected capability source into the Roadmap and release notes.
- [x] Preview `release prepare-stage v1.0.0-alpha.10`, inspect every controlled digest, then apply
      write mode from a clean branch.
- [x] Run `cargo run -p xtask --locked -- release check` and Fast CI on the metadata PR; merge
      through branch protection.
- [x] Dispatch one nonpublishing build-once candidate from the exact protected source.
- [x] Verify the five CLI targets, supported Apple Silicon App packages, lifecycle, accessibility,
      integrity, SBOM, provenance, and signing evidence owned by the native workflows.
- [x] Run required Academic and Generic Codex synthetic dogfood against exact Alpha.10 bytes;
      retain Claude and bounded MCP results only as truthful non-blocking observations.
- [x] If all candidate gates pass, create the protected annotated tag and promote the same cached
      candidate without recompilation.
- [x] Download every public asset independently and verify manifest, checksums, provenance,
      package contract, executable identity, starter resources, Skills digests, MCP inventory, and
      App-closed smoke.
- [ ] Merge the evidence record, mark the new Roadmap/Issue/milestone Verified, and rebind Issue #70
      to public Beta.1 validation. Do not claim invited-user or Beta evidence.

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

- [x] PRD, design, and this plan receive explicit user approval before the first child starts.
- [x] No product code and version transition share one PR.
- [x] No tag exists before candidate qualification.
- [x] No public-release claim exists before independent downloaded-byte verification.
- [x] No cohort/Beta claim is inferred from synthetic host dogfood.

## Rollback points

- Before the evidence merge: revert the bounded reconciliation branch; public Alpha.10 and
  historical evidence are unaffected.
- If reconciliation finds a product defect: qualify a later sequential prerelease from new
  protected source; never move the Alpha.10 tag or replace its assets.
- After merge: correct any policy defect in a new protected PR and never rewrite historical
  evidence.
