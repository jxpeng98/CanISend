# Workflow

1. Run `uv run academic-prep init-profile --mode hybrid`.
2. Fill local private profile files under ignored `profile/`.
3. Run `uv run academic-prep extract-profile-evidence`.
4. Fetch jobs.ac.uk RSS leads with `uv run academic-prep fetch-jobs-ac-uk`.
5. Create one job workspace with `uv run academic-prep new-job-from-lead` or `uv run academic-prep new-job`.
6. Paste or import the full selected advert into `job_advert.md`; RSS leads are not full adverts.
7. Run `uv run academic-prep run --job jobs/<job-slug>`.
8. Review `parsed_job.json`, criteria checklist, fit report, cover letter, CV notes, and final package.
9. Optionally run `uv run academic-prep render-typst --job jobs/<job-slug>`.
10. Submit manually outside the tool.
