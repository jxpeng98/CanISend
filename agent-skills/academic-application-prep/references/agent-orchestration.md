# Agent Orchestration

This project is intended to work well with Codex, Claude Code, Gemini, or another local coding agent.

Before touching private data, agents can inspect `examples/end_to_end/README.md` and run `uv run pytest tests/test_examples.py -v` from a development checkout to see the full local workflow with fake data.

Agents should coordinate the workflow through private workspace files and CLI commands:

1. Read this skill before changing prompts, file contracts, or workflow behavior.
2. For a new installed-package workspace, run `academic-prep init-workspace --workspace <private-workspace>` and `academic-prep doctor --workspace <private-workspace>`.
3. Fetch or refresh jobs.ac.uk RSS leads with `academic-prep fetch-jobs-ac-uk`.
4. Ask the user to choose a lead, or use an explicitly provided lead index.
5. Create the job workspace with `academic-prep new-job-from-lead`.
6. Ensure the full advert is pasted into `jobs/<job-slug>/job_advert.md`; RSS leads are not full adverts.
7. Run `academic-prep extract-profile-evidence` when private profile Typst sources changed.
8. Run `academic-prep run --job jobs/<job-slug>` with `--llm-parser` and `--llm-drafts` only when provider config is available.
9. Review generated evidence citations, criteria coverage, cover letter content JSON, and final package before rendering.
10. Optionally run `academic-prep render-typst --job jobs/<job-slug>`.
11. Leave final submission, sensitive declarations, and portal interaction to the user.

From a development checkout, prefix CLI commands with `uv run`.

Agents must treat `profile/`, `jobs/`, and `job_leads/` as private local data. Do not commit real CVs, statements, job adverts, generated packages, PDFs, or source URLs that reveal application strategy.

The Typst layer is structured. Agents should update `03_cover_letter_draft.md` or `jobs/<job-slug>/typst/cover_letter_content.json` and let `cover_letter.typ` use `modernpro-coverletter`; do not replace this with line-by-line Markdown-to-Typst conversion.
