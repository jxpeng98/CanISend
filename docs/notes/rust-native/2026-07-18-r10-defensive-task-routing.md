# R10 defensive assurance task routing

**Date:** 2026-07-18

**Roadmap item:** Developer workflow / R10 assurance follow-up

**Status:** Repository scope and task-routing guidance implemented

## Problem

Broad “cybersecurity” wording can cause Codex or another host to pause even when the actual work is ordinary Rust
release QA over repository-owned code. The classification belongs to the host and cannot be downgraded by a Rust
dependency or project configuration.

## Decision

The root `AGENTS.md` now establishes CanISend as a defensive, local-first product scope. It permits dependency,
artifact, parser, privacy, recovery, signing, and generated-input fuzz assurance while explicitly excluding
third-party targeting, credential acquisition, evasion, payload deployment, persistence, exfiltration, and
destructive testing.

The [routing guide](../../development/defensive-assurance-routing.md) replaces broad task labels with a five-part
contract: owned component, defensive invariant, bounded fixture, expected evidence, and explicit external-target
exclusion. Extended assurance remains separated from the fast Rust loop.

## Non-goal

This change does not bypass a Codex/Claude safety policy and does not weaken CanISend's URL destination, path,
consent, privacy, signing, provenance, or dependency controls. A host may still require extra review for a genuinely
sensitive task.

## Validation

`xtask release check` validates local links and requires the root scope to retain the defensive boundary, explicit
third-party exclusion, no-bypass rule, and four verification tiers. The focused xtask suite passes 17 tests, Clippy
passes with warnings denied, and the complete machine release check remains under one second once compiled.
