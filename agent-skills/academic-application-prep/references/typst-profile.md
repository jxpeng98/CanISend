# Typst-First Profiles

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
