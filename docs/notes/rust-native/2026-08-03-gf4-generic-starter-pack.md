# GF4-PACK-001 generic starter Pack

**Date:** 2026-08-03

**Roadmap items:** GF4-PACK-001; contributes to M1F-PACK/DAG/DELIV/I18N

## Outcome

CanISend now embeds and verifies `org.canisend.generic-application` alongside the academic
reference Pack. The generic starter has optional Pack metadata, neutral Requirement and Evidence
taxonomies, a nine-stage graph, two custom Deliverable kinds, English/Simplified Chinese
vocabulary, a domain-neutral embedded template, and the existing registered validator set.

## Implementation

- The resource catalog embeds the generic Manifest and its exact template body.
- `canisend-resources` assembles the Manifest and declared body as one bounded Pack input.
- `canisend-app` uses one loader for both built-in Packs and exposes a verified built-in registry.
- Registry resolution remains exact by ID, version, and content digest.
- The Pack reuses only registered local/URL/text-PDF intake, Typst renderer, and defensive
  validator capabilities.

## Defensive invariants

The owned component is CanISend's embedded Workflow Pack loader. The defensive invariant is that
the generic Pack receives no special trust: wrong identity, undeclared/missing/changed resources,
digest mismatch, incompatible runtime, unknown capability, invalid graph, or invalid Deliverable
catalog fails before the bundle can enter the registry. Loading and compiling the Pack performs no
Workspace mutation.

## Verification

Focused tests cover resource embedding, canonical digest agreement, runtime verification,
domain-neutral IDs and vocabulary, all-optional metadata, compiled graph/terminal stage, two custom
Deliverables and cardinality, Simplified Chinese locale compatibility, and exact dual-Pack registry
resolution. The complete source and release gates are run before this record is committed.

## Remaining boundary

GF4-FLOW-001 now binds the Pack-qualified graph and Deliverables to a canonical v3 local fixture.
Pack selection/configuration and Agent v3 surface work remain GF4-UI-001 and GF4-AGENT-001. The
first usable Alpha remains unqualified.
