# Provider Config

Use this reference before running LLM-backed parser or draft generation.

## Provider Modes

The default deterministic pipeline needs no provider.

LLM-backed behavior is explicit opt-in:

```bash
canisend run --workspace <private-workspace> --job jobs/<job-slug> --llm-parser
canisend run --workspace <private-workspace> --job jobs/<job-slug> --llm-drafts
```

Use both flags only when the user wants both provider-backed parsing and provider-backed drafting.

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

Use this when the user wants Codex, Claude Code, Gemini, or another local CLI to act as the model provider:

```bash
ACADEMIC_PREP_LLM_PROVIDER=command
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300
```

The command must read the prompt from stdin and write the completion to stdout. It should return non-zero on failure.

Examples the user may adapt:

```bash
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_COMMAND="claude -p"
ACADEMIC_PREP_LLM_COMMAND="gemini -p"
```

Do not assume a specific CLI is installed. Run `canisend doctor --workspace <private-workspace>` and ask the user to configure missing provider settings.

## Output Requirements

The parser provider must return JSON matching `schemas/parsed_job.schema.json`. It must not invent missing advert fields.

Draft providers must cite evidence as backticked references such as:

```text
`profile/generated/cv.evidence.md#Teaching`
```

Unknown citations fail validation. Missing evidence should be marked as a risk or gap, not replaced with unsupported claims.
