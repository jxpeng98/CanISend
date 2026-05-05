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
- When generating application-specific cover letter content, edit Markdown drafts or `jobs/<job-slug>/typst/cover_letter_content.json`.
- Keep `cover_letter.typ` as a structured renderer that imports `modernpro-coverletter`.

## Evidence Extraction Limits

Current extraction is conservative. It recognizes:

- `#section("...")`
- Typst headings such as `= Research Statement`
- common modernpro-style blocks such as `#education(...)`, `#job(...)`, `#award(...)`, and reference/publication blocks
- publication list lines such as `+ @paper2025`

If an important claim is not extracted into `profile/generated/`, report it as a profile evidence gap and ask the user whether to add structured evidence. Do not cite unextracted private Typst prose as if it were normalized evidence.

## Job-Specific Typst Outputs

The pipeline writes:

```text
jobs/<job-slug>/typst/
  cover_letter_content.json
  cover_letter.typ
  application_package_content.json
  application_package.typ
```

The content JSON files are the preferred agent-editable interface. The `.typ` files should remain stable renderer sources unless the template contract changes.
