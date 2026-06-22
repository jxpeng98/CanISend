# Typst-First Profiles

Use this reference when the user's source CV, cover letter base, research statement, or teaching statement is Typst.

## Profile Modes

Profile initialization supports:

- `markdown`: Markdown evidence files only.
- `typst`: Typst manifest and Typst sources only.
- `hybrid`: both Markdown evidence files and Typst-first sources.

Use `profile/profile.yaml` as the local manifest. Real Typst profile sources belong in:

```text
profile/typst/
  cv.typ
  cover_letter_base.typ
  research_statement.typ
  teaching_statement.typ
```

Normalized evidence should be generated into:

```text
profile/generated/
  cv.evidence.md
  research_statement.evidence.md
  teaching_statement.evidence.md
```

The evidence layer is what matchers and checkers should read. Typst remains the human-facing source format.

Users may replace the starter Typst files with fully written private sources that already use `modernpro-cv` and `modernpro-coverletter`. The application pipeline should not rewrite those profile sources. For each job, it writes structured Typst data under `jobs/<job-slug>/typst/` and maps that data into `modernpro-coverletter` source files.

## Agent Rules

- Treat `profile/typst/*.typ` as private user-authored source.
- Do not rewrite the user's CV or statements unless the user explicitly asks for edits to those private files.
- Prefer writing suggestions to `04_cv_tailoring_notes.md` over directly changing `profile/typst/cv.typ`.
- When finalizing application-specific cover letter content, edit Markdown drafts first, then directly edit `jobs/<job-slug>/typst/cover_letter.typ`.
- Keep generated job-specific `.typ` files bounded by `// CANISEND: section ...` markers; do not rewrite unrelated sections.

## Evidence Extraction Limits

Current extraction is conservative. It recognizes:

- `#section("...")`
- Typst headings such as `= Research Statement`
- common modernpro-style blocks such as `#education(...)`, `#job(...)`, `#award(...)`, and reference/publication blocks
- multi-line modernpro-style entries such as `#dated-entry(...)`, `#entry(...)`, and `#event(...)`
- statement paragraphs and bullet lines under Typst headings
- publication list lines such as `+ @paper2025`

Generated evidence receives stable item IDs such as `cv-001`. New generated materials should cite `profile/generated/file.evidence.md#Section/item-id` instead of citing private Typst source directly.

If an important claim is not extracted into `profile/generated/`, report it as a profile evidence gap and ask the user whether to add structured evidence. Do not cite unextracted private Typst prose as if it were normalized evidence.

## Job-Specific Typst Outputs

The pipeline writes:

```text
jobs/<job-slug>/typst/
  cover_letter.typ
  application_package.typ
```

The `.typ` files are the preferred agent-editable interface. Content JSON artifacts may exist for compatibility/debugging, but they are secondary and should not be the normal editing surface.
