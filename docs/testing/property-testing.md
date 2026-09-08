# Deterministic Property Testing

CanISend keeps a distinct generated property-test target at
`crates/canisend-contracts/tests/property_contract.rs`. It exercises public strong primitives over fixed generated
domains rather than relying only on hand-selected examples.

## Properties

The suite currently proves:

- generated portable relative paths survive construction and JSON round trips without normalization;
- inserting an empty, traversal, Windows-device, or trailing-dot/space component is always rejected;
- generated lowercase SHA-256 values round trip, while length and uppercase mutations are rejected; and
- generated UUIDv7 identities and positive revisions preserve their exact serialized identity.

Generation uses a checked-in fixed seed and 512 cases per primary property. This makes failures reproducible across
the release targets, adds no test-framework dependency, and keeps the macOS fast CI path bounded. This suite
complements the scheduled [libFuzzer targets](scheduled-fuzzing.md); neither is evidence for the other.

## Commands and gates

Run the target directly with:

```bash
cargo test -p canisend-contracts --locked --test property_contract
```

macOS Fast CI includes this target in `cargo test --workspace --exclude canisend-gui --locked`;
it does not run it again separately. The Linux native-release source gate retains the explicit
command above. The property policy check requires the test file, this policy, both workflow
commands, and all four property functions. Development and release both retain generated coverage.
