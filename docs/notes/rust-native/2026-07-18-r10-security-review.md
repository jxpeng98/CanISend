# R10.1 Security Review Checkpoint

**Date:** 2026-07-18

**Branch:** `rewrite/rust-native`

**Commits:** `09ff041` (URL/path boundaries), `49f0ea2` (dependency policy), `751a6dd` (threat model)

**CI evidence:** GitHub Actions run `29629132522`

## Outcome

R10.1 closes the first complete security review of the Rust-native product. The review maps sixteen concrete threats
to controls and automated evidence, then audits the externally influenced boundaries: user and discovery URLs,
redirects, provider hosts, local and exported paths, embedded resources, backup archives, private logs, provider
payloads, and the restricted Typst renderer.

Provider discovery requests now retain the general DNS-pinned/per-hop SSRF policy and also require every requested
and redirected hostname to match the selected adapter's exact allowlist. Portable relative paths reject control
characters, overlong components, colon-bearing Windows stream syntax, device names, and trailing dots/spaces in
addition to traversal and internal-workspace paths.

## Dependency policy

CI runs `EmbarkStudios/cargo-deny-action@v2.0.18` with advisories, licenses, and sources enabled against release
features and targets. The policy permits only reviewed registry/Git sources and an explicit license set.

The audit identified quick-xml advisories RUSTSEC-2026-0194 and RUSTSEC-2026-0195 plus unmaintained bincode, paste,
yaml-rust, rustybuzz, and ttf-parser nodes in the pinned Typst graph. `deny.toml` records narrow exceptions because
the active renderer accepts only the fixed embedded template, escaped typed strings, verified embedded assets, and
embedded fonts. It exposes no bibliography/XML/YAML input, package/file resolver, user-authored Typst, or system-font
scan.

Before any of those surfaces is added, the affected dependencies must be upgraded or moved behind a separately
constrained process. This is an explicit release blocker, not a silent acceptance of broader input.

## Evidence

- 78 Rust tests passed at the reviewed commit.
- Workspace Clippy passed for all targets/features with warnings denied.
- Forty schemas and the complete resource manifest passed release drift checks.
- Local cargo-deny advisories, licenses, and sources checks passed.
- GitHub Actions run `29629132522` passed dependency policy, quality, macOS, Linux, and Windows jobs.

The authoritative security documentation is [the threat model](../../security/threat-model.md) and root
`SECURITY.md`. R10.2 recovery review follows this checkpoint.
