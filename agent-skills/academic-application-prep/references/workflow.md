# Workflow

1. Run `uv run academic-prep init-profile --mode hybrid`.
2. Fill local private profile files under ignored `profile/`.
3. Fetch jobs.ac.uk RSS leads with `uv run academic-prep fetch-jobs-ac-uk`.
4. Create one job workspace with `uv run academic-prep new-job`.
5. Paste or import the selected advert into `job_advert.md`.
6. Run `uv run academic-prep run --job jobs/<job-slug>`.
7. Review `parsed_job.json`, criteria checklist, fit report, cover letter, CV notes, and final package.
8. Optionally run `uv run academic-prep render-typst --job jobs/<job-slug>`.
9. Submit manually outside the tool.
