# R0 Dependency Spikes

This workspace proves the dependencies that could block the Rust-native architecture before the active Python tree
is removed.

`native-probe` checks:

- Bundled SQLite opens, creates a table, writes, and reads.
- Rust types generate a valid JSON Schema.
- Draft 2020-12 candidate validation accepts and rejects the expected values.
- Typst compiles with embedded fonts and no system-font scan.
- `typst-pdf` returns a PDF in process.
- `pdf-extract` and direct `lopdf` extraction are compared against the same generated PDF. The probe requires the
  selected higher-level extractor to recover the expected text and reports the direct `lopdf` result separately.

`network-probe` checks that a Reqwest client compiles with Rustls and without native TLS. Set
`CANISEND_R0_NETWORK_GET=1` to perform an explicit request to `https://example.com/`; the default run is offline.

Run locally:

```text
cargo run --manifest-path spikes/r0-dependencies/Cargo.toml -p canisend-r0-native-probe
cargo run --manifest-path spikes/r0-dependencies/Cargo.toml -p canisend-r0-network-probe
```

Compile the network probe for a target without executing it:

```text
cargo check --manifest-path spikes/r0-dependencies/Cargo.toml \
  -p canisend-r0-network-probe --target <target-triple>
```

The spike is evidence for selecting a dependency family. Production code must wrap these libraries behind the crate
boundaries and policies defined by the Rust-native ADRs.
