# Alpha.10 lean Beta.1 readiness review

## Scope

This body-free maintainer review supports `M4-READY-001`. It reviews exact public Alpha.10 as the
input to a separate Beta.1 stage task. It does not change Alpha.10 bytes, authorize a tag or
publication, activate feature freeze, or claim invited-user evidence.

## Exact baseline

- Public tag: `v1.0.0-alpha.10`
- Protected source: `cd40180f2ff8ac957276f1948ba88da428511a82`
- Candidate run: `32678848156`
- Candidate artifact: `9503978913` / `canisend-v1.0.0-alpha.10-release-assets`
- Promotion/public-verification run: `33267148891`
- Public release: `https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.10`
- Provider schema: `canisend.provider-dogfood/v2`
- Provider evidence note:
  `docs/notes/rust-native/2026-08-29-alpha10-codex-first-qualification.md`

The candidate and public assets have the recorded same-byte identity. The provider record binds
the Academic and Generic Codex CLI preview/cancel scenarios, Agent v4, Workspace v4,
host-resource v4, both Pack digests, and all four Skill digests. Neither scenario mutated an
Application or performed a submission.

## Maintainer review

Known limitations were reviewed against the active release guidance. The following remain
intentional and accurately disclosed:

- community packages do not provide normal operating-system trust;
- CanISend never logs in, uploads, sends, or submits an Application;
- image-only PDFs require separate trusted OCR and user review;
- Codex CLI is the required external-host evidence boundary; Claude real-host sessions are
  non-blocking compatibility observations;
- invited-user evidence has not started and contributes zero users and zero user flows;
- Beta.1, feature freeze, RC, and Stable remain separate pending outcomes.

The reviewed blocker classes are data loss, privacy, evidence, Pack integrity, rendering,
recovery, host setup, supported install, and release integrity. Their non-Issue evidence is the
exact Alpha.10 public release matrix, the two Codex provider scenarios, and this maintainer review.
The readiness writer separately rechecks public Issue number/state/labels and refuses any open
Issue carrying both `priority:P0` and `state:blocked`; planned future Roadmap Issues may remain
open.

## Cohort boundary

The readiness record must report:

- synthetic users: `0`;
- invited users: `0`;
- completed user flows: `0`;
- cohort start: public `v1.0.0-beta.1`;
- cohort completion deadline: before `v1.0.0-rc.1`.

Synthetic maintainer dogfood is release engineering evidence, not user evidence.

## Retention boundary

This note retains no application body, prompt, transcript, Issue title/body/comment, local
Workspace path or object identifier, credential, provider token, approval token, or private user
content.
