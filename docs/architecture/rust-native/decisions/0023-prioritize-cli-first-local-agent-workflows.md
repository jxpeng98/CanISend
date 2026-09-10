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

## Development-flow amendment — 2026-09-07

The owner requested removal of unnecessary development gates and repeated manual confirmation.
The ordering above expresses code dependencies, not a requirement to finish human qualification
before implementing the next slice. Automated single-Host protocol/lifecycle/resumption coverage
is sufficient to start dependent coordination work; real Host acceptance remains required before
qualifying the affected standalone or collaboration candidate.

Use `xtask source check` for development and independent CLI CI without frontend prerequisites.
Keep `xtask release check` as the complete candidate check, including exact human/artifact evidence
and feature-freeze dispositions. Pending release evidence is neither a development lock nor a
reason to rewrite historical records. Collect exact dispositions at qualification rather than
requesting process approval after each implementation step. Product consent, private-data access,
revision checks, audit, recovery and release publication authority are unchanged.

The owner's local-integration clarification makes branches and PRs optional development tools.
Reuse the current branch, merge completed work locally, and batch broader checks at meaningful
milestones. Keep focused bug and trust/data-boundary regressions. Request external review only
when needed; protected remote integration and release evidence keep their existing requirements.
Local integration does not claim remote CI or publication, and does not authorize unrelated
branch deletion, force pushes, or changing remote protections.

## Optional product delegation amendment — 2026-09-10

The owner requested an optional mode that reduces repeated product confirmations. Default MCP
behavior remains individual native forms. An eligible form can also accept an unchecked-by-default
Auto approval grant for one canonical Workspace path/UUID, Application UUID and exact Pack in the
current connection, with a 60-minute monotonic lifetime. A repository-owned allowlist permits
routine Application reads and Requirement, pasted Source, Plan, draft and review changes. Exact
Requirement Source references must already occur in the current Application. Shared Profile/
Evidence access and changes, exports, new Sources and unknown operations still ask individually.

This user-selected standing grant prospectively replaces per-call human confirmation for its
eligible scope. It does not derive authority from model arguments or add a model approval service.
Scope switches, denial, failed authorization, explicit cancellation, expiry and reconnect revoke it.
Every mutation keeps its exact preview, revision, digest, single-use token and existing audit.
Response metadata identifies delegated calls; persisted user-authority fields do not attest that
the human individually inspected every automatic decision. Missing facts and final acceptance
remain the user's decisions. No external tool, network, upload or submission authority is added.

The change leaves Agent/Workspace v4 payloads and stored formats intact; optional form and MCP
metadata fields carry the choice and status. Simulated protocol coverage is distinct from actual
Host acceptance and qualification of a newly built release artifact.
