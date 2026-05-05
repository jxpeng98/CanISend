# Academic Application Preparation Copilot - V1 Engineering Proposal

## 1. Project Definition

**Project name:** Academic Application Preparation Copilot

**Short name:** AAP Copilot

**Chinese positioning:** 学术岗位申请材料准备助手

AAP Copilot is a **local-first, auditable, semi-automated academic job application material preparation system**. It helps a user turn an academic job advert and a local academic profile into evidence-based application materials, criteria coverage checks, and Typst-ready source files.

This project is **not** an auto-application bot. It prepares materials; it does not create accounts, fill application portals, submit forms, or answer sensitive declarations.

### 1.1 One-Sentence Definition

AAP Copilot is a local CLI tool that converts academic job adverts and a Markdown-based academic profile into tailored, evidence-grounded application materials and Typst-ready source files for manual review and submission.

### 1.2 Target Users

V1 is designed for individual academic job applicants, especially:

- PhD candidates
- Early-career researchers
- Postdoctoral researchers
- Teaching Fellows
- Research Fellows
- Lecturers applying for new academic roles
- Applicants in Economics, Finance, Management, Business School, and related fields

### 1.3 Core Principle

```text
Prepare, do not submit.
Draft, do not fabricate.
Assist, do not replace judgement.
```

The system must make it easier to prepare strong materials, but the user remains responsible for review, editing, and final submission.

---

## 2. V1 Goal

The V1 goal is to build a **pure CLI + local file workflow** that runs end-to-end from job advert input to an application preparation package.

### 2.1 Inputs

V1 accepts:

- A local academic profile stored as Markdown/YAML files.
- A job advert pasted into the CLI or imported from local `.md` / `.txt` files.
- Optional job metadata such as source URL, deadline, institution, and department.
- Optional existing Typst templates stored inside the project.

### 2.2 Processing

V1 performs:

- Job advert normalization.
- Job information and criteria extraction.
- Profile evidence lookup.
- Fit analysis and risk identification.
- Cover letter drafting.
- CV tailoring note generation.
- Criteria coverage checking.
- Final application package assembly.
- Typst source generation.
- Optional Typst PDF compilation when explicitly requested and `typst` is installed locally.

### 2.3 Outputs

Application-facing generated outputs should be written in English:

- `01_job_summary.md`
- `02_fit_report.md`
- `03_cover_letter_draft.md`
- `04_cv_tailoring_notes.md`
- `05_criteria_checklist.md`
- `06_final_application_package.md`
- Typst source files under `typst/`
- Optional PDF files under `pdf/` when rendering is requested

Project documentation may include Chinese positioning and Chinese explanations where useful, but generated application materials should be English by default.

---

## 3. V1 Scope and Non-Goals

### 3.1 In Scope for V1

V1 should implement:

1. A local `profile/` folder containing Markdown evidence files and YAML metadata.
2. A local `templates/typst/` folder containing reusable Typst templates.
3. A `jobs/<job-slug>/` folder for each application preparation task.
4. `job.yaml` as lightweight job metadata and status tracking.
5. jobs.ac.uk RSS lead fetching with local keyword include/exclude filters.
6. Job advert import from pasted text, local Markdown, or local TXT.
7. `parsed_job.json` generation from `job_advert.md`.
8. Fit report generation with evidence references.
9. Cover letter draft generation with conservative academic language.
10. CV tailoring notes rather than full automatic CV rewriting.
11. Criteria coverage checking against essential and desirable criteria.
12. Final application package generation.
13. Typst source generation using project-managed templates based on `modernpro-cv` and `modernpro-coverletter`.
14. Optional Typst PDF compilation via explicit CLI command or flag.
15. An LLM provider abstraction supporting:
    - OpenAI-compatible API provider.
    - Configurable local command provider for CLI-based model access.

### 3.2 Out of Scope for V1

V1 must not implement:

- Automatic application submission.
- Automatic university account creation.
- Browser automation.
- CAPTCHA handling.
- Form filling on university application portals.
- Automatic completion of Equal Opportunities forms.
- Automatic responses to right-to-work, visa, disability, criminal record, or other sensitive declarations.
- Web crawling.
- URL scraping.
- PDF job advert import.
- DOCX import.
- Streamlit dashboard.
- SQLite database.
- Multi-user accounts.
- Cloud sync.
- SaaS deployment.
- Default PDF/DOCX export as the main delivery mechanism.

These features may appear in the roadmap, but they are not V1 requirements.

V1 RSS support is intentionally limited to jobs.ac.uk RSS feeds and local filtering. It does not scrape individual job pages.

---

## 4. Recommended V1 Project Structure

The project should be initialized from an empty folder into this structure:

```text
auto-academic-jobs/
  README.md
  pyproject.toml
  .env.example
  .gitignore

  profile/
    cv.md
    publications.md
    teaching_experience.md
    research_statement.md
    teaching_statement.md
    service_leadership.md
    grants_awards.md
    references.md
    personal_profile.yaml

  templates/
    typst/
      cover_letter.typ
      cv_notes.typ
      application_package.typ

  jobs/
    .gitkeep

  prompts/
    job_parser.md
    profile_matcher.md
    cover_letter_writer.md
    cv_tailor.md
    criteria_checker.md
    package_builder.md

  agent-skills/
    academic-application-prep/
      SKILL.md
      references/
        workflow.md
        file-contracts.md
        typst-profile.md
        privacy.md

  schemas/
    parsed_job.schema.json
    fit_report.schema.json
    criteria_check.schema.json

  src/
    academic_prep/
      __init__.py
      cli.py
      config.py
      models.py
      llm.py
      ingest.py
      parse.py
      match.py
      generate.py
      criteria.py
      package.py
      typst.py
      files.py

  tests/
    test_ingest.py
    test_parse.py
    test_match.py
    test_generate.py
    test_package.py
    test_typst.py
```

This structure is a target implementation shape, not a requirement that every file be created before the first working milestone.

---

## 5. Core Workflow

V1 should treat each job folder as one complete application preparation task.

### 5.1 Initialize Profile

```bash
academic-prep init-profile --mode hybrid
```

Creates the local profile folder if it does not exist:

```text
profile/
  profile.yaml
  cv.md
  publications.md
  teaching_experience.md
  research_statement.md
  teaching_statement.md
  service_leadership.md
  grants_awards.md
  references.md
  personal_profile.yaml
  typst/
    cv.typ
    cover_letter_base.typ
    research_statement.typ
    teaching_statement.typ
  generated/
    .gitkeep
```

Markdown files are one evidence source. Typst files can also be the human-facing profile source when the user already maintains CV and statements with `modernpro-cv` and `modernpro-coverletter`.

The local manifest is `profile/profile.yaml`. The normalized evidence layer should live under `profile/generated/`. Generated materials must cite evidence from Markdown files or generated evidence files at file + section/item level.

### 5.2 Generate Profile Evidence

```bash
academic-prep extract-profile-evidence --profile-dir profile
```

This reads `profile/profile.yaml`, extracts supported evidence from Typst-first profile sources, and writes Markdown evidence files under:

```text
profile/generated/
  cv.evidence.md
  research_statement.evidence.md
  teaching_statement.evidence.md
```

The job pipeline should read this normalized evidence layer before generating fit reports, criteria checks, or application drafts.

### 5.3 Create a Job

```bash
academic-prep new-job \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --source-url "https://example.edu/jobs/123"
```

Generated folder:

```text
jobs/2026-06-15_university-x_lecturer-in-economics/
  job.yaml
  job_advert.md
```

The user may paste the advert into the CLI, or import a local `.md` or `.txt` file.

### 5.4 Run the Preparation Pipeline

```bash
academic-prep run --job jobs/2026-06-15_university-x_lecturer-in-economics
```

The default parser should be deterministic and local. The LLM-backed parser must be an explicit opt-in so a blank project can run without provider credentials:

```bash
academic-prep run \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --llm-parser
```

The pipeline should also allow prompt directory overrides for experiments while keeping the built-in file contracts stable:

```bash
academic-prep run \
  --job jobs/2026-06-15_university-x_lecturer-in-economics \
  --llm-parser \
  --prompt-dir prompts
```

Expected generated files:

```text
jobs/2026-06-15_university-x_lecturer-in-economics/
  job.yaml
  job_advert.md
  parsed_job.json
  01_job_summary.md
  02_fit_report.md
  03_cover_letter_draft.md
  04_cv_tailoring_notes.md
  05_criteria_checklist.md
  06_final_application_package.md
  typst/
    cover_letter.typ
    application_package.typ
```

### 5.5 Render Typst Outputs

```bash
academic-prep render-typst --job jobs/2026-06-15_university-x_lecturer-in-economics
```

Rendering is optional. The command should:

- Check whether `typst` is available on the local machine.
- Compile generated `.typ` files into `pdf/`.
- Fail with a clear message if `typst` is not installed.
- Leave the generated `.typ` files intact.

Example optional output:

```text
jobs/2026-06-15_university-x_lecturer-in-economics/
  pdf/
    cover_letter.pdf
    application_package.pdf
```

---

## 6. File Contracts

### 6.1 Profile Folder

The Markdown profile files are the canonical evidence source.

Recommended files:

```text
profile/
  cv.md
  publications.md
  teaching_experience.md
  research_statement.md
  teaching_statement.md
  service_leadership.md
  grants_awards.md
  references.md
  personal_profile.yaml
```

Each file should use headings and bullet points so the system can cite evidence at file + section/item level.

Example evidence reference:

```text
profile/teaching_experience.md#Quantitative Methods Teaching
```

V1 does not require exact line-level citations or a chunk index.

### 6.2 Job Folder

Each job folder should contain all inputs and outputs for one role:

```text
jobs/<job-slug>/
  job.yaml
  job_advert.md
  parsed_job.json
  01_job_summary.md
  02_fit_report.md
  03_cover_letter_draft.md
  04_cv_tailoring_notes.md
  05_criteria_checklist.md
  06_final_application_package.md
  typst/
    cover_letter.typ
    application_package.typ
  pdf/
    cover_letter.pdf
    application_package.pdf
```

The `pdf/` folder is optional and exists only after explicit Typst rendering.

### 6.3 `job.yaml`

`job.yaml` is the lightweight job metadata and status file.

Example:

```yaml
id: 2026-06-15_university-x_lecturer-in-economics
title: Lecturer in Economics
institution: University X
department: Department of Economics
location: United Kingdom
deadline: 2026-06-15
source_url: https://example.edu/jobs/123
status: new
created_at: 2026-05-03T23:00:00Z
updated_at: 2026-05-03T23:00:00Z
notes: ""
```

Allowed status values:

```text
new
advert_imported
parsed
matched
drafted
packaged
ready_for_manual_review
submitted_manually
interview
rejected
offer
withdrawn
```

V1 does not submit applications, but post-submission statuses are useful for manual tracking.

### 6.4 `parsed_job.json`

`parsed_job.json` stores structured information extracted from `job_advert.md`.

Example:

```json
{
  "title": "Lecturer in Economics",
  "institution": "University X",
  "department": "Department of Economics",
  "location": "United Kingdom",
  "deadline": "2026-06-15",
  "salary": "Grade 7",
  "contract_type": "Permanent",
  "role_type": "Lecturer",
  "research_fields": ["Economics", "Finance", "Econometrics"],
  "teaching_fields": ["Statistics", "Econometrics", "Finance"],
  "essential_criteria": [
    {
      "criterion": "PhD or near completion in Economics or related field",
      "source_text": "PhD or near completion in Economics or a closely related discipline"
    }
  ],
  "desirable_criteria": [
    {
      "criterion": "Experience supervising dissertations",
      "source_text": "Experience of undergraduate or postgraduate dissertation supervision"
    }
  ],
  "required_documents": ["CV", "Cover letter", "Research statement", "Teaching statement"],
  "application_url": "https://example.edu/jobs/123",
  "unknown_fields": [],
  "notes": ""
}
```

Parser rules:

- Do not invent missing job information.
- Use `unknown` for unclear scalar fields.
- Use an empty list for unavailable list fields.
- Preserve original wording in `source_text` when possible.
- Separate essential and desirable criteria.
- Extract required documents when stated.

### 6.5 Evidence References

Generated claims must cite profile evidence using file + section/item references.

Example:

```markdown
- Strong quantitative teaching fit: supported by `profile/teaching_experience.md#Econometrics and Statistics Teaching`.
```

The system must not claim publications, teaching modules, awards, grants, supervision, committee work, or service roles unless those claims are supported by the profile files.

### 6.6 Typst Source Files

Generated Typst files should be stored under the job folder:

```text
jobs/<job-slug>/typst/
  cover_letter.typ
  application_package.typ
```

The generated `.typ` files should consume structured content generated by the pipeline and reuse project templates from:

```text
templates/typst/
```

V1 should generate Typst source even when PDF rendering is not requested.

---

## 7. LLM Provider Abstraction

V1 should define a small provider interface so the rest of the system does not depend on one model vendor.

### 7.1 Provider Types

V1 supports two provider categories:

1. **OpenAI-compatible API provider**
2. **Local command provider**

### 7.2 OpenAI-Compatible API Provider

This is the default provider.

Configuration should support:

```text
ACADEMIC_PREP_LLM_PROVIDER=openai-compatible
OPENAI_API_KEY=
OPENAI_BASE_URL=
OPENAI_MODEL=
```

Rules:

- The provider should work with services that expose an OpenAI-compatible chat completion API.
- API keys must come from environment variables or local config, not source code.
- The provider should expose clear errors for missing keys, unreachable base URLs, or invalid model names.

### 7.3 Local Command Provider

The command provider is a generic fallback for environments where the user has local CLI model access but no API access.

Configuration should support:

```text
ACADEMIC_PREP_LLM_PROVIDER=command
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300
```

The command provider should:

- Send the prompt and context to a configured local command.
- Capture stdout as the model response.
- Treat non-zero exit status as failure.
- Enforce a timeout.
- Avoid hardcoding Codex, Gemini, Claude Code, or any one CLI in V1.

Dedicated provider adapters for specific CLIs may be added later if needed.

### 7.4 Prompt Files and Agent Skills

Application prompt files should live in `prompts/`:

```text
prompts/
  job_parser.md
  profile_matcher.md
  cover_letter_writer.md
  cv_tailor.md
  criteria_checker.md
  package_builder.md
```

Each prompt file should define:

- Role
- Task
- Inputs
- Output format
- Constraints
- Evidence rules
- Quality criteria

Codex-readable project skills should live separately in `agent-skills/`:

```text
agent-skills/
  academic-application-prep/
    SKILL.md
    references/
      workflow.md
      file-contracts.md
      typst-profile.md
      privacy.md
```

This split prevents application prompts from being confused with agent skills.

---

## 8. Functional Modules

### 8.1 Profile Initialization

Creates the local `profile/` folder and starter Markdown/YAML files.

Acceptance criteria:

- Does not overwrite existing profile files unless the user explicitly confirms.
- Creates files with headings that support evidence references.
- Explains that Markdown is the canonical evidence source.

### 8.2 Job Import

Creates a job folder, `job.yaml`, and `job_advert.md`.

Supported input methods:

- Paste job advert text.
- Import local `.md`.
- Import local `.txt`.

Acceptance criteria:

- Stores the raw advert unchanged in `job_advert.md`.
- Saves source URL only as metadata.
- Does not fetch or scrape URLs in V1.
- Does not parse PDF files in V1.

### 8.3 Job Parser

Converts `job_advert.md` into `parsed_job.json` and `01_job_summary.md`.

Acceptance criteria:

- Extracts core job metadata.
- Extracts essential and desirable criteria separately.
- Extracts required documents when stated.
- Preserves important source wording.
- Marks missing information as `unknown` or an empty list.

### 8.4 Profile Matcher and Fit Report

Compares `parsed_job.json` with the Markdown profile evidence base.

Output:

```text
02_fit_report.md
```

Suggested fit dimensions:

```text
Research fit
Teaching fit
Methods/data fit
Department fit
Evidence strength
Application risks
```

Acceptance criteria:

- Every strong-fit claim cites profile evidence.
- Every gap includes a practical repair suggestion.
- The tone is conservative and avoids excessive optimism.
- Unsupported claims are flagged rather than inserted into drafts.

### 8.5 Cover Letter Draft Generator

Generates:

```text
03_cover_letter_draft.md
typst/cover_letter.typ
```

Recommended structure:

1. Opening paragraph
2. Research fit paragraph
3. Teaching fit paragraph
4. Departmental contribution paragraph
5. Service and leadership paragraph
6. Closing paragraph

Acceptance criteria:

- Uses a British academic tone by default.
- Aligns claims with job criteria and profile evidence.
- Does not invent experience.
- Leaves placeholders only where the user must manually decide content.

### 8.6 CV Tailoring Advisor

Generates:

```text
04_cv_tailoring_notes.md
```

V1 should not rewrite the full CV automatically. It should generate precise editing advice:

- Which sections to move higher.
- Which teaching experience to emphasize.
- Which research projects to foreground.
- Which methods or data skills to make visible.
- Which bullet points need rewriting.
- Whether the role calls for a teaching-focused or research-focused CV emphasis.

### 8.7 Criteria Coverage Checker

Generates:

```text
05_criteria_checklist.md
```

Suggested table:

```markdown
| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |
|---|---|---|---|---|
| PhD or near completion | Strong | `profile/cv.md#Education` | Low | Add expected completion date if not visible. |
| Teaching experience | Partial | `profile/teaching_experience.md#Seminar Teaching` | Medium | Name modules and student levels explicitly. |
```

Coverage values:

```text
strong
partial
weak
missing
```

Acceptance criteria:

- Checks essential criteria one by one.
- Checks desirable criteria separately.
- Cites evidence sources.
- Identifies missing evidence clearly.

### 8.8 Final Package Builder

Generates:

```text
06_final_application_package.md
typst/application_package.typ
```

The package should include:

1. Job information
2. Application strategy
3. Fit report summary
4. Cover letter draft
5. CV tailoring notes
6. Criteria coverage checklist
7. Required documents checklist
8. Manual submission notes
9. Remaining actions before submission

Acceptance criteria:

- The package is readable as a standalone application preparation dossier.
- Required documents are listed.
- Remaining manual actions are explicit.
- The package does not imply that the system submitted anything.

### 8.9 Typst Source Generator

Generates Typst source files under each job folder.

Acceptance criteria:

- Uses templates from `templates/typst/`.
- Writes generated `.typ` files under `jobs/<job-slug>/typst/`.
- Does not require Typst to be installed for source generation.
- Can optionally compile PDF when requested.

---

## 9. Quality and Safety Rules

### 9.1 Evidence-First Generation

The system must not invent:

- Publications
- Working papers
- Teaching modules
- Student feedback
- Grants
- Awards
- Supervision experience
- Committee roles
- Service or leadership roles
- Institutional affiliations

If useful evidence is missing, the system should state the gap and suggest what the user may add manually.

### 9.2 Conservative Academic Language

Generated materials should avoid:

- Unsupported excellence claims
- Exaggerated impact claims
- Generic institutional praise
- Empty enthusiasm
- Overconfident fit statements

The preferred style is specific, evidence-based, and academically professional.

### 9.3 Human-in-the-Loop Boundary

The user must manually review:

- Final cover letter
- CV edits
- Research statement
- Teaching statement
- Sensitive declarations
- Required uploads
- Final portal submission

The tool may prepare materials and check coverage, but it must not make final declarations on the user's behalf.

### 9.4 Traceability

Each generated output should be traceable to:

- The raw job advert.
- The parsed job fields.
- The profile evidence file and section/item.
- The prompt file used for generation.
- The timestamp of generation.

---

## 10. Recommended Implementation Stack

V1 recommended stack:

```text
Language: Python
Package manager: uv
CLI: Typer
Validation: Pydantic
Config: environment variables + local config file
Data storage: local files only
Profile source: Markdown + YAML
Generated application sources: Markdown + Typst
LLM providers: OpenAI-compatible API + generic command provider
Testing: pytest
```

V1 should not require a database, web server, dashboard, browser automation, or cloud service.

---

## 11. V1 Milestones

### Milestone 1: Project Skeleton and Profile Initialization

Deliverables:

- `pyproject.toml`
- `src/academic_prep/`
- `academic-prep init-profile`
- Starter `profile/` files
- `.env.example`

Acceptance criteria:

- A fresh project can create the expected profile folder.
- Existing profile files are not overwritten by default.

### Milestone 2: Job Import and Metadata

Deliverables:

- `academic-prep new-job`
- `job.yaml`
- `job_advert.md`

Acceptance criteria:

- A pasted, `.md`, or `.txt` advert creates one isolated job folder.
- URL is stored only as metadata.

### Milestone 3: LLM Provider Layer

Deliverables:

- OpenAI-compatible API provider.
- Generic command provider.
- Shared provider interface.
- Provider configuration validation.

Acceptance criteria:

- The rest of the pipeline calls one provider interface.
- API credentials are never hardcoded.
- Command provider failures are clear and recoverable.

### Milestone 4: Parser and Job Summary

Deliverables:

- `parsed_job.json`
- `01_job_summary.md`
- `prompts/job_parser.md`

Acceptance criteria:

- Essential and desirable criteria are separated.
- Missing fields are not invented.
- Important original wording is preserved.

### Milestone 5: Matching and Draft Generation

Deliverables:

- `02_fit_report.md`
- `03_cover_letter_draft.md`
- `04_cv_tailoring_notes.md`
- Prompt files for matcher, writer, and CV tailoring.

Acceptance criteria:

- Strong claims cite evidence.
- Gaps are visible.
- Cover letter draft is useful but clearly reviewable.

### Milestone 6: Criteria Checklist and Final Package

Deliverables:

- `05_criteria_checklist.md`
- `06_final_application_package.md`
- `typst/application_package.typ`

Acceptance criteria:

- Criteria checklist is actionable.
- Final package includes remaining manual actions.
- Typst source is generated without requiring PDF compilation.

### Milestone 7: Optional Typst Rendering

Deliverables:

- `academic-prep render-typst`
- Optional `pdf/` output folder

Acceptance criteria:

- Rendering succeeds when `typst` is installed.
- Missing Typst binary produces a clear error.
- Source generation remains independent of PDF rendering.

---

## 12. V1 Success Metrics

V1 is successful if:

- A user can create a profile and job folder from a blank project.
- A user can generate a complete application preparation package from a local job advert.
- Essential and desirable criteria are extracted into a reviewable structure.
- Generated materials cite profile evidence at file + section/item level.
- The cover letter draft contains no obvious fabricated experience.
- CV tailoring notes are specific and actionable.
- Criteria coverage identifies missing or weak evidence.
- Typst source files are generated for formal document production.
- The full workflow can be run without a database, dashboard, browser automation, or cloud service.

---

## 13. Roadmap After V1

### V1.1: Better Local Tracking

Possible additions:

- Job list command.
- Deadline sorting.
- Status summaries from `job.yaml`.
- Outcome and feedback notes.

This should still be file-based unless a database becomes necessary.

### V1.2: Document Import and Export

Possible additions:

- Local PDF job advert text extraction.
- DOCX/PDF export workflows.
- Typst template variants.
- Application package ZIP export.

### V1.3: Model Provider Enhancements

Possible additions:

- Dedicated Codex CLI adapter.
- Dedicated Gemini CLI adapter.
- Dedicated Claude Code adapter.
- Provider capability detection.
- Prompt/result logging for audit.

### V2.0: Lightweight UI

Possible additions:

- Streamlit or other local dashboard.
- Job list view.
- Fit score and deadline overview.
- Generated material previews.
- Manual status updates.

### V2.1: Job Discovery

Possible additions:

- Additional jobs.ac.uk feed presets.
- HigherEdJobs feed support if a stable feed source is available.
- EURAXESS import.
- Duplicate detection.

URL scraping and crawling should remain opt-in and explicitly bounded.

### V3.0: Advanced Desktop Workflow

Possible additions:

- Tauri or desktop UI.
- Local document store.
- Workflow orchestration.
- Optional browser assistant for upload preparation.

Even in later versions, final application submission should remain manual unless the project is explicitly re-scoped.

---

## 14. Final V1 Summary

AAP Copilot V1 should be implemented as:

```text
Python CLI + local files + Markdown evidence base + LLM provider abstraction + Typst source generation
```

The core user value is:

- Extract the structure of an academic job advert.
- Match it against a local academic profile.
- Generate evidence-grounded application drafts.
- Check criteria coverage before submission.
- Produce Typst-ready sources for polished final materials.

The system should remain local-first, auditable, conservative, and human-reviewed.
