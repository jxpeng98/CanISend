import json
import hashlib
import os
from pathlib import Path

import pytest
import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.ready_check import REQUIRED_SOURCE_FILES, check_application_package


COVER_LETTER_TYPST = "\n".join(
    [
        '#import "@preview/modernpro-coverletter:0.0.8": *',
        "Application letter body.",
        "// CANISEND: section opening",
        "// CANISEND: section research_fit",
        "// CANISEND: section teaching_fit",
        "// CANISEND: section departmental_contribution",
        "// CANISEND: section service_leadership",
        "// CANISEND: section closing",
        "",
    ]
)
APPLICATION_PACKAGE_TYPST = "\n".join(
    [
        '#import "@preview/modernpro-coverletter:0.0.8": *',
        "= Application Package",
        "// CANISEND: section job_information",
        "// CANISEND: section fit_report",
        "// CANISEND: section cover_letter",
        "// CANISEND: section cv_tailoring_notes",
        "// CANISEND: section criteria_checklist",
        "// CANISEND: section remaining_actions",
        "",
    ]
)


def _allow_legacy_package_gate(monkeypatch: pytest.MonkeyPatch) -> None:
    """Keep legacy gate-unit tests focused on their pre-APP-Q5 concern."""

    monkeypatch.setattr(
        "canisend.ready_check._check_aggregate_package_readiness",
        lambda _workspace, _job_dir, _issues: None,
    )


def test_required_source_files_compatibility_constant_is_preserved():
    assert REQUIRED_SOURCE_FILES == ["job.yaml", "job_advert.md"]


def test_check_package_reports_missing_required_outputs(tmp_path):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text("status: advert_imported\n", encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "Package check failed" in result.output
    assert "missing parsed_job.json" in result.output
    assert "missing 00_preparation_questions.md" in result.output
    assert "missing 07_material_review_checklist.md" in result.output


def test_check_package_write_report_does_not_create_a_missing_job_directory(tmp_path):
    workspace = tmp_path / "workspace"
    missing_job = workspace / "jobs" / "missing-role"
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            "missing-role",
            "--write-report",
        ],
    )

    assert result.exit_code == 1
    assert "job directory does not exist" in result.output
    assert not missing_job.exists()


def test_check_package_reports_unknown_citations_and_placeholders(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    (job_dir / "03_cover_letter_draft.md").write_text(
        "# Cover Letter Draft\n\n"
        "I can support teaching (`profile/generated/other.evidence.md#Teaching`).\n\n"
        "Yours sincerely,\n\n"
        "[Applicant name]\n",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "unknown evidence citation" in result.output
    assert "placeholder [Applicant name]" in result.output


def test_check_package_legacy_minimal_package_fails_closed_without_aggregate_review(
    tmp_path,
):
    workspace, job_dir = _complete_workspace(tmp_path)
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "Package check failed" in result.output
    assert "current aggregate package Review is missing or unsafe" in result.output
    assert not (job_dir / "application_gate_report.json").exists()


def test_check_package_requires_typst_sources_and_stable_markers(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    runner = CliRunner()
    cover_letter = job_dir / "typst" / "cover_letter.typ"
    cover_letter.unlink()

    missing_source = runner.invoke(
        app,
        ["check-package", "--workspace", str(workspace), "--job", "example-role"],
    )

    assert missing_source.exit_code == 1
    assert "missing typst/cover_letter.typ" in missing_source.output

    cover_letter.write_text(COVER_LETTER_TYPST, encoding="utf-8")
    application_package = job_dir / "typst" / "application_package.typ"
    application_package.write_text(
        APPLICATION_PACKAGE_TYPST.replace("// CANISEND: section criteria_checklist\n", ""),
        encoding="utf-8",
    )

    missing_marker = runner.invoke(
        app,
        ["check-package", "--workspace", str(workspace), "--job", "example-role"],
    )

    assert missing_marker.exit_code == 1
    assert "missing stable section marker: // CANISEND: section criteria_checklist" in missing_marker.output


def test_check_package_rejects_incomplete_parsed_job(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    (job_dir / "parsed_job.json").write_text(
        json.dumps({"title": "Lecturer", "institution": "Example University"}) + "\n",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "invalid parsed job: missing required field: department" in result.output


@pytest.mark.parametrize(
    "advert_text",
    [
        (
            "# Job Advert Pending Import\n\n"
            "The full advert still needs manual paste, PDF import, or explicit fetch before final parsing.\n"
        ),
        (
            "# Lecturer\n\n"
            "> RSS lead only. Paste the full advert manually below before final generation.\n\n"
            "## Full Advert\n\n"
            "Paste the full advert manually here before relying on parsed criteria or generated drafts.\n"
        ),
    ],
)
def test_check_package_rejects_pending_or_lead_advert_stub(tmp_path, advert_text):
    workspace, job_dir = _complete_workspace(tmp_path)
    (job_dir / "job_advert.md").write_text(advert_text, encoding="utf-8")
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "job advert is still a lead or pending-import stub" in result.output


def test_check_package_accepts_lead_record_after_full_advert_is_added(
    tmp_path,
    monkeypatch,
):
    workspace, job_dir = _complete_workspace(tmp_path)
    _allow_legacy_package_gate(monkeypatch)
    (job_dir / "job_advert.md").write_text(
        "# Lecturer\n\n"
        "> RSS lead only. Paste the full advert manually below before final generation.\n\n"
        "## Full Advert\n\n"
        "The successful candidate will teach econometrics and publish applied research.\n",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 0
    assert "Package check passed" in result.output


def test_check_package_rejects_explicit_material_review_blocker(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    (job_dir / "07_material_review_checklist.md").write_text(
        "# Material Review Checklist\n\n| Criterion | HR Status |\n|---|---|\n| Teaching | BLOCKER |\n",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(app, ["check-package", "--workspace", str(workspace), "--job", "example-role"])

    assert result.exit_code == 1
    assert "material review contains an explicit BLOCKER" in result.output


@pytest.mark.parametrize(
    ("metadata_text", "expected_message"),
    [
        ("title: [\n", "invalid job metadata YAML"),
        ("- status\n- packaged\n", "job metadata must be a mapping"),
        (
            "title: Lecturer\ninstitution: Example University\n"
            "source_url: https://example.edu/jobs/lecturer\nstatus: packaged\n",
            "job metadata missing required field: deadline",
        ),
        (
            "title: Lecturer\ninstitution: Example University\ndeadline: 2026-08-01\n"
            "source_url: https://example.edu/jobs/lecturer\nstatus: advert_imported\n",
            "job metadata status must be packaged",
        ),
    ],
)
def test_check_package_rejects_invalid_or_incomplete_job_metadata(
    tmp_path,
    metadata_text,
    expected_message,
):
    workspace, job_dir = _complete_workspace(tmp_path)
    (job_dir / "job.yaml").write_text(metadata_text, encoding="utf-8")

    result = check_application_package(job_dir, workspace / "profile")

    assert not result.ok
    assert any(issue.gate == "APP-Q1" and expected_message in issue.message for issue in result.issues)


def test_check_package_rejects_job_metadata_that_disagrees_with_parsed_job(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))
    metadata["title"] = "Professor"
    metadata["source_url"] = "https://example.edu/jobs/different"
    (job_dir / "job.yaml").write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")

    result = check_application_package(job_dir, workspace / "profile")

    messages = [issue.message for issue in result.issues if issue.gate == "APP-Q1"]
    assert "job metadata title does not match parsed_job.json title" in messages
    assert "job metadata source_url does not match parsed_job.json application_url" in messages


def test_check_package_reports_missing_and_stale_generated_evidence(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    profile_dir = workspace / "profile"
    source_path = profile_dir / "typst" / "cv.typ"
    evidence_path = profile_dir / "generated" / "cv.evidence.md"
    evidence_path.unlink()

    missing = check_application_package(job_dir, profile_dir)

    assert any(
        issue.gate == "APP-Q2" and "missing generated evidence for profile source: cv" in issue.message
        for issue in missing.issues
    )

    evidence_path.write_text("# Evidence: cv\n", encoding="utf-8")
    os.utime(evidence_path, ns=(100, 100))
    os.utime(source_path, ns=(200, 200))

    stale = check_application_package(job_dir, profile_dir)

    assert any(
        issue.gate == "APP-Q2" and "generated evidence is stale for profile source: cv" in issue.message
        for issue in stale.issues
    )


def test_check_package_requires_modernpro_import_body_and_reconciled_candidates(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    profile_dir = workspace / "profile"
    cover_path = job_dir / "typst" / "cover_letter.typ"
    cover_path.write_text(
        COVER_LETTER_TYPST.replace('#import "@preview/modernpro-coverletter:0.0.8": *\n', ""),
        encoding="utf-8",
    )

    missing_import = check_application_package(job_dir, profile_dir)

    assert any(
        issue.gate == "APP-Q4" and issue.message == "missing modernpro Typst import"
        for issue in missing_import.issues
    )

    cover_path.write_text(
        "\n".join(
            [
                '#import "@preview/modernpro-coverletter:0.0.8": *',
                *[line for line in COVER_LETTER_TYPST.splitlines() if line.startswith("//")],
                "",
            ]
        ),
        encoding="utf-8",
    )
    no_body = check_application_package(job_dir, profile_dir)

    assert any(
        issue.gate == "APP-Q4" and "non-comment body line" in issue.message
        for issue in no_body.issues
    )

    cover_path.write_text(COVER_LETTER_TYPST, encoding="utf-8")
    candidate_path = job_dir / "typst" / "cover_letter.generated.typ"
    candidate_path.write_text(COVER_LETTER_TYPST, encoding="utf-8")
    candidate = check_application_package(job_dir, profile_dir)

    assert any(
        issue.gate == "APP-Q4"
        and issue.path == "typst/cover_letter.generated.typ"
        and "must be reviewed and reconciled" in issue.message
        for issue in candidate.issues
    )


def test_check_package_assigns_metadata_evidence_artifact_and_typst_gates(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    profile_dir = workspace / "profile"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text(encoding="utf-8"))
    metadata["title"] = "Professor"
    (job_dir / "job.yaml").write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")
    evidence_path = profile_dir / "generated" / "cv.evidence.md"
    source_path = profile_dir / "typst" / "cv.typ"
    os.utime(evidence_path, ns=(100, 100))
    os.utime(source_path, ns=(200, 200))
    (job_dir / "typst" / "cover_letter_content.json").unlink()
    (job_dir / "typst" / "cover_letter.generated.typ").write_text(
        COVER_LETTER_TYPST,
        encoding="utf-8",
    )

    result = check_application_package(job_dir, profile_dir)

    assert any(issue.gate == "APP-Q1" and issue.path == "job.yaml" for issue in result.issues)
    assert any(issue.gate == "APP-Q2" and "stale" in issue.message for issue in result.issues)
    assert any(
        issue.gate == "APP-Q3" and issue.path == "typst/cover_letter_content.json"
        for issue in result.issues
    )
    assert any(
        issue.gate == "APP-Q4" and issue.path == "typst/cover_letter.generated.typ"
        for issue in result.issues
    )


def test_check_package_writes_machine_readable_gate_report_only_when_requested(
    tmp_path,
    monkeypatch,
):
    workspace, job_dir = _complete_workspace(tmp_path)
    _allow_legacy_package_gate(monkeypatch)
    runner = CliRunner()

    passed = runner.invoke(
        app,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--write-report",
        ],
    )

    report_path = job_dir / "application_gate_report.json"
    assert passed.exit_code == 0
    assert "Package check passed" in passed.output
    assert "Application gate report:" in passed.output
    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["schema_version"] == "1.0.0"
    assert report["generated_at"].endswith("Z")
    assert report["status"] == "PASS"
    assert report["issues"] == []
    expected_inputs = {
        "job/job.yaml": job_dir / "job.yaml",
        "job/job_advert.md": job_dir / "job_advert.md",
        "job/parsed_job.json": job_dir / "parsed_job.json",
        "job/typst/cover_letter.typ": job_dir / "typst" / "cover_letter.typ",
        "profile/profile.yaml": workspace / "profile" / "profile.yaml",
        "profile/typst/cv.typ": workspace / "profile" / "typst" / "cv.typ",
        "profile/generated/cv.evidence.md": workspace / "profile" / "generated" / "cv.evidence.md",
    }
    for label, path in expected_inputs.items():
        assert report["input_hashes"][label] == hashlib.sha256(path.read_bytes()).hexdigest()
    assert all(not label.startswith("/") for label in report["input_hashes"])
    assert str(tmp_path) not in json.dumps(report["input_hashes"])
    assert all(len(value) == 64 for value in report["input_hashes"].values())

    (job_dir / "07_material_review_checklist.md").write_text(
        "# Material Review Checklist\n\n- BLOCKER: verify an essential criterion.\n",
        encoding="utf-8",
    )
    failed = runner.invoke(
        app,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--write-report",
        ],
    )

    assert failed.exit_code == 1
    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["status"] == "FAIL"
    assert {
        "gate": "APP-Q4",
        "path": "07_material_review_checklist.md",
        "message": "material review contains an explicit BLOCKER",
    } in report["issues"]


def test_check_package_json_pass_is_read_only_by_default(tmp_path, monkeypatch):
    workspace, job_dir = _complete_workspace(tmp_path)
    _allow_legacy_package_gate(monkeypatch)

    result = CliRunner().invoke(
        app,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 0
    assert payload["operation"] == "package.check"
    assert payload["ok"] is True
    assert payload["gate"] == {"status": "PASS", "issue_count": 0, "report_path": None}
    assert [action["id"] for action in payload["next_actions"]] == ["package.review"]
    assert not (job_dir / "application_gate_report.json").exists()
    assert str(workspace) not in result.stdout


def test_check_package_json_fail_is_completed_gate_with_safe_blockers(tmp_path):
    workspace, job_dir = _complete_workspace(tmp_path)
    private_text = "PRIVATE REVIEW BODY"
    (job_dir / "07_material_review_checklist.md").write_text(
        f"# Review\n\n{private_text}\n\n- BLOCKER: resolve criterion.\n",
        encoding="utf-8",
    )

    result = CliRunner().invoke(
        app,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 1
    assert payload["ok"] is True
    assert payload["error"] is None
    assert payload["gate"]["status"] == "FAIL"
    assert payload["gate"]["issue_count"] >= 1
    assert payload["blockers"]
    assert [action["id"] for action in payload["next_actions"]] == ["package.resolve_blockers"]
    assert private_text not in result.stdout
    assert not (job_dir / "application_gate_report.json").exists()


def test_check_package_json_writes_report_only_when_explicit(tmp_path, monkeypatch):
    workspace, job_dir = _complete_workspace(tmp_path)
    _allow_legacy_package_gate(monkeypatch)

    result = CliRunner().invoke(
        app,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--write-report",
            "--format",
            "json",
        ],
    )
    payload = json.loads(result.stdout)

    assert result.exit_code == 0
    assert (job_dir / "application_gate_report.json").exists()
    assert payload["gate"]["report_path"] == "jobs/example-role/application_gate_report.json"
    assert any(
        artifact["kind"] == "application_gate_report" and artifact["exists"]
        for artifact in payload["artifacts"]
    )


def _complete_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    profile_dir = workspace / "profile"
    generated_dir = profile_dir / "generated"
    job_dir = workspace / "jobs" / "example-role"
    generated_dir.mkdir(parents=True)
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        yaml.safe_dump(
            {
                "profile_dir": "profile",
                "jobs_dir": "jobs",
                "job_leads_dir": "job_leads",
                "prompt_dir": "prompts",
                "template_dir": "templates",
                "schema_dir": "schemas",
                "agent_skills_dir": "agent-skills",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (profile_dir / "typst").mkdir()
    (profile_dir / "typst" / "cv.typ").write_text("#section(\"Teaching\")\n", encoding="utf-8")
    (profile_dir / "profile.yaml").write_text(
        yaml.safe_dump(
            {
                "profile_mode": "typst",
                "sources": {"cv": "typst/cv.typ"},
                "generated": {"cv_evidence": "generated/cv.evidence.md"},
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (generated_dir / "cv.evidence.md").write_text(
        "# Evidence: cv\n\n"
        "## Teaching\n\n"
        "- [cv-001] `job`: Teaching Assistant for Econometrics\n",
        encoding="utf-8",
    )
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "title": "Lecturer",
                "institution": "Example University",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/jobs/lecturer",
                "status": "packaged",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text(
        "# Advert\n\nThe successful candidate will teach econometrics.\n",
        encoding="utf-8",
    )
    (job_dir / "parsed_job.json").write_text(
        json.dumps(
            {
                "title": "Lecturer",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "salary": "unknown",
                "contract_type": "permanent",
                "role_type": "academic",
                "research_fields": ["econometrics"],
                "teaching_fields": ["econometrics"],
                "essential_criteria": [
                    {
                        "criterion": "Ability to teach econometrics",
                        "source_text": "Ability to teach econometrics",
                    }
                ],
                "desirable_criteria": [],
                "required_documents": ["CV", "cover letter"],
                "application_url": "https://example.edu/jobs/lecturer",
                "unknown_fields": [],
                "notes": "",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    citation = "`profile/generated/cv.evidence.md#Teaching/cv-001`"
    markdown_files = {
        "00_preparation_questions.md": "# Preparation Questions\n\n- English variant: us\n- Writing style: direct\n",
        "01_job_summary.md": "# Job Summary\n\n- Title: Lecturer\n",
        "02_fit_report.md": f"# Fit Report\n\nTeaching evidence {citation}.\n",
        "03_cover_letter_draft.md": f"# Cover Letter Draft\n\nI can support teaching ({citation}).\n",
        "04_cv_tailoring_notes.md": f"# CV Tailoring Notes\n\n- Move teaching higher ({citation}).\n",
        "05_criteria_checklist.md": f"# Criteria Coverage Checklist\n\nTeaching: strong ({citation}).\n",
        "06_final_application_package.md": f"# Final Application Package\n\nTeaching fit {citation}.\n",
        "07_material_review_checklist.md": f"# Material Review Checklist\n\n- Citation: {citation}\n",
    }
    for filename, contents in markdown_files.items():
        (job_dir / filename).write_text(contents, encoding="utf-8")
    typst_dir = job_dir / "typst"
    typst_dir.mkdir()
    (typst_dir / "cover_letter_content.json").write_text("{}\n", encoding="utf-8")
    (typst_dir / "application_package_content.json").write_text("{}\n", encoding="utf-8")
    (typst_dir / "cover_letter.typ").write_text(COVER_LETTER_TYPST, encoding="utf-8")
    (typst_dir / "application_package.typ").write_text(APPLICATION_PACKAGE_TYPST, encoding="utf-8")
    return workspace, job_dir
