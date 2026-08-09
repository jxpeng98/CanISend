# M3 packaged desktop dual-Pack journey

**Date:** 2026-08-09

**Roadmap items:** `M3-BOOTSTRAP-001`, `M3-DESKTOP-002`, `M3-DESKTOP-003`,
`M3-EVID-001`

**Base source:** merge `e34c6cfd8ddf2a9ba3e617bdb911a995768707ce` plus the four-file
desktop/shared-validation fix recorded below

**Data:** synthetic, body-free, local temporary directories only

## Exact local package

The `release-alpha` macOS App was built as version `1.0.0-alpha.7`, staged with bundle identifier
`io.github.jxpeng98.canisend.design-preview`, ad-hoc signed, and verified for one unified host,
final-byte integrity, bundle size, version, and signatures before interaction.

- unified host SHA-256:
  `bc95b7a8fb4787f94a29c0dec40c3d3c6154edb72c43668b3f9956cc1b6e3634`;
- companion integrity manifest SHA-256:
  `7d22c336c155da2a20e5f49321f90bcd31ac785f8c830278264dc739c791357f`;
- preview receipt SHA-256:
  `896961c199414213590b145d862560d57ea45535fd593e1fb46ea8fcc2107925`;
- academic Pack `1.0.0` digest:
  `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07`;
- generic Pack `1.0.0` digest:
  `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321`;
- Codex Agent v4 resource manifest SHA-256:
  `afeebed6dc269d05703986fbc7237ee23a204de97187f79e5b6ce0d78ce08414`;
- Claude Agent v4 resource manifest SHA-256:
  `72705440f255c2ccea7e7564a0458700fb32b085ed5a17297fbd0819adf18bbd`.

The receipt fixes `publication_allowed` to `false`; these identities retain local interaction
evidence and do not qualify public Alpha.7 bytes.

## Packaged interaction outcomes

Using only the packaged App UI for the user journey:

1. a new empty directory became one neutral `canisend.workspace/v4` Workspace at schema 20;
2. setup installed the four first-party Skills and project-local MCP configuration for both Codex
   and Claude without creating a Profile, private body, Application, or Workspace mode;
3. generic and academic pasted-text intake each previewed exact Source spans and one proposed
   Requirement without mutation, then committed the reviewed Source and explicit Application link;
4. one reusable private-local Profile Source was initialized and explicitly associated, with
   consent, to one Application from each Pack in the same Workspace;
5. both Applications completed Requirement confirmation, Plan proposal and confirmation,
   Deliverable drafting, consent-gated private review, and explicit approval;
6. the academic Application exported two one-page validated PDFs and the generic Application
   exported one one-page validated PDF; both manifests record `submission_performed: false`;
7. closing and reopening the App retained both Pack-bound Applications at revision 6 and their
   Profile Source associations;
8. Workspace integrity passed with seven referenced Blobs;
9. the App created and verified a `canisend.backup/v2` backup with seven Blobs, restored it into a
   separate empty path, and reopened both Applications from the restored Workspace.

The review action stayed disabled until private-read consent was granted, and export stayed
disabled until scoped private-write consent was granted. No external submission or provider send
occurred.

## Failures found and fixed

The first interaction pass found that Pack switching could retain the previous Pack's Requirement
category. The desktop now replaces a category that is absent from the active Pack and uses a
Pack-neutral empty-state label.

The second pass found that omitted Deliverables were sent with `manual-import`, allowing preview
to succeed before semantic commit failed. The desktop now sends no execution mode for an omitted
Deliverable, and the shared Store validation rejects this shape during preview for every caller.
One focused Rust regression and the existing desktop contract test retain both fixes.

## Remaining qualification boundary

This journey used a local ad-hoc-signed preview. Protected CI on the resulting commit, exact
clean-tag native packages, Codex and Claude Code dogfood against the frozen candidate, public
promotion, download, and public-byte reverification remain required before Alpha.7 publication.
The synthetic Workspace contained no confirmed Evidence item, so this interaction proves explicit
Profile Source association; the separate association and Agent v4 fixtures retain confirmed
Evidence success and denial coverage.
