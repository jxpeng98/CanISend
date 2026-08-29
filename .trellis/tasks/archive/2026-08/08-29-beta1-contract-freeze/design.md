# Beta.1 contract-freeze design

## Boundary

```text
qualified readiness v2 + Alpha.10 package contract + repository-owned v4 authorities
  -> build_beta_contract_freeze
  -> canisend.beta-contract-freeze/v2
  -> one shared validator
  -> release check and later Beta.1 stage authority
```

This task changes release-control metadata only. It does not change product bytes or apply the
Alpha-to-Beta transition.

## Root cause

The existing builder still imports the legacy `AGENT_PROTOCOL`, `PUBLIC_SCHEMA_VERSION`, and
`WORKSPACE_FORMAT` aliases and retains the historical migration ceiling of 13. The stage boundary
then checks only the freeze baseline. Updating the JSON alone would therefore self-report v2/v3
authority or allow an incomplete record with the right Alpha source.

The smallest complete fix is to build one v2 value from authorities already validated elsewhere
and make both callers compare against that exact value.

## Reused authorities

- `validate_qualified_beta_readiness` owns exact Alpha.10, provider, Pack, Skill, and maintainer
  eligibility.
- `alpha_package_contract_bindings` owns v4 protocols, both Packs, the resource manifest,
  operation registry, and migration inventory.
- `beta_readiness_v2_contracts` adds host resources, task-resource model, and Skill digests.
- The checked-in schema directories own their canonical bytes.
- `ErrorCode::ALL` and `ExitClass` own stable CLI exit classification.
- The validated `release/alpha-package-contract.json` owns standalone CLI and macOS layouts.

No parallel schema registry or duplicated release helper is introduced.

## Freeze v2 shape

The record contains five bounded sections:

- `baseline`: exact Alpha.10 tag and protected source commit;
- `contracts`: the shared Alpha package bindings extended only with readiness-owned resource/task/
  Skill fields;
- `schemas`: counts for the four active schema families plus one deterministic tree digest;
- `exit_codes`: all stable exit classes and the ordered error-code mapping derived from
  `canisend-contracts`;
- `alpha_package_contract`: checked-in path, v3 schema, SHA-256, and the bound layout section names.

Exact value equality is the validator. Unknown fields and every changed digest or mapping fail
without a second handwritten field matrix.

## Data flow and validation

1. Parse the active workspace version and qualified readiness file.
2. Run the complete readiness validator before reading its Alpha baseline.
3. Validate the active Alpha package contract against current v4 bindings.
4. Merge only the readiness-only resource/task/Skill fields into those shared bindings and verify
   their overlapping Agent/Workspace/Pack fields agree.
5. Digest canonical schema bytes, derive exit mappings, and digest the validated package contract.
6. Render the candidate. `release check` and `prepare-stage` compare the checked-in record to this
   same generated value.

## Compatibility and history

`canisend.beta-contract-freeze/v2` is an intentional internal evidence-schema break. No qualified
active v1 record exists for Alpha.10. Historical v1 evidence remains unchanged, while public App,
CLI, MCP, Workspace, Agent, and Pack contracts do not change.

## Rollback

Before protected merge, drop the branch. After merge but before staging, restore the active file to
canonical Alpha.10 pending state through a protected PR. Never rewrite Alpha.10, readiness evidence,
the public tag, or historical freeze records.
