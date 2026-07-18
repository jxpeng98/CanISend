# R11.4 Stable support-policy preparation

**Date:** 2026-07-18

**Roadmap item:** R11.4 preparation

**Status:** Machine-checked pre-Stable draft; publication pending Stable qualification

## Boundary

The Rust-native product needs a support promise that follows its actual release evidence rather than the historical
Python package. The new policy defines the `0.7` version window, five native target triples, Agent v2/schema v2
freeze, resource v2 verification, append-only workspace v2 migrations, generated Codex/Claude packs, runtime
independence, and explicit unsupported surfaces.

`release/support-policy.json` is compared with live Rust constants, the database schema inventory, release target
count, and workspace version by `xtask release check`. During a prerelease it must say `pre-stable-draft`; a Stable
workspace version automatically changes the expected value to `published`, forcing an explicit reviewed policy
update before a stable source gate can pass.

## Deliberately narrow promises

- Prereleases are supported only until superseded; Stable supports the latest patch in the current `0.7` minor.
- There is no LTS or service-level agreement.
- The five target triples are the qualification boundary; the policy does not claim every historical OS image.
- Python `0.6`, Linux arm64, scanned PDFs, GUI/browser automation, and automatic portal submission are unsupported.
- Human CLI text is not a machine API; JSON/schema/capability surfaces are.
- Workspace rollback means restoring a verified pre-upgrade backup to a new path, never deleting migrations.

## Remaining qualification

The R11.4 roadmap checkbox remains open. Stable must publish this policy with the exact signed Stable artifacts,
supported package-manager manifests, release notes, and a measured feedback snapshot. The published policy must then
be checked against the final Stable version, target set, protocol freeze, and workspace schema.
