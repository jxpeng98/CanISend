# CanISend Engineering Guidelines

> High-signal rules derived from the repository's accepted ADRs, code, and contribution policy.

---

## Overview

These files guide Trellis work across the Rust workspace and its adapters. `AGENTS.md`, accepted
ADRs, public contracts, and machine policy remain authoritative when a summary here disagrees.

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Crate ownership and adapter boundaries | Current |
| [Database Guidelines](./database-guidelines.md) | SQLite, immutable Blob, migration, and transaction rules | Current |
| [Error Handling](./error-handling.md) | Typed errors and stable cross-surface classification | Current |
| [Quality Guidelines](./quality-guidelines.md) | Minimum-sufficient verification and review rules | Current |
| [Logging Guidelines](./logging-guidelines.md) | Body-free output and no-telemetry boundary | Current |

---

**Language**: All documentation should be written in **English**.
