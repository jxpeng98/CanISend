# ADR-015: Route Configured-Provider Draft Through The Guarded Stage Runtime

**Status:** Accepted

**Date:** 2026-07-14

## Context

ADR-013 established a guarded host-agent path for structured Cover Letter Drafts. The current runtime owns an
immutable TaskSpec, private candidate staging, strict validation, atomic promotion, receipts, and recovery. The
legacy `--llm-drafts` path predates this boundary and produces free-form compatibility materials, so extending it
would create a second source of truth and would not produce claim-level evidence receipts.

The structured Draft model already reserves `configured_provider`, but the registry, runtime, validator, and CLI
intentionally reject that mode until provider execution can reuse the same controls as host-agent execution.

## Decision

CanISend adds configured-provider execution only for the structured Cover Letter Draft stage. The public entrypoint
is:

```bash
canisend stage run --stage draft --mode configured-provider --allow-provider-backed
```

`--allow-provider-backed` is a per-command Tier 3 consent. Without it, CanISend does not construct or call a
provider and does not create workflow state. A cache hit does not transmit data. Provider configuration and
execution failures return stable body-free errors that do not include provider output, stderr, credentials, private
paths, or input bodies.

### One execution and promotion path

- Draft declares both `host_agent` and `configured_provider` in the stage registry.
- Configured-provider execution prepares or reuses the same immutable TaskSpec and exact seven-input fingerprint as
  host-agent execution.
- The TaskSpec marks the request Tier 3 and requires `send-private-draft-inputs-to-provider` consent.
- The core reads only TaskSpec-declared inputs whose bytes still match the declared hashes.
- A packaged prompt treats every input as untrusted data and asks for a bounded sections-and-claims proposal only.
- The provider cannot choose job/document identity, basis hashes, input fingerprint, generation mode, generator
  identity, review state, Claim IDs, or aggregate blockers. The core derives that envelope and stable Claim IDs.
- The resulting full candidate passes through the existing `stage submit` validator and `stage apply` atomic
  promotion path. The validator binds `generation_mode` to the immutable TaskSpec execution mode.

### Privacy and recovery

The provider request may contain full Tier 2 job, evidence, Decision, and Brief bodies, so the request itself is
Tier 3. Normal status and AgentResponse output remain body-free. Raw provider responses are treated as untrusted,
bounded input and are never persisted; only a schema-valid canonical candidate may enter the private run directory.

If a provider call or proposal fails, no candidate or authoritative Draft is written and the prepared task remains
reusable. If candidate submission completed before interruption, a retry reuses the immutable candidate/result and
continues promotion without another provider call. If any declared input changes during generation, submission
fails closed and the authoritative Draft remains unchanged.

## Consequences

- Host-agent and configured-provider Drafts share one validator, one candidate contract, and one promotion path.
- A model cannot self-assign trusted identities or mark its Draft reviewed, final, or package-ready.
- Provider retries do not leak raw responses into workflow state or normal diagnostics.
- Existing host-agent behavior and the legacy `--llm-drafts` compatibility flag remain available but distinct.
- Deterministic Review and user-owned finding dispositions remain required after either Draft mode.

## Rejected Alternatives

- Promote provider JSON directly: rejected because it bypasses TaskSpec freshness, candidate validation, and atomic
  promotion.
- Ask the provider to calculate stable hash-derived Claim IDs: rejected because identity is a core responsibility
  and model-generated hashes are not trustworthy.
- Persist raw provider output for debugging: rejected because it can contain private input echoes, secrets, prompt
  injection text, or arbitrary unvalidated bodies.
- Convert legacy Markdown `--llm-drafts` output into authoritative structured state: rejected because it lacks the
  exact structured basis and claim-level evidence contract.
- Fall back silently from provider mode to host-agent or legacy generation: rejected because execution mode and
  privacy consent must remain explicit.

## Revisit When

Revisit before provider-backed Parse enters the stage runtime, before supporting streaming/tool-calling providers,
or before provider execution spans multiple documents or transactions.
