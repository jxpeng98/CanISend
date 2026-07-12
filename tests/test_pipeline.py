import hashlib
import json
from pathlib import Path
import subprocess
import sys

import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.git_tracking import APPLICATION_MATERIAL_RELATIVE_PATHS
from canisend.stage_runtime import inspect_stage_status, run_deterministic_stage


def _write_basic_job(tmp_path: Path) -> Path:
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "Department of Economics",
                "location": "United Kingdom",
                "deadline": "2026-06-15",
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text(
        """# Lecturer in Economics

Department: Department of Economics
Location: United Kingdom
Salary: Grade 7
Contract: Permanent
Role type: Lecturer
Research fields: Economics, Finance, Econometrics
Teaching fields: Statistics, Econometrics
Required documents: CV, Cover letter, Research statement, Teaching statement

Essential criteria:
- PhD or near completion in Economics or related field
- Evidence of teaching excellence

Desirable criteria:
- Experience supervising dissertations
""",
        encoding="utf-8",
    )
    return job_dir


def _set_advert_title(job_dir: Path, title: str) -> None:
    advert_path = job_dir / "job_advert.md"
    lines = advert_path.read_text(encoding="utf-8").splitlines()
    lines[0] = f"# {title}"
    advert_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _file_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def test_run_pipeline_generates_parsed_job_and_application_outputs(tmp_path):
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "Department of Economics",
                "location": "United Kingdom",
                "deadline": "2026-06-15",
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text(
        """# Lecturer in Economics

Department: Department of Economics
Location: United Kingdom
Salary: Grade 7
Contract: Permanent
Role type: Lecturer
Research fields: Economics, Finance, Econometrics
Teaching fields: Statistics, Econometrics
Required documents: CV, Cover letter, Research statement, Teaching statement

Essential criteria:
- PhD or near completion in Economics or related field
- Evidence of teaching excellence

Desirable criteria:
- Experience supervising dissertations
"""
    )
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir)])

    assert result.exit_code == 0
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    assert parsed_job["title"] == "Lecturer in Economics"
    assert parsed_job["institution"] == "University X"
    assert parsed_job["department"] == "Department of Economics"
    assert parsed_job["research_fields"] == ["Economics", "Finance", "Econometrics"]
    assert parsed_job["teaching_fields"] == ["Statistics", "Econometrics"]
    assert parsed_job["essential_criteria"][0]["criterion"] == "PhD or near completion in Economics or related field"
    assert parsed_job["desirable_criteria"][0]["criterion"] == "Experience supervising dissertations"
    assert parsed_job["required_documents"] == [
        "CV",
        "Cover letter",
        "Research statement",
        "Teaching statement",
    ]
    expected_outputs = [
        "00_preparation_questions.md",
        "01_job_summary.md",
        "02_fit_report.md",
        "03_cover_letter_draft.md",
        "04_cv_tailoring_notes.md",
        "05_criteria_checklist.md",
        "06_final_application_package.md",
        "07_material_review_checklist.md",
        "typst/cover_letter_content.json",
        "typst/cover_letter.typ",
        "typst/application_package_content.json",
        "typst/application_package.typ",
    ]
    for output in expected_outputs:
        assert (job_dir / output).exists()
    prep_questions = (job_dir / "00_preparation_questions.md").read_text()
    assert "US English or UK English" in prep_questions
    assert "grill me" in prep_questions.lower()
    assert "writing style" in prep_questions.lower()
    assert "specific motivation" in prep_questions.lower()
    assert "Remaining Actions Before Submission" in (job_dir / "06_final_application_package.md").read_text()
    cover_source = (job_dir / "typst" / "cover_letter.typ").read_text()
    package_source = (job_dir / "typst" / "application_package.typ").read_text()
    cover_content = json.loads((job_dir / "typst" / "cover_letter_content.json").read_text())
    assert '@preview/modernpro-coverletter:0.0.8' in cover_source
    assert '@preview/modernpro-coverletter:0.0.8' in package_source
    assert 'json("cover_letter_content.json")' not in cover_source
    assert 'json("application_package_content.json")' not in package_source
    assert "// CANISEND: section research_fit" in cover_source
    assert "// CANISEND: section criteria_checklist" in package_source
    assert "# Cover Letter Draft" not in cover_source
    assert "## Research Fit" not in cover_source
    assert cover_content["recipient"]["institution"] == "University X"
    assert cover_content["job"]["title"] == "Lecturer in Economics"
    review_checklist = (job_dir / "07_material_review_checklist.md").read_text()
    assert "03_cover_letter_draft.md" in review_checklist
    assert "04_cv_tailoring_notes.md" in review_checklist
    assert "Manual judgement required" in review_checklist
    updated_metadata = yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))
    assert updated_metadata["status"] == "packaged"
    assert updated_metadata["updated_at"] != "2026-05-03T23:00:00Z"
    assert updated_metadata["updated_at"].endswith("Z")


def test_legacy_run_preserves_equivalent_stage_parse_and_confirm_outputs(tmp_path):
    workspace = tmp_path
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    job_dir = _write_basic_job(workspace)
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    parsed_path = job_dir / "parsed_job.json"
    criteria_path = job_dir / "criteria.json"
    parsed_hash = _file_hash(parsed_path)
    parsed_mtime = parsed_path.stat().st_mtime_ns
    criteria_hash = _file_hash(criteria_path)

    result = CliRunner().invoke(
        app,
        [
            "run",
            "--workspace",
            str(workspace),
            "--job",
            str(job_dir.relative_to(workspace)),
            "--no-git-add-materials",
        ],
    )

    assert result.exit_code == 0, result.output
    assert _file_hash(parsed_path) == parsed_hash
    assert parsed_path.stat().st_mtime_ns == parsed_mtime
    assert _file_hash(criteria_path) == criteria_hash
    assert inspect_stage_status(workspace, job_dir, stage="parse").stage.status == "succeeded"
    assert inspect_stage_status(workspace, job_dir, stage="confirm").stage.status == "succeeded"


def test_run_pipeline_updates_unedited_generated_typst_sources(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()

    first_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    primary_path = job_dir / "typst" / "cover_letter.typ"
    first_source = primary_path.read_text(encoding="utf-8")

    _set_advert_title(job_dir, "Senior Lecturer in Economics")
    second_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])

    assert first_result.exit_code == 0
    assert second_result.exit_code == 0
    assert primary_path.read_text(encoding="utf-8") != first_source
    assert "Senior Lecturer in Economics" in primary_path.read_text(encoding="utf-8")
    assert not (job_dir / "typst" / "cover_letter.generated.typ").exists()
    manifest = json.loads(
        (job_dir / "typst" / ".canisend-generated.json").read_text(encoding="utf-8")
    )
    assert set(manifest["files"]) == {"cover_letter.typ", "application_package.typ"}
    record = manifest["files"]["cover_letter.typ"]
    assert record["primary_hash"] == _file_hash(primary_path)
    assert record["candidate_hash"] is None
    package_path = job_dir / "typst" / "application_package.typ"
    assert manifest["files"]["application_package.typ"]["primary_hash"] == _file_hash(
        package_path
    )


def test_run_pipeline_marks_existing_gate_report_stale(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    report_path = job_dir / "application_gate_report.json"
    report_path.write_text(
        json.dumps({"schema_version": "1.0.0", "status": "PASS", "input_hashes": {}}),
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        ["run", "--job", str(job_dir), "--no-git-add-materials"],
    )

    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert result.exit_code == 0
    assert report["status"] == "STALE"
    assert report["invalidated_at"].endswith("Z")
    assert report["invalidation_reason"] == "application artifacts were regenerated"


def test_run_pipeline_initializes_matching_legacy_typst_without_candidate(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    typst_dir = job_dir / "typst"
    manifest_path = typst_dir / ".canisend-generated.json"
    primary_path = typst_dir / "cover_letter.typ"
    original_hash = _file_hash(primary_path)
    manifest_path.unlink()

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])

    assert first_result.exit_code == 0
    assert result.exit_code == 0
    assert _file_hash(primary_path) == original_hash
    assert not (typst_dir / "cover_letter.generated.typ").exists()
    assert "WARNING: Preserved edited Typst source" not in result.output
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert manifest["files"]["cover_letter.typ"]["primary_hash"] == original_hash


def test_run_pipeline_preserves_edited_typst_and_writes_candidate(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    primary_path = job_dir / "typst" / "cover_letter.typ"
    primary_generated_hash = _file_hash(primary_path)
    edited_source = primary_path.read_text(encoding="utf-8") + "\n// USER EDIT\n"
    primary_path.write_text(edited_source, encoding="utf-8")
    _set_advert_title(job_dir, "Senior Lecturer in Economics")

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])

    candidate_path = job_dir / "typst" / "cover_letter.generated.typ"
    assert first_result.exit_code == 0
    assert result.exit_code == 0
    assert primary_path.read_text(encoding="utf-8") == edited_source
    assert candidate_path.exists()
    assert "Senior Lecturer in Economics" in candidate_path.read_text(encoding="utf-8")
    assert "WARNING: Preserved edited Typst source" in result.output
    assert str(candidate_path) in result.output
    manifest = json.loads(
        (job_dir / "typst" / ".canisend-generated.json").read_text(encoding="utf-8")
    )
    record = manifest["files"]["cover_letter.typ"]
    assert record["primary_hash"] == primary_generated_hash
    assert record["candidate_hash"] == _file_hash(candidate_path)


def test_run_pipeline_updates_primary_after_generated_candidate_is_adopted(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    primary_path = job_dir / "typst" / "cover_letter.typ"
    primary_path.write_text(
        primary_path.read_text(encoding="utf-8") + "\n// USER EDIT\n",
        encoding="utf-8",
    )
    _set_advert_title(job_dir, "Senior Lecturer in Economics")
    conflict_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    candidate_path = job_dir / "typst" / "cover_letter.generated.typ"
    adopted_candidate = candidate_path.read_text(encoding="utf-8")
    primary_path.write_text(adopted_candidate, encoding="utf-8")
    _set_advert_title(job_dir, "Professor of Applied Economics")

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])

    assert first_result.exit_code == 0
    assert conflict_result.exit_code == 0
    assert result.exit_code == 0
    assert "Professor of Applied Economics" in primary_path.read_text(encoding="utf-8")
    assert primary_path.read_text(encoding="utf-8") != adopted_candidate
    assert not candidate_path.exists()
    assert "WARNING: Preserved edited Typst source" not in result.output
    manifest = json.loads(
        (job_dir / "typst" / ".canisend-generated.json").read_text(encoding="utf-8")
    )
    record = manifest["files"]["cover_letter.typ"]
    assert record["primary_hash"] == _file_hash(primary_path)
    assert record["candidate_hash"] is None


def test_run_pipeline_preserves_separately_edited_candidate_after_adoption(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    primary_path = job_dir / "typst" / "cover_letter.typ"
    primary_path.write_text(
        primary_path.read_text(encoding="utf-8") + "\n// USER EDIT\n",
        encoding="utf-8",
    )
    _set_advert_title(job_dir, "Senior Lecturer in Economics")
    conflict_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    candidate_path = job_dir / "typst" / "cover_letter.generated.typ"
    primary_path.write_text(candidate_path.read_text(encoding="utf-8"), encoding="utf-8")
    edited_candidate = candidate_path.read_text(encoding="utf-8") + "\n// CANDIDATE EDIT\n"
    candidate_path.write_text(edited_candidate, encoding="utf-8")
    _set_advert_title(job_dir, "Professor of Applied Economics")

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])

    assert first_result.exit_code == 0
    assert conflict_result.exit_code == 0
    assert result.exit_code == 0
    assert "Professor of Applied Economics" in primary_path.read_text(encoding="utf-8")
    assert candidate_path.read_text(encoding="utf-8") == edited_candidate
    manifest = json.loads(
        (job_dir / "typst" / ".canisend-generated.json").read_text(encoding="utf-8")
    )
    assert manifest["files"]["cover_letter.typ"]["candidate_hash"] is None


def test_run_pipeline_treats_unknown_manifest_version_conservatively(tmp_path):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])
    typst_dir = job_dir / "typst"
    primary_path = typst_dir / "cover_letter.typ"
    edited_source = primary_path.read_text(encoding="utf-8") + "\n// USER EDIT\n"
    primary_path.write_text(edited_source, encoding="utf-8")
    manifest_path = typst_dir / ".canisend-generated.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["version"] = 999
    manifest["files"]["cover_letter.typ"]["primary_hash"] = _file_hash(primary_path)
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    _set_advert_title(job_dir, "Senior Lecturer in Economics")

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--no-git-add-materials"])

    assert first_result.exit_code == 0
    assert result.exit_code == 0
    assert primary_path.read_text(encoding="utf-8") == edited_source
    assert (typst_dir / "cover_letter.generated.typ").exists()
    assert "WARNING: Preserved edited Typst source" in result.output
    rewritten_manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert rewritten_manifest["version"] == 1


def test_run_git_add_materials_flag_stages_generated_application_materials(tmp_path, monkeypatch):
    job_dir = _write_basic_job(tmp_path)
    calls = []

    def fake_run(command, *, cwd, text, capture_output, check):
        calls.append((command, cwd))
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--git-add-materials"])

    assert result.exit_code == 0
    assert "Added 9 generated application material files to git." in result.output
    assert len(calls) == 1
    command, cwd = calls[0]
    assert command[:4] == ["git", "add", "-f", "--"]
    assert cwd == Path(".").resolve()
    staged = set(command[4:])
    assert staged == {str(job_dir / relative_path) for relative_path in APPLICATION_MATERIAL_RELATIVE_PATHS}


def test_run_skips_git_staging_when_typst_candidate_requires_reconciliation(tmp_path, monkeypatch):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(
        app,
        ["run", "--job", str(job_dir), "--no-git-add-materials"],
    )
    primary_path = job_dir / "typst" / "cover_letter.typ"
    primary_path.write_text(
        primary_path.read_text(encoding="utf-8") + "\n// USER EDIT\n",
        encoding="utf-8",
    )
    _set_advert_title(job_dir, "Senior Lecturer in Economics")
    calls = []

    def fake_run(command, *, cwd, text, capture_output, check):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--git-add-materials"])

    assert first_result.exit_code == 0
    assert result.exit_code == 0
    assert "Skipped git staging" in result.output
    assert (job_dir / "typst" / "cover_letter.generated.typ").exists()
    assert calls == []


def test_run_skips_git_staging_for_preexisting_pending_typst_candidate(tmp_path, monkeypatch):
    job_dir = _write_basic_job(tmp_path)
    runner = CliRunner()
    first_result = runner.invoke(
        app,
        ["run", "--job", str(job_dir), "--no-git-add-materials"],
    )
    candidate_path = job_dir / "typst" / "cover_letter.generated.typ"
    candidate_path.write_text("// separately edited candidate\n", encoding="utf-8")
    calls = []

    def fake_run(command, *, cwd, text, capture_output, check):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--git-add-materials"])

    assert first_result.exit_code == 0
    assert result.exit_code == 0
    assert "Pending Typst candidate" in result.output
    assert "Skipped git staging" in result.output
    assert calls == []


def test_run_interactive_git_add_materials_prompt_can_stage_outputs(tmp_path, monkeypatch):
    job_dir = _write_basic_job(tmp_path)
    calls = []

    def fake_run(command, *, cwd, text, capture_output, check):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr("canisend.cli._stdin_is_interactive", lambda: True)
    monkeypatch.setattr("canisend.cli.typer.confirm", lambda message, default: True)
    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir)])

    assert result.exit_code == 0
    assert "Added 9 generated application material files to git." in result.output
    assert len(calls) == 1


def test_run_skips_git_add_materials_prompt_when_noninteractive(tmp_path, monkeypatch):
    job_dir = _write_basic_job(tmp_path)
    calls = []

    def fake_run(command, *, cwd, text, capture_output, check):
        calls.append(command)
        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

    monkeypatch.setattr("canisend.git_tracking.subprocess.run", fake_run)
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir)])

    assert result.exit_code == 0
    assert calls == []


def test_run_pipeline_writes_material_review_checklist_with_item_level_evidence(tmp_path):
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "Department of Economics",
                "location": "United Kingdom",
                "deadline": "2026-06-15",
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text(
        "# Lecturer in Economics\n\n"
        "Teaching fields: Econometrics\n\n"
        "Essential criteria:\n"
        "- Evidence of teaching excellence\n"
    )
    profile_dir = tmp_path / "profile"
    generated_dir = profile_dir / "generated"
    generated_dir.mkdir(parents=True)
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- [cv-001] `job`: Teaching Assistant for Econometrics\n"
    )
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    review_checklist = (job_dir / "07_material_review_checklist.md").read_text()
    assert "`profile/generated/cv.evidence.md#Teaching/cv-001`" in review_checklist
    assert "Cover Letter Draft" in review_checklist
    assert "CV Tailoring Notes" in review_checklist
    assert "Do not edit `profile/typst/cv.typ` unless the user explicitly asks" in review_checklist
    assert "edits_profile_input: true" in review_checklist


def test_run_pipeline_reads_generated_profile_evidence(tmp_path):
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "",
                "location": "",
                "deadline": "2026-06-15",
                "source_url": "",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text(
        "# Lecturer in Economics\n\n"
        "Essential criteria:\n"
        "- Evidence of teaching excellence\n"
    )
    profile_dir = tmp_path / "profile"
    generated_dir = profile_dir / "generated"
    generated_dir.mkdir(parents=True)
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- `job`: position: Teaching Assistant, institution: University X\n"
    )
    runner = CliRunner()

    result = runner.invoke(app, ["run", "--job", str(job_dir), "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    fit_report = (job_dir / "02_fit_report.md").read_text()
    criteria_checklist = (job_dir / "05_criteria_checklist.md").read_text()
    assert "profile/generated/cv.evidence.md#Teaching" in fit_report
    assert "Evidence of teaching excellence" in criteria_checklist
    assert "coverage" in criteria_checklist.lower()


def test_run_pipeline_can_use_llm_parser_with_command_provider(tmp_path, monkeypatch):
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "",
                "location": "",
                "deadline": "2026-06-15",
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text("Raw advert text")
    prompt_dir = tmp_path / "prompts"
    prompt_dir.mkdir()
    (prompt_dir / "job_parser.md").write_text("Parse this:\n{job_metadata}\n{job_advert}")
    captured_prompt = tmp_path / "captured_prompt.txt"
    fake_parser = tmp_path / "fake_parser.py"
    fake_parser.write_text(
        "import json\n"
        "import pathlib\n"
        "import sys\n"
        f"pathlib.Path({str(captured_prompt)!r}).write_text(sys.stdin.read())\n"
        "print(json.dumps({\n"
        "  'title': 'LLM Parsed Lecturer',\n"
        "  'institution': 'University X',\n"
        "  'department': 'Economics',\n"
        "  'location': 'United Kingdom',\n"
        "  'deadline': '2026-06-15',\n"
        "  'salary': 'unknown',\n"
        "  'contract_type': 'unknown',\n"
        "  'role_type': 'Lecturer',\n"
        "  'research_fields': ['Economics'],\n"
        "  'teaching_fields': ['Econometrics'],\n"
        "  'essential_criteria': [{'criterion': 'PhD', 'source_text': 'PhD'}],\n"
        "  'desirable_criteria': [],\n"
        "  'required_documents': ['CV'],\n"
        "  'application_url': 'https://example.edu/jobs/123',\n"
        "  'unknown_fields': [],\n"
        "  'notes': ''\n"
        "}))\n"
    )
    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", f"{sys.executable} {fake_parser}")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "run",
            "--job",
            str(job_dir),
            "--llm-parser",
            "--prompt-dir",
            str(prompt_dir),
        ],
    )

    assert result.exit_code == 0
    parsed_job = json.loads((job_dir / "parsed_job.json").read_text())
    assert parsed_job["title"] == "LLM Parsed Lecturer"
    prompt = captured_prompt.read_text()
    assert prompt.count("Raw advert text") == 1
    assert "University X" in prompt


def test_run_pipeline_can_use_llm_drafts_with_command_provider(tmp_path, monkeypatch):
    def fail_structured_view_load(*args, **kwargs):
        raise AssertionError("LLM drafts must not be replaced by deterministic Match views")

    monkeypatch.setattr(
        "canisend.pipeline.load_current_structured_match_views",
        fail_structured_view_load,
    )
    job_dir = tmp_path / "jobs" / "2026-06-15_university-x_lecturer-in-economics"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "2026-06-15_university-x_lecturer-in-economics",
                "title": "Lecturer in Economics",
                "institution": "University X",
                "department": "",
                "location": "",
                "deadline": "2026-06-15",
                "source_url": "https://example.edu/jobs/123",
                "status": "advert_imported",
                "created_at": "2026-05-03T23:00:00Z",
                "updated_at": "2026-05-03T23:00:00Z",
                "notes": "",
                "english_variant": "us",
                "writing_style": "direct and warm",
            },
            sort_keys=False,
        )
    )
    (job_dir / "job_advert.md").write_text(
        "# Lecturer in Economics\n\n"
        "Teaching fields: Econometrics\n\n"
        "Essential criteria:\n"
        "- Evidence of teaching excellence\n"
    )
    profile_dir = tmp_path / "profile"
    generated_dir = profile_dir / "generated"
    generated_dir.mkdir(parents=True)
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- `job`: Teaching Assistant for Econometrics\n"
    )
    fake_generator = tmp_path / "fake_generator.py"
    fake_generator.write_text(
        "import sys\n"
        "prompt = sys.stdin.read()\n"
        "citation = '`profile/generated/cv.evidence.md#Teaching`'\n"
        "if '# Profile Matcher' in prompt:\n"
        "    print(f'# Fit Report\\n\\n- Strong teaching fit for econometrics ({citation}).')\n"
        "elif '# Cover Letter Writer' in prompt:\n"
        "    assert 'US English' in prompt, prompt\n"
        "    assert 'direct and warm' in prompt, prompt\n"
        "    print(f'# Cover Letter Draft\\n\\nI can contribute to econometrics teaching ({citation}).')\n"
        "elif '# CV Tailor' in prompt:\n"
        "    print(f'# CV Tailoring Notes\\n\\n- Move teaching evidence higher ({citation}).')\n"
        "elif '# Criteria Checker' in prompt:\n"
        "    print(f'# Criteria Coverage Checklist\\n\\n| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\\n|---|---|---|---|---|\\n| Evidence of teaching excellence | strong | {citation} | low | Keep the evidence visible. |')\n"
        "elif '# Package Builder' in prompt:\n"
        "    print(f'# Final Application Package\\n\\n## Job Information\\n\\n- Title: Lecturer in Economics\\n- Institution: University X\\n- Evidence: {citation}')\n"
        "else:\n"
        "    raise SystemExit('unexpected prompt')\n"
    )
    monkeypatch.setenv("ACADEMIC_PREP_LLM_PROVIDER", "command")
    monkeypatch.setenv("ACADEMIC_PREP_LLM_COMMAND", f"{sys.executable} {fake_generator}")
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "run",
            "--job",
            str(job_dir),
            "--profile-dir",
            str(profile_dir),
            "--llm-drafts",
        ],
    )

    assert result.exit_code == 0
    assert "Strong teaching fit for econometrics" in (job_dir / "02_fit_report.md").read_text()
    assert "I can contribute to econometrics teaching" in (job_dir / "03_cover_letter_draft.md").read_text()
    assert "Move teaching evidence higher" in (job_dir / "04_cv_tailoring_notes.md").read_text()
    assert "strong" in (job_dir / "05_criteria_checklist.md").read_text()
    package_content = json.loads((job_dir / "typst" / "application_package_content.json").read_text())
    assert "Strong teaching fit for econometrics" in package_content["fit_report"]
    assert "I can contribute to econometrics teaching" in package_content["cover_letter"]


def test_run_dry_run_with_llm_parser_does_not_construct_or_call_provider(tmp_path, monkeypatch):
    job_dir = _write_basic_job(tmp_path)
    before = {
        path.relative_to(job_dir).as_posix(): path.read_bytes()
        for path in job_dir.rglob("*")
        if path.is_file()
    }

    def fail_provider(*args, **kwargs):
        raise AssertionError("dry-run must not construct or call a model provider")

    monkeypatch.setattr("canisend.llm.load_llm_config", fail_provider)
    monkeypatch.setattr("canisend.llm.provider_from_config", fail_provider)
    result = CliRunner().invoke(
        app,
        [
            "run",
            "--job",
            str(job_dir),
            "--dry-run",
            "--llm-parser",
            "--llm-drafts",
        ],
    )
    after = {
        path.relative_to(job_dir).as_posix(): path.read_bytes()
        for path in job_dir.rglob("*")
        if path.is_file()
    }

    assert result.exit_code == 0
    assert "LLM-backed (planned; not executed in dry run)" in result.output
    assert "Draft mode: LLM-backed (planned; not executed in dry run)" in result.output
    assert after == before
