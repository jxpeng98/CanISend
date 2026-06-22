---
name: canisend-humanizer
description: Use when humanizing, smoothing, or making CanISend-generated application text more coherent, specific, and human while preserving evidence, citations, privacy boundaries, and job criteria.
---

# CanISend Humanizer

Focus only on making CanISend-generated application text read like a coherent person wrote it. Use this for cover letters, research statements, teaching statements, CV profile text, criteria responses, and short application summaries.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, motivation, achievements, institutional fit, lived experience, citations, dates, metrics, or personal details.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, PDFs, or enabling LLM-backed commands.
- Prefer `profile/generated/` evidence, `parsed_job.json`, criteria checklists, and existing draft text over raw private files.
- Preserve evidence citations and explicit uncertainty. A smoother sentence must not make a weak claim sound stronger than the source allows.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence, citation, and readiness gates.
- `../canisend/references/file-contracts.md`: generated draft, criteria, review, and Typst output paths.
- `../canisend/references/workflow.md`: local-first generation and manual submission boundaries.

## Workflow

1. Identify the target document, audience, and job stage: cover letter, statement, CV text, criteria response, or package summary.
2. Build a source map before rewriting: which claims come from `profile/generated/`, which criteria come from `parsed_job.json` or checklists, and which details are unsupported.
3. Find the through-line for the passage: why this applicant fits this role, what the paragraph proves, and what the reader should remember.
4. Rewrite around that through-line instead of polishing sentence-by-sentence. Each sentence should connect by cause, contrast, sequence, or emphasis.
5. Add only grounded details from the source map: role names, criteria language, project names, methods, teaching areas, timelines, or evidence-backed outcomes.
6. Remove stitched-together assistant prose: generic enthusiasm, repeated "I am excited", list-like paragraphs, empty transitions, and claims that sound broad but prove little.
7. Keep the applicant's voice proportionate: confident where evidence is strong, careful where evidence is partial, and explicit about gaps.
8. Before saying the text is ready, check `../canisend/references/quality-gates.md`.

## Coherence Checks

Before returning revised text, verify:

- The opening sentence has a real job/application reason, not a generic warm-up.
- Each paragraph has one job: motivation, evidence, fit, contribution, or closing.
- Details explain a claim rather than decorate it.
- Criteria language is woven into normal prose instead of pasted in as keywords.
- Transitions show why the next sentence follows: for example, because, in practice, this matters for, this connects to, or taken together.
- No new fact, citation, motivation, or emotion has been invented.

## Output

For simple rewrites, output only the revised text.

When the rewrite touches claims, citations, or private evidence, include:

- Revised text
- Evidence-preservation notes
- Unsupported details or claims the user should confirm

If the source text is too thin to humanize safely, ask for 2-4 concrete details: target role, audience, one real evidence point, and the tone the user wants.

## Style Moves

Prefer:

- Specific nouns and verbs from the job and evidence files
- Measured confidence instead of generic excitement
- Short connective phrases that explain why ideas belong together
- Slightly varied paragraph lengths that follow the argument
- Concrete examples that are already present in CanISend artifacts

Avoid:

- Random human-sounding flourishes that make the draft feel patched together
- Invented personal motivation or institutional familiarity
- Orphaned details that do not change the reader's understanding
- Over-polished assistant symmetry, bullet-list prose, and repeated sentence frames
- Replacing evidence citations with smoother but unverifiable claims
