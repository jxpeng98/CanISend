# ADR-RN-0023: Prioritize CLI-first local Agent workflows

**Status:** Accepted direction; implementation and release qualification pending

**Date:** 2026-09-06

**Decision owner:** CanISend maintainer

## Context

The owner requested a return to CLI-first delivery, closure of the previous App development
stage, and integration into the existing Beta development line before choosing the next slice.
The supplied 2026-09-05 CLI-first execution package is design input, not an instruction to execute
all twelve work packages or to publish. Its `main` baseline differs from the current checkout.

R0/R1 have recorded source completion. R2 has partial local implementation and open provider,
consent, history, and qualification gates. Further embedded-client work is no longer the immediate
product priority. These facts must survive closeout without turning incomplete work into acceptance.

## Decision

- The primary delivery path is the existing native Rust CLI, MCP server, and verified Pack/Skill
  resources used through a user-selected external Agent. Core installation, configuration,
  approval, application preparation, export, and recovery must not require a CanISend window.
- Preserve `canisend-app` as the shared domain facade, Workspace v4/Application/Pack identities,
  SQLite/Blob authority, both built-in Packs, consent, revision checks, audit, and recovery.
- Order work as baseline reconciliation, CLI build/resource/install isolation, trusted headless
  approval, one complete Host journey and same-device resumption, then bounded same-device
  task/candidate/review coordination and conflict recovery. Qualify the standalone baseline and
  collaboration separately. Additional Hosts are optional after the first verified journey.
- Workers propose bounded candidates. Prefer an existing single Broker instance for final
  preview, reliable user confirmation, commit, and receipt. Cross-process tokens are not shared
  authority; a TTY, `--yes`, role claim, or model boolean alone is not evidence of human approval.
  Add a local coordination service only when a demonstrated cross-connection requirement needs it.
- Same-device collaboration means real running Host sessions sharing canonical local product
  records. It does not imply automatic workers, arbitrary same-user process isolation, or support
  across WSL/container/VM namespaces. Candidate reviews bind exact input versions and digests;
  concurrent commits must revalidate rather than silently overwrite.
- Close the App-first execution direction as superseded. Retain R0/R1 and partial R2 source and
  evidence for review; defer unfinished R2-R6 work, custom chat, ACP, and React/Electron changes.
  Existing GUI support and maintenance commitments remain until separately changed. No GUI or
  user-data deletion is part of this decision, and unqualified private embedded tools remain off.
- Cross-device migration/synchronization is last and separately scoped. Do not add another
  repository, common package, transcript authority, approval store, or general Agent platform.

This supersedes ADR-RN-0022's App-first priority and R2-R6 delivery ordering prospectively. Its
historical decisions and defensive constraints remain recorded. The existing master roadmap owns
the adopted LF-C01–12 mapping; the downloaded tasks are not a second status authority.

## Acceptance and release boundary

CLI source isolation, clean-machine consumer operation, real Host business acceptance, and exact
artifact qualification are separate evidence. Preserve historical Beta.1 and private Beta.2 facts,
the active freeze, and the existing user-evidence thresholds. A changed validation build requires
an explicit tested evidence-contract transition; historical flows cannot be relabelled.

The current request authorizes direction and stage closeout/integration, not all future feature
implementation, release publication, announcements, or support expansion. Resolve the exact merge
target before changing branch ancestry. Review existing uncommitted work and apply exact freeze
dispositions and required CI before claiming integration complete.
