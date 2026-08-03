# Alpha.6 dual-Pack package-contract record

Date: 2026-08-03

Roadmap task: `M2-CONTRACT-001`

Status: source implemented; protected-main verification pending

## Decision

Alpha.6 advances the active package authority from
`canisend.alpha-package-contract/v2` to `canisend.alpha-package-contract/v3`. The v2 contract
remains historical truth for Alpha.5 and earlier. The v3 contract is an Alpha migration-checkpoint
authority only; it does not establish, refresh, or imply a Beta freeze baseline.

The v3 contract machine-binds the release package inventory to:

- Agent protocol `canisend.agent/v3`;
- Workspace format `canisend.workspace/v3`;
- workflow-Pack format `canisend.workflow-pack/v1`;
- both embedded Pack IDs, versions, and content digests;
- the exact embedded resource manifest;
- the exact operation registry, including 19 deprecated Academic-Pack compatibility aliases; and
- the complete contiguous migration inventory through database schema version 17.

## Bound source authorities

| Authority | Bound value |
|---|---|
| Academic Pack | `org.canisend.academic-job` `1.0.0`, digest `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07` |
| Generic Pack | `org.canisend.generic-application` `1.0.0`, digest `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321` |
| Resource manifest | 75 entries, SHA-256 `1db3b5b35eb6eb96da6c0caab31ff35e35cee932daf3a56efa2ebc9e8aa74476` |
| Operation registry | `canisend.operation-registry/v1`, SHA-256 `561752d6ab39498cb4103d5dc84457766381b8171d96bb851697a2558f2297a5` |
| Migration tree | 17 canonical SQL files, SHA-256 `d6ba9d1fb7ba1a92374b97534019d1ca247dc3fc4a2dac4f4cf2ae3181b81dae` |

`cargo run -p xtask --locked -- release alpha-package-bindings` renders the recomputed binding
object for review. `release check` compares that object with the checked-in v3 contract and fails
closed on any field, digest, count, path, protocol, Pack, registry, or migration drift.

## Regression evidence

Focused tests prove that:

- Alpha.5 selects package-contract v2 while Alpha.6 and Alpha.7 select v3;
- the current Alpha.6 contract matches every recomputed authority;
- mutating the Generic Pack digest, resource-manifest digest, operation-registry digest, migration
  tree digest, or schema causes the contract check to fail; and
- sequential Alpha transitions still accept the historical Alpha.5 v2 input while future
  Alpha.6-to-Alpha.7 iteration retains v3.

Local verification on the implementation branch:

```text
cargo test -p xtask --locked
87 passed; 0 failed

cargo clippy -p xtask --all-targets --locked -- -D warnings
passed

cargo run -p xtask --locked -- release check
passed, including Alpha package contract and release-status checks
```

This is source-contract evidence only. It does not replace `M2-SOURCE-001`, the exact native
candidate matrix, lifecycle qualification, Host Agent smoke, promotion, or public re-verification.
