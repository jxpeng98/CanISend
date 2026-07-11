# CanISend Agent Bridge

Use the project skill at `agent-skills/canisend/SKILL.md` before preparing academic job application materials.

## Model Exposure Modes

Be explicit about the mode you are operating in:

- Direct CLI deterministic mode: local `canisend` commands without LLM flags can run without sending profile or job text to a model provider.
- Agent-assisted mode is not local-only: any file, PDF, webpage, or generated material you read or summarize may be processed by the agent model provider.
- LLM-backed CLI mode: `extract-profile-evidence --llm-augment`, `--llm-parser`, `--llm-drafts`, or a command provider may transmit selected private advert, profile, evidence, and draft context to the configured provider.

Start every workflow by identifying the private workspace and requesting its machine-readable context:

```bash
canisend agent context --workspace . --format json
```

If running from outside the workspace, replace `.` with the workspace path.
Use `canisend doctor --workspace .` when a human-readable environment diagnostic is also useful.

Core rules:

- Allowed by default: run local deterministic `canisend` commands, inspect generated evidence, and review current job metadata/artifacts needed for the task.
- Ask first: before reading full private CVs, statements, full job adverts, references, PDFs, source URLs, Evidence
  snapshots/candidates/catalogs, generated packages, or before enabling LLM-backed CLI flags/providers.
- Never do: submit applications, fill portals, create accounts, scrape full job pages, answer sensitive declarations, upload packages, or fabricate evidence.
- Do not quote private materials in chat unless the user explicitly asks.
- Do not stage private files such as `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real application packages.
- Use `canisend` CLI commands for workspace actions.
- Prefer generated evidence from `profile/generated/`; report gaps rather than inventing claims.
- Resume the deterministic Decision Spine through `stage run --stage evidence|parse|confirm|match`; Evidence and Match
  require no platform API or configured provider. Never write their run files or authoritative outputs directly.
- Re-extract older Typst-backed generated evidence when its source-hash receipt is missing or stale. Resumable Evidence
  rejects workspace-external profile roots.
- Treat every `criterion_matches.json` classification as `review_state=proposed`, not a Decision or readiness claim.
- Evidence snapshots, candidates, and catalogs may duplicate private profile bodies until the user removes the run or
  job; privacy-safe workflow control records and Match output do not contain those bodies.
- Read `agent-skills/canisend/references/platforms.md` for cross-platform guidance.
- Read `agent-skills/canisend/references/privacy.md` before summarizing, quoting, staging, or committing private application data.
- Read `agent-skills/canisend/references/quality-gates.md` before presenting generated materials as ready.
