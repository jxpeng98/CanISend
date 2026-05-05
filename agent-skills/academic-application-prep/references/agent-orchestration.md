# Agent Orchestration

This project is intended to work well with Codex, Claude Code, Gemini, or another local coding agent.

Before touching private data, agents can inspect `examples/end_to_end/README.md` and run `uv run pytest tests/test_examples.py -v` to see the full local workflow with fake data.

Agents should coordinate the workflow through repository files and CLI commands:

1. Read this skill before changing prompts, file contracts, or workflow behavior.
2. Fetch or refresh jobs.ac.uk RSS leads with `uv run academic-prep fetch-jobs-ac-uk`.
3. Ask the user to choose a lead, or use an explicitly provided lead index.
4. Create the job workspace with `uv run academic-prep new-job-from-lead`.
5. Ensure the full advert is pasted into `jobs/<job-slug>/job_advert.md`; RSS leads are not full adverts.
6. Run `uv run academic-prep extract-profile-evidence` when private profile Typst sources changed.
7. Run `uv run academic-prep run --job jobs/<job-slug>` with `--llm-parser` and `--llm-drafts` only when provider config is available.
8. Review generated evidence citations, criteria coverage, cover letter content JSON, and final package before rendering.
9. Optionally run `uv run academic-prep render-typst --job jobs/<job-slug>`.
10. Leave final submission, sensitive declarations, and portal interaction to the user.

Agents must treat `profile/`, `jobs/`, and `job_leads/` as private local data. Do not commit real CVs, statements, job adverts, generated packages, PDFs, or source URLs that reveal application strategy.

The Typst layer is structured. Agents should update `03_cover_letter_draft.md` or `jobs/<job-slug>/typst/cover_letter_content.json` and let `cover_letter.typ` use `modernpro-coverletter`; do not replace this with line-by-line Markdown-to-Typst conversion.
