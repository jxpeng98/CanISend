# M3 Agent v4 dual-Pack lifecycle preflight

**Date:** 2026-08-09

**Roadmap items:** `M3-CLI-001`, `M3-PARITY-001`, and `M3-DESKTOP-003`

**Product source base:** merge `d5eea826c6e0de8ad4d7537cc0b9c830adeb34f9`

**Data:** synthetic, body-free, local temporary directories only

## Scope

The packaged Agent v4 MCP smoke now keeps one stdio server alive while it runs the guarded
Requirement → Plan → Deliverable lifecycle for generic and academic Applications in the same
clean Workspace v4. It derives every commit token and digest from the preceding response instead
of substituting a source-level or in-process service call.

For both built-in Packs, the preflight covers:

- exact Source-bound Requirement extraction and confirmation;
- Pack-qualified Plan proposal and confirmation;
- required Deliverable drafting and consented private-body audit;
- single-use approval, explicit denial, replay refusal, stale-context refusal, wrong-Application
  refusal, invalid cross-Pack Deliverable kind refusal, and no private-body disclosure on denied
  reads; and
- final Application revision, Pack identity, Requirement state, Plan state, Deliverable count,
  and Workspace integrity reconciliation.

## Local exact-binary results

The same smoke passed with the App closed against:

1. the current `release-alpha` standalone `canisend` binary;
2. the current `release-alpha` unified `canisend-gui` binary in CLI/MCP mode; and
3. that unified host after staging it inside an ad-hoc-signed macOS App and verifying the bundle
   and companion integrity manifest.

The exact locally extracted standalone archive and macOS App ZIP also passed the rewritten
clean-v4 documented quick start, Agent v4 host setup/status/remove plus pre-v4 refusal, the full
guarded dual-Pack MCP lifecycle, Workspace retention, bundle/signature integrity, and packaged GUI
launch. This caught and removed two active release-harness dependencies on the retired
`agent capabilities` and `smoke_host_agent.sh` surfaces before they could reach an Alpha.7
candidate.

The staged local host identities were:

- unified host SHA-256:
  `4a0f44844eb9de952c3edc357a5da57fc1deba0456d8c0650a6127f860815070`;
- companion integrity manifest SHA-256:
  `f5ee418e344c4d8ddc1b774a6e49d35e676812486cf8766ccc97fa8809d03fc0`.
- standalone CLI archive SHA-256:
  `a27a4369ec8bea1b04cf7ddbee4c9648db54d5eb777149bb71cd17fabc2af140`;
- macOS App ZIP SHA-256:
  `004aa2c90930bd420a70e813d4db9d6ef199289657c2e7010312ec0cf6918d3e`;
- macOS installer DMG SHA-256:
  `5a5b050e56bf40d2140e59c26b901906fab0b8aa739ab34b7a4b39f72adb3ac0`.

These are non-publishable local preflight bytes. They are ad-hoc signed, retain the post-Alpha.6
development version, and do not qualify or authorize Alpha.7. The protected Linux, Windows, and
macOS development matrix must pass the new smoke before this implementation is merged. Later,
the five-target clean-tag release matrix and public-byte reverification remain mandatory for
`M3-CLI-001` and `M3-ALPHA7-001`.
