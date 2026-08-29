# Codex-first Alpha.10 qualification design

## Boundary

This is a release-policy and evidence reconciliation over immutable public Alpha.10 bytes. It does
not change the product, rebuild a package, add a workflow, or publish Beta.1.

ADR-RN-0020 remains valid: Codex and Claude resources still come from one Agent v4 model and all
hosts use the same guarded MCP/application facade. No new ADR is needed because the protocol,
resource architecture, and available host adapters do not change. The Master Roadmap and support
guidance own which real-host evidence blocks the next release stage.

## Remaining identity chain

~~~text
verified public Alpha.10 bytes V
  -> existing Codex Generic evidence HG
  -> one missing Codex Academic evidence HA
  -> provider-dogfood/v2 record D
  -> protected policy/evidence reconciliation R
  -> separate Beta.1 readiness task
~~~

`V` already binds source `cd40180f2ff8ac957276f1948ba88da428511a82`, candidate run
`32678848156`, artifact `9503978913`, promotion run `33267148891`, 16 public assets, and
manifest SHA-256 `6669fb73e728d64bea10cb99d4f403ed7ffcb06e15401bfe04f93230a35e7bb5`.
Only `HA/D/R` remain. A product-byte change exits this task and requires a later sequential
prerelease; it is never folded into Alpha.10.

## Evidence contract

`canisend.provider-dogfood/v2` reuses the existing record structure and validators for:

- exact source, candidate artifact, archive, and executable identity;
- synthetic-metadata-only consent and zero retained private/secret material;
- Workspace/Agent/resource contracts;
- exact evidence-note digest;
- both built-in Pack identities and four Skill digests; and
- fail-closed scenario fields, revisions, preview state, mutation, and submission results.

The only policy-specific change is the canonical required scenario set:

1. `codex-cli-academic-requirement-preview-cancel`;
2. `codex-cli-generic-requirement-preview-cancel`.

Both must be `passed`, use the exact public Alpha.10 binary, preserve revision and proposed state,
and perform no mutation or submission. The v2 schema string itself identifies the Codex-first
policy; no extra policy field, helper, schema file, or workflow is added. Historical exclusions
remain in dated notes, so the active v2 `excluded_attempts` list is empty.

Claude Desktop's pass and Claude Code's `skipped-by-maintainer` outcome remain body-free
observations in the dated note and Trellis metadata. They do not enter the required passed set and
cannot be presented as equivalent evidence.

## Validation ownership

| Invariant | Owner used in this task |
|---|---|
| Provider schema, exact fields, both Pack scenarios, privacy rejection | Existing `xtask` validator and one existing test |
| Academic real-host behavior | One Codex CLI preview/cancel against public Alpha.10 |
| Generic real-host behavior | Existing exact Alpha.10 Codex evidence |
| MCP/App/CLI mixed-Pack lifecycle and failure cases | Existing Alpha.10 packaged Agent v4 smoke |
| Native targets, App package, SBOM, provenance, signing | Existing Alpha.10 candidate workflow |
| Same-byte promotion and public assets | Existing Alpha.10 promotion/public verification |
| Full source integration | Protected Fast CI |
| Extended dependency/platform assurance | Existing scheduled workflows |

No owner is duplicated locally. The remaining Rust change runs its focused validator test,
formatting, affected `xtask` Clippy, and one final source gate.

## Beta.1 handoff

After protected Alpha.10 reconciliation:

- Alpha.10 becomes the current Codex-qualified checkpoint.
- Real invited-user evidence moves after Beta.1 and before RC instead of blocking entry to Beta.
- Issue #71 becomes the next bounded task. It will design and implement lean Beta-readiness
  authority, build one exact Beta.1 candidate, promote the same bytes, independently verify the
  public release, and activate feature freeze.
- That task must preserve zero open P0 data/privacy/recovery/release blockers and cannot convert
  synthetic dogfood into user counts.

Keeping Beta readiness in a separate task preserves one independently verifiable outcome and
prevents an evidence-policy change from sharing a PR with a stage transition.

## Rollback and failure

- If the missing Academic Codex scenario fails without changing product bytes, retain the
  body-free failure and keep Alpha.10/Beta entry incomplete.
- If it exposes a product defect, fix protected source and qualify a later sequential prerelease;
  never move the Alpha.10 tag or replace its assets.
- Before the evidence PR merges, reverting the branch leaves Alpha.10 public with the existing gap
  and Alpha.9 as the machine provider baseline.
- After merge, a policy defect is corrected in a new protected PR. Historical Alpha.9 and
  Alpha.10 release evidence is never rewritten.
