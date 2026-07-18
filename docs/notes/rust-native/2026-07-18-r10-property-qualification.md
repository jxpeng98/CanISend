# R10 generated property-test qualification

**Date:** 2026-07-18

**Roadmap item:** Definition of Done release evidence

**Status:** Qualified on the complete ordinary CI matrix

## Gap

The ordinary Rust suite already covered unit, integration, end-to-end, schema, resource, and release behavior, but
the final Definition of Done named property tests as a distinct class. No separately identifiable property target or
CI step existed, so the combined quality checkbox could not be closed from indirect coverage.

## Implementation

`crates/canisend-contracts/tests/property_contract.rs` generates a fixed, reproducible domain for four public
contract properties:

1. portable relative paths retain their exact value through construction and JSON round trips;
2. generated insertion of traversal, empty, reserved-device, or trailing-dot/space components is always rejected;
3. generated lowercase SHA-256 digests round trip while case and length mutations fail; and
4. generated UUIDv7 identifiers and positive revisions retain exact identity through JSON.

The target uses a fixed seed and bounded case count, so a failure is reproducible without adding a new test-framework
dependency. It is explicitly executed by ordinary CI and the native release source gate. The release checker
requires the named properties, test command, policy document, and both workflow entries.

## Local evidence

- `cargo test -p canisend-contracts --locked --test property_contract`: 4 passed.
- `cargo test -p xtask --locked`: 19 passed after adding the policy self-check.
- Clippy passes for `canisend-contracts` and `xtask` with warnings denied.
- `xtask release check` reports `property-test policy: ok (4 generated properties)`.

## Qualification boundary

GitHub Actions run `29640222233` passed all eight jobs at exact implementation commit
`1d52a839eeb089349995dec917633caad0458d90`. The `rust-quality` job's distinct `Run generated property contracts`
step passed, together with formatting, Clippy, the complete Rust suite, release checks, release build, and packaged
smoke. Dependency policy, three-OS recovery, and three-OS render/documentation jobs also passed.

The final combined quality checkbox is therefore qualified. The property suite complements scheduled libFuzzer
execution but does not qualify or replace it.
