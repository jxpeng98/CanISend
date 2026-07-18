# Security Policy

CanISend is currently a pre-release Rust-native rebuild. Security fixes are applied to the latest published pre-release
and the active development branch; older development snapshots are not supported.

Please report a suspected vulnerability through GitHub's private **Security advisories → Report a vulnerability**
workflow for this repository. Do not include private application material, credentials, or a working exploit in a
public issue.

Include the affected version/commit, platform, command or data flow, expected impact, and the smallest non-sensitive
reproduction you can provide. The project will acknowledge the report, reproduce and classify it, coordinate a fix and
release, and publish a minimally necessary advisory after affected users can update.

The current trust boundaries, reviewed controls, dependency exceptions, and release blockers are maintained in the
[Rust-native threat model](docs/security/threat-model.md).
