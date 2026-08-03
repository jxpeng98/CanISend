# M2-SCOPE-001 — Alpha.6 scope freeze

**Date:** 2026-08-03

**Roadmap item:** M2-SCOPE-001 / GitHub Issue #45

## Decision

Alpha.6 is frozen as a dual-Pack framework and migration checkpoint. It includes the already
implemented `org.canisend.generic-application` Pack and canonical v3 surfaces as well as the
`org.canisend.academic-job` reference Pack, bounded v2 compatibility, and Workspace v2→v3
migration. It remains an Alpha testing checkpoint and cannot authorize Beta.

This corrects a scheduling assumption in the earlier Roadmap. The Generic Pack landed before the
M1 exit gate and is already embedded in the verified resource catalog, default Workspace
selection, CLI, MCP, desktop, documentation, examples, operation registry, and semantic-parity
matrix. An Alpha.6 package built from current source cannot omit it without a product rollback and
a new package/resource contract. Qualification must describe the bytes that actually ship.

## Included release surface

- `canisend.workflow-pack/v1`, Workspace v3, Agent v3, and the neutral Application model;
- exact embedded Academic and Generic Pack identities, versions, digests, resources, stage graphs,
  Deliverable catalogs, localization, renderers, and validators;
- Generic v3 create, plan, compose, review, approval, managed projection, render, consented local
  export, CLI, MCP, Agent-host handoff, and desktop paths;
- Academic reference-pack parity through the shared v3 facade plus bounded Workspace/Agent v2 and
  `job` compatibility;
- dry-run-first, verified-backup, failure-atomic Workspace v2→v3 migration that preserves Academic
  Pack authority;
- the four fictional Generic Pack examples and the executable dual-Workspace quick start;
- post-Alpha.5 integrity, recovery, approval, architecture, accessibility, release, and
  documentation fixes; and
- the first publicly conveyed `GPL-3.0-only` CanISend release, without changing Alpha.5 history.

## Explicitly excluded

- external Pack installation, publisher authentication, Pack marketplace discovery, or executable
  Pack hooks;
- automatic Generic URL, HTML, PDF, or local-file normalization beyond the reviewed bounded input
  documented by the current Pack;
- a claim that the Generic starter Pack models every grant, admission, tender, professional role,
  or regulated submission;
- portal login, upload, form filling, email, or submission;
- public Windows or Linux GUI support, Linux arm64 CLI, trusted publisher identity, Apple
  notarization, or production Authenticode claims not present in the exact release manifest;
- Beta readiness, contract freeze, or completion of the M3 target-user cohort; and
- any unrelated feature after this freeze.

After the protected-main merge containing this record, Alpha.6 changes are limited to its named M2
version, package-contract, GPL, source-gate, candidate, lifecycle, Agent-smoke, promotion,
publication, evidence, documentation, and confirmed release-blocker work. A new feature requires
an explicit Roadmap exception and resets every affected candidate qualification.

## Required qualification consequence

The Alpha.6 package contract and exact candidate evidence must bind both built-in Pack digests.
Source, lifecycle, CLI/MCP/desktop parity, Codex, and Claude qualification must exercise both Packs
at the bounded capability level described above. Alpha.7 remains responsible for feedback-driven
hardening, broader non-academic scenarios, target-user evidence, and the only Alpha baseline that
may proceed toward Beta.

The exact protected-main merge commit and successful merge-commit CI run are recorded on Issue
#45 after review. That external record is the freeze boundary; this note does not predict its own
commit identifier.
