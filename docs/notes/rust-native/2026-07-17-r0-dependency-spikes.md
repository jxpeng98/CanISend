# R0 Dependency Spike Evidence

**Date:** 2026-07-17

**Local platform:** `aarch64-apple-darwin`

**Rust:** `rustc 1.97.0 (2d8144b78 2026-07-07)`

## Selected dependency families

| Capability | Spike version | Decision |
|---|---:|---|
| SQLite | `rusqlite 0.40.1` with `bundled` | accepted for the initial storage implementation |
| Rust contract schema | `schemars 1.2.1` | accepted |
| Candidate validation | `jsonschema 0.48.0`, Draft 2020-12 | accepted with remote resolution disabled by default |
| PDF extraction | `pdf-extract 0.12.0` / `lopdf 0.42.0` | use `pdf-extract` as the initial high-level adapter |
| Typst compiler | Typst and `typst-pdf 0.15.1` | accepted behind the render port |
| Typst embedding helper | `typst-as-lib 0.16.0` | accepted for alpha behind a private adapter; API instability must not leak |
| HTTP/TLS | `reqwest 0.13.4` with Rustls | accepted pending native CI matrix evidence |

The lockfile resolved 466 packages for the combined native spike. This is a substantial dependency and compile-time
cost, so production features must remain narrow and renderer dependencies must stay isolated from fast core tests.

## Native probe evidence

Command:

```text
cargo run --manifest-path spikes/r0-dependencies/Cargo.toml \
  -p canisend-r0-native-probe --locked
```

Observed result:

```json
{
  "lopdf_recovered_expected_text": false,
  "ok": true,
  "pdf_bytes": 12261,
  "probe": "canisend-r0-native",
  "sqlite_version": "3.53.2"
}
```

The omitted text fields showed that both extractors recovered readable content. Direct `lopdf` inserted line breaks
and repeated spaces inside the expected sentence, so the strict phrase flag was false; `pdf-extract` returned the
complete expected phrase. This justifies beginning with the higher-level adapter while retaining the original PDF
bytes and adding a broader corpus during R4.

The probe also proved:

- Bundled SQLite creation, insert, and query.
- Generated schema meta-validation.
- Draft 2020-12 acceptance of a valid candidate and rejection of an invalid type.
- Embedded Typst fonts without a system-font scan.
- In-process Typst PDF generation without an external `typst` command.
- PDF text extraction from in-memory bytes.

The debug native probe was 186 MB. This is not a release-size measurement. LTO, stripping, resource selection, and a
release profile will be measured during R9/R10 rather than hiding the current cost.

`otool -L` reported only macOS system libraries and `libiconv`; it reported no Python runtime dependency.

## Network probe evidence

An explicit HTTPS request passed on the local macOS host:

```text
env CANISEND_R0_NETWORK_GET=1 cargo run \
  --manifest-path spikes/r0-dependencies/Cargo.toml \
  -p canisend-r0-network-probe
```

Result:

```text
reqwest-rustls-client-ok
```

The client disables automatic redirects so CanISend can apply its own redirect/address policy.

## Cross-platform evidence boundary

A macOS-hosted `cargo check --target x86_64-pc-windows-msvc` reached `aws-lc-sys` but failed because the macOS host
does not contain Windows SDK headers such as `windows.h`. That result is a cross-toolchain limitation and does not
prove either Windows success or a product failure.

Native Ubuntu, macOS, and Windows execution is therefore delegated to `.github/workflows/rust-r0-spikes.yml`. R0 is
not complete until that matrix passes. The workflow runs the locked native probe and an explicit Rustls HTTPS request
on every platform.

## Follow-up risks

- `typst-as-lib` documents its API as not fully stable. It must remain private to `canisend-io`.
- Embedded fonts and the complete Typst dependency graph materially affect binary size.
- `pdf-extract` remains provisional until the R4 academic-job PDF corpus passes.
- Reqwest 0.13 Rustls currently brings AWS-LC; native platform CI and supply-chain review are mandatory.
- The combined spike is not the production crate graph. R1/R2 will prevent renderer dependencies from entering core.
