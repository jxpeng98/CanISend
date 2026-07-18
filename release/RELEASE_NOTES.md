# CanISend 0.7.0-alpha.1

CanISend 0.7 is a greenfield Rust-native release. It installs as a platform-specific executable and does not require
Python, Node.js, Java, a separately installed SQLite library, or a Typst command.

The alpha provides local-first job intake, discovery imports, evidence and criteria workflows, matching, application
planning, structured drafting and review, readiness checks, editable exports, and embedded PDF rendering. Codex,
Claude, and custom hosts integrate through the versioned `canisend.agent/v2` JSON protocol and exported agent packs.

This prerelease does not migrate Python-era workspaces. Read `KNOWN_LIMITATIONS.md` before using it with real data.
Back up important Rust-native workspaces before upgrading between prereleases.
