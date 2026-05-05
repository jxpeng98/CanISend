# Workflow

For a fake-data reference workflow, inspect `examples/end_to_end/README.md`.

1. For installed-package use, run `academic-prep init-workspace --workspace <private-workspace>`.
2. Run `academic-prep doctor --workspace <private-workspace>` and resolve missing profile/provider/tooling items.
3. Fill local private profile files under ignored `profile/`; in Typst-first workflows, these are already-written `modernpro-cv` and `modernpro-coverletter` sources.
4. Run `academic-prep extract-profile-evidence --workspace <private-workspace>`.
5. Fetch jobs.ac.uk RSS leads with `academic-prep fetch-jobs-ac-uk --workspace <private-workspace>`.
6. Create one job workspace with `academic-prep new-job-from-lead --workspace <private-workspace>` or `academic-prep new-job --workspace <private-workspace>`.
7. Paste or import the full selected advert into `job_advert.md`; RSS leads are not full adverts.
8. Run `academic-prep run --workspace <private-workspace> --job jobs/<job-slug>`.
9. Review `parsed_job.json`, criteria checklist, fit report, cover letter, CV notes, Typst content JSON, and final package.
10. Optionally run `academic-prep render-typst --workspace <private-workspace> --job jobs/<job-slug>`.
11. Submit manually outside the tool.

From a development checkout, prefix commands with `uv run`.

Use `academic-prep update-workspace --workspace <private-workspace>` after package upgrades to copy newly packaged defaults without overwriting local edits. Use `--overwrite` only when replacing local prompt/template/skill copies is intentional.
