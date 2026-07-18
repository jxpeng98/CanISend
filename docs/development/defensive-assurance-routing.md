# Route Defensive Assurance Work Without Broad Cybersecurity Prompts

Codex, Claude, and other hosts may apply additional review when a request is phrased as a broad cybersecurity task.
That host-level classification is not a Rust setting and CanISend does not try to disable or bypass it. Routine work
should instead state the actual repository-owned engineering outcome.

## Use a narrow task contract

Every security-adjacent task should identify:

1. the CanISend component owned by this repository;
2. the defensive invariant being preserved;
3. a bounded local fixture or generated fuzz input;
4. the expected test, error category, or release evidence; and
5. an explicit exclusion of third-party targeting, credential acquisition, and protection bypass.

Examples:

```text
Add a bounded malformed-PDF regression test to canisend-io. Use generated local bytes only, require a typed error,
and do not contact or test any external system.
```

```text
Verify that the existing URL destination policy rejects loopback/private destinations at every redirect using the
repository's local HTTP fixture. Do not scan or probe public hosts.
```

```text
Run the pinned dependency advisory/license gate against Cargo.lock and report repository remediation only.
```

These descriptions are more accurate than “perform cybersecurity work” and give the host enough scope to distinguish
defensive product assurance from unrelated offensive activity.

## Separate execution tiers

Normal implementation uses focused crate tests and the fast Rust CI. Extended assurance is isolated:

- weekly/manual fuzzing for repository parsers;
- dependency and license policy checks;
- release-only signing, notarization, Authenticode, provenance, and archive verification;
- private vulnerability handling through [`SECURITY.md`](../../SECURITY.md).

This separation reduces unnecessary prompt coupling and developer wait time. It does not reduce the protection
provided by URL destination checks, path confinement, provider consent, private-data redaction, workspace integrity,
signing, or supply-chain evidence.

## When a host still pauses

Stop expanding the request. Split ordinary refactoring from the defensive test, provide the exact owned file and
fixture, remove ambiguous real-world targets or secrets, and state the non-offensive outcome. If the host still
declines, retain the product gate and complete the work through an authorized maintainer review; do not rephrase the
same request as a safety bypass.
