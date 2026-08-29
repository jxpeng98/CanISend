# Lean Beta.1 readiness design

## Boundary

This task changes one release-control path:

```text
public Alpha.10 + provider-dogfood v2 + maintainer review + public Issue labels
  -> refresh_beta_readiness.sh
  -> release/beta-readiness.json v2
  -> xtask verify/readiness freshness/Beta transition authority
```

It does not change the CanISend product binary or perform the Beta stage transition.

## Reused authorities

- `release/provider-dogfood.json` remains the exact Codex, artifact, consent, Pack, and Skill
  evidence authority.
- `beta_readiness_contracts` remains the immutable Alpha package base. Derive the v2 readiness
  projection from it and add host resources and Skills there, so Alpha.10 package-contract bytes
  do not change.
- `exact_json_fields`, `validate_evidence_note`, and the provider validator remain the trust-boundary
  primitives.
- `release/beta-readiness.json` remains the only active readiness file.

## Readiness v2 shape

The qualified record keeps the current release identity, freshness, telemetry, blocker, and status
fields and replaces `user_evidence` with three bounded sections:

- `provider_evidence`: provider schema, checked-in file SHA-256, and the two canonical Codex
  scenario IDs;
- `maintainer_validation`: passed schema, body-free reviewer token, known-limitations result, and
  checked-in evidence-note path/SHA-256;
- `cohort_evidence`: canonical zero counts plus `v1.0.0-beta.1` start and `v1.0.0-rc.1` completion
  boundary.

The `contracts` object directly includes resource format, task-resource-model digest, both Pack
digests, and all four Skill digests. Exact object-field validation makes private or speculative
fields invalid.

## Public Issue projection

The refresh script retains only Issue number, state, and label names. An applicable mechanical
blocker is an open Issue with both `priority:P0` and `state:blocked`. Planned future P0 work is not
an Alpha.10 defect and therefore does not block readiness.

The snapshot records total/open counts and an empty `open_p0_blocker_issue_numbers` array. The
validator also requires nine reviewed blocker classes and non-Issue maintainer/provider evidence.
This ensures that an empty GitHub result cannot qualify Beta by itself.

## Maintainer validation input

The existing refresh script accepts one JSON file with exactly:

```json
{
  "schema": "canisend.beta-maintainer-validation/v1",
  "status": "passed",
  "reviewer": "BODY_FREE_TOKEN",
  "known_limitations_reviewed": true,
  "evidence_note": {
    "path": "docs/notes/REVIEWED-NOTE.md",
    "sha256": "SHA256"
  }
}
```

The input is embedded into readiness after validation. It contains no issue body, prompt,
conversation, application body, credential, path outside `docs/notes/`, or provider payload.

## Compatibility and history

`canisend.beta-readiness/v2` is an intentional internal evidence-schema break. No qualified v1
readiness record exists for the active Alpha.10 state, and historical release records stay in Git.
Old v1 cohort input cannot authorize Beta.1. Public CLI/App/Workspace/Agent contracts do not change.

## Rollback

Before protected merge, drop the readiness branch. After merge but before Beta stage, restore the
active readiness file to canonical Alpha.10 pending state in a new protected PR. Never rewrite
Alpha.10 source, tag, artifacts, provider evidence, or historical records.
