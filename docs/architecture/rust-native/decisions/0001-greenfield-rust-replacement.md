# ADR-RN-0001: Replace the Active Python Product with a Greenfield Rust Implementation

**Status:** Accepted

**Date:** 2026-07-17

## Context

CanISend must be installable and usable without Python. The Python implementation accumulated product knowledge but
also accumulated filesystem coordination, compatibility paths, package-resource rules, and a large Pytest suite.
Translating those mechanisms line by line would preserve constraints that the new product does not need.

## Decision

CanISend will be rebuilt as a Rust-native product in the existing repository.

- The final Python source is preserved only in Git history and the `archive/python-v0.6.0b1-final` tag.
- The active branch will remove the Python package, Pytest suite, Python scripts, Python build metadata, and Python CI.
- The Rust implementation uses new workspace and agent protocol versions.
- Old workspaces, CLI output, serialized bytes, and internal Python APIs are unsupported.
- Existing resources and product behaviors may be reintroduced only after review against the Rust-native roadmap.

## Consequences

- The implementation can simplify state, task, and schema boundaries instead of recreating compatibility code.
- A user who needs the historical product must explicitly use the archive tag.
- Rust release notes must clearly describe the breaking product generation.
- Tests specify the new Rust product and never execute Python as an oracle.

## Rejected alternatives

- Bundle the Python interpreter: rejected because it keeps Python packaging and runtime complexity.
- Maintain Python and Rust implementations in parallel: rejected because it doubles product and test maintenance.
- Translate files one by one while preserving wire compatibility: rejected because it preserves the legacy architecture.

## Revisit when

Only revisit if a separately funded legacy import tool is proposed. Such a tool must remain outside the normal Rust
runtime and cannot reintroduce Python as a product dependency.
