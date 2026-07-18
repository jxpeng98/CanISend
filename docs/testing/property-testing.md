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

Generation uses a checked-in fixed seed and 512 cases per primary property. This makes failures reproducible on
Linux, macOS, and Windows, adds no test-framework dependency, and keeps the fast CI path bounded. This suite
complements the scheduled [libFuzzer targets](scheduled-fuzzing.md); neither is evidence for the other.

## Commands and gates

Run the target directly with:

```bash
cargo test -p canisend-contracts --locked --test property_contract
```

Both ordinary CI and the native release source gate run that exact command as a named step. `xtask release check`
requires the test file, this policy, both workflow steps, and the four property functions, so the release-wide
quality checkbox cannot silently regress to unit tests alone.
