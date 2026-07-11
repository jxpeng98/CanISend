# CanISend Claude Bridge

@agent-skills/canisend/SKILL.md

Start every workflow by identifying the private workspace and requesting its machine-readable context:

```bash
canisend agent context --workspace . --format json
```

If running from outside the workspace, replace `.` with the workspace path.
Use `canisend doctor --workspace .` when a human-readable environment diagnostic is also useful.

Agent boundaries:

- Direct CLI deterministic mode can be local-only. Agent-assisted mode is not local-only: any file, PDF, webpage, or generated material you read or summarize may be processed by the agent model provider.
- LLM-backed CLI mode (`extract-profile-evidence --llm-augment`, `--llm-parser`, `--llm-drafts`, or command providers) may transmit selected private advert, profile, evidence, and draft context to the configured provider.
- Allowed by default: run local deterministic `canisend` commands, inspect generated evidence, and review current job metadata/artifacts needed for the task.
- Ask first: before reading full private CVs, statements, full job adverts, references, PDFs, source URLs, Evidence
  snapshots/candidates/catalogs, generated packages, `criteria.json`, or `criterion_matches.json`; also ask before
  enabling LLM-backed CLI flags/providers. Criteria may contain corrected wording, and Match remains Tier 2 even
  though it is body-minimized.
- Never do: submit applications, fill portals, create accounts, scrape full job pages, answer sensitive declarations, upload packages, or fabricate evidence.
- Do not quote private materials in chat unless the user explicitly asks.
- Do not stage private files such as `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real application packages.
- Prefer generated evidence from `profile/generated/`; report gaps rather than inventing claims.
- Resume the deterministic Decision Spine through `stage run --stage evidence|parse|confirm|match`; Evidence and Match
  require no platform API or configured provider. Never write their run files or authoritative outputs directly.
- Re-extract older Typst-backed generated evidence when its source-hash receipt is missing or stale. Resumable Evidence
  rejects workspace-external profile roots.
- Treat every `criterion_matches.json` classification as `review_state=proposed`, not a Decision or readiness claim.
- Use `corrections status|init|update`, `decision status|init|update`, and `user-mutation recover` for user-owned
  writes. Never directly overwrite or normalize either YAML; use one strict patch, current revision/hash, and explicit
  `--confirm-user-owned-write`. Empty initialization is fingerprint-neutral; rerun Confirm after each semantic
  correction before another.
- Unknown is not confirmed empty; undecided is not apply/hold/skip. A Decision keeps its value when its derived basis
  becomes review-required and must be explicitly reconfirmed.
- Evidence snapshots, candidates, and catalogs may duplicate private profile bodies until the user removes the run or
  job; privacy-safe workflow control records and Match output do not contain those bodies.
- User YAML/private mutation candidates/corrected Criteria are Tier 2; Tier 1 receipts and AgentResponse never contain
  correction text or rationale. CAS assumes a stable job directory and cooperative writers, so avoid concurrent
  manual saves.
- Reset/clear/withdraw is not erasure: private-mode candidates (0600 on POSIX) and correction history remain for audit/recovery.
  Keep jobs private/git-ignored and never promise automatic secure deletion from the job, backups, or snapshots.

For cross-platform details, read @agent-skills/canisend/references/platforms.md.
For privacy details, read @agent-skills/canisend/references/privacy.md.
