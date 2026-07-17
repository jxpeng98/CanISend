# Security Policy

## Supported code

The active `rewrite/rust-native` line is pre-release software. The archived Python implementation is retained for
history and does not receive security fixes through the Rust rebuild branch.

## Reporting

Do not open a public issue containing private job adverts, profile evidence, drafts, API tokens, workspace databases,
or provider payloads. Report a vulnerability privately through the repository's GitHub security advisory feature.

## Current security boundaries

- CanISend prepares materials and does not submit applications.
- Secrets must not be stored in workspace configuration or committed files.
- Normal JSON responses and logs must not include private document bodies.
- Host agents must not edit `.canisend/` internal state.
- URL fetching, PDF parsing, provider transmission, SQLite state, and embedded rendering remain security-sensitive
  implementation areas and must satisfy their roadmap gates before release.
- No telemetry or crash upload is enabled by default.

The complete Rust-native threat model is a required R10 deliverable.
