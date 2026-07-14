# Provider Config

Use this reference before running LLM-backed profile-evidence augmentation, parser, draft generation, or command-provider workflows.

## Provider Modes

The default deterministic pipeline needs no provider.

LLM-backed behavior is explicit opt-in:

```bash
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> \
  --stage draft --mode configured-provider --allow-provider-backed --format json
canisend run --workspace <private-workspace> --job jobs/<job-slug> --llm-parser
canisend run --workspace <private-workspace> --job jobs/<job-slug> --llm-drafts
canisend extract-profile-evidence --workspace <private-workspace> --llm-augment
```

Use each flag only when the user wants that provider-backed step.

Profile evidence augmentation is explicit opt-in through `extract-profile-evidence --llm-augment`. It is not enabled by default.

Before enabling provider-backed parsing, drafting, profile augmentation, or a command provider, tell the user it
transmits private advert and evidence context, plus selected profile, Decision, Brief, or draft context, to the
configured provider or local command. Structured configured-provider Cover Letter Draft sends
exactly `parsed_job.json`, `criteria.json`, `evidence_catalog.json`, `criterion_matches.json`,
`application_decision.yaml`, `application_brief.yaml`, and `required_document_plan.json`. Require
`--allow-provider-backed` on each non-cached Cover Letter Draft invocation. Research Statement configured-provider
execution is not implemented. Do not add that flag, `--llm-augment`,
`--llm-parser`, `--llm-drafts`, or `ACADEMIC_PREP_LLM_PROVIDER=command` unless the user explicitly approves that
mode for the current workspace or job.

The provider environment variable prefix remains `ACADEMIC_PREP_LLM_*` in V1 for compatibility with existing local workspaces and scripts. Treat it as the stable V1 provider config surface unless a later release documents a migration.

## OpenAI-Compatible Provider

Use this when the user has an OpenAI-compatible chat completions endpoint:

```bash
ACADEMIC_PREP_LLM_PROVIDER=openai-compatible
OPENAI_API_KEY=...
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=...
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300
```

`OPENAI_BASE_URL` can point to another compatible endpoint. Do not hardcode vendor-specific adapters into workflow instructions unless the project adds that adapter.

## Local Command Provider

Use this when the user wants Codex, Claude Code, or another local CLI to act as the model provider:

```bash
ACADEMIC_PREP_LLM_PROVIDER=command
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300
```

The command provider is separate from `canisend orchestrate`: it must read the prompt from stdin and
write the completion to stdout. It should return non-zero on failure. If a CLI expects the prompt as
an argument, use an adapter command or use orchestrator worker `prompt_mode: arg` instead.

The command provider inherits the same privacy boundary as any other LLM-backed mode: it may receive the full prompt, advert text, parsed job metadata, profile source snippets, generated evidence references, and draft context. Keep `.env` and API keys out of prompts and logs.

Examples the user may adapt:

```bash
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_COMMAND="my-stdin-compatible-model-cli"
```

Do not assume a specific CLI is installed. Run `canisend doctor --workspace <private-workspace>` and ask the user to configure missing provider settings.

## Output Requirements

Structured configured-provider Draft uses `prompts/structured_cover_letter_draft.md`. The provider returns one
bounded JSON object containing only `sections` and Claim semantics. It must not supply Claim IDs, hashes, job or
document identity, generation metadata, review state, or aggregate blockers; the trusted core derives those fields
and validates the canonical candidate through the same Draft validator used by host-agent submission. Raw provider
output is untrusted and is never stored. Invalid output, provider failure, or input drift cannot promote
`cover_letter_draft.json`; a previously submitted valid candidate resumes without another call.

Legacy `canisend run --llm-drafts` remains a separate compatibility path with its existing output contract.

The parser provider must return JSON matching `schemas/parsed_job.schema.json`. It must not invent missing advert fields.

Draft providers must cite evidence as backticked references such as:

```text
`profile/generated/cv.evidence.md#Teaching`
```

Unknown citations fail validation. Missing evidence should be marked as a risk or gap, not replaced with unsupported claims.

Profile-evidence augmentation providers must return structured evidence tied to local source chunks or existing generated evidence. Items without a verifiable local source are rejected and must not become unsupported claims.
