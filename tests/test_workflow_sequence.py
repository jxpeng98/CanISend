from __future__ import annotations

from pathlib import Path
import json

import yaml
from typer.testing import CliRunner

from canisend.cli import app
from canisend.bundle_projection import load_artifact_bundle
from canisend.workflow_sequence import SequenceOptions, plan_sequence, run_sequence


def _workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    generated = workspace / "profile" / "generated"
    job.mkdir(parents=True)
    generated.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\nprompt_dir: prompts\n",
        encoding="utf-8",
    )
    (generated / "cv.evidence.md").write_text(
        "# Evidence: CV\n\n"
        "## Education\n\n"
        "- [cv-001] `qualification`: PhD in Economics\n",
        encoding="utf-8",
    )
    (job / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/jobs/example-role",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct",
                "created_at": "2026-07-17T10:00:00Z",
                "updated_at": "2026-07-17T10:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job / "job_advert.md").write_text(
        "# Lecturer in Economics\n\n"
        "Department: Economics\n"
        "Location: London\n"
        "Required documents: CV, Cover letter\n\n"
        "Essential criteria:\n"
        "- PhD in Economics\n\n"
        "Desirable criteria:\n"
        "- Experience teaching econometrics\n",
        encoding="utf-8",
    )
    return workspace, job


def test_sequence_plan_is_read_only_and_exposes_independent_work(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)

    plan = plan_sequence(workspace, job)
    decisions = {(item.stage, item.document_id): item.decision for item in plan.items}

    assert decisions[("intake", None)] == "current"
    assert decisions[("evidence", None)] == "execute"
    assert decisions[("parse", None)] == "execute"
    assert decisions[("confirm", None)] == "blocked"
    assert not (job / "workflow").exists()
    assert not (job / "parsed_job.json").exists()


def test_sequence_runs_all_eligible_stages_then_stops_at_user_decision(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)

    result = run_sequence(workspace, job)

    assert [(item.stage, item.document_id) for item in result.executed] == [
        ("evidence", None),
        ("parse", None),
        ("confirm", None),
        ("match", None),
    ]
    assert result.stop_item is not None
    assert result.stop_item.stage == "decide"
    assert result.stop_item.decision == "blocked"
    assert "input_not_ready:decision_not_initialized" in result.stop_item.reason_codes
    assert (job / "evidence_catalog.json").is_file()
    assert (job / "parsed_job.json").is_file()
    assert (job / "criteria.json").is_file()
    assert (job / "criterion_matches.json").is_file()
    assert not (job / "00_preparation_questions.md").exists()
    assert not (job / "package_bundle.json").exists()


def test_second_sequence_run_is_a_true_stage_and_projection_noop(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    first = run_sequence(workspace, job)
    mtimes = {
        path.name: path.stat().st_mtime_ns
        for path in (
            job / "evidence_catalog.json",
            job / "parsed_job.json",
            job / "criteria.json",
            job / "criterion_matches.json",
        )
    }

    second = run_sequence(workspace, job, options=SequenceOptions())

    assert first.stop_item is not None
    assert second.executed == ()
    assert second.projected_paths == ()
    assert second.stop_item is not None and second.stop_item.stage == "decide"
    assert {
        path.name: path.stat().st_mtime_ns
        for path in (
            job / "evidence_catalog.json",
            job / "parsed_job.json",
            job / "criteria.json",
            job / "criterion_matches.json",
        )
    } == mtimes


def test_run_cli_is_sequence_wrapper_with_agent_response_and_read_only_dry_run(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    runner = CliRunner()

    preview = runner.invoke(
        app,
        [
            "run",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--dry-run",
        ],
    )

    assert preview.exit_code == 0
    assert "Workflow sequence for example-role" in preview.output
    assert "Outputs retained by the compatibility contract" in preview.output
    assert str(job / "03_cover_letter_draft.md") in preview.output
    assert not (job / "workflow").exists()

    executed = runner.invoke(
        app,
        [
            "run",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--no-git-add-materials",
            "--format",
            "json",
        ],
    )
    payload = json.loads(executed.stdout)

    assert executed.exit_code == 0
    assert payload["operation"] == "workflow.sequence_run"
    assert payload["ok"] is True
    assert payload["extensions"]["canisend.sequence.executed_count"] == 4
    assert payload["extensions"]["canisend.sequence.legacy_compatibility"] is True
    assert payload["extensions"]["canisend.sequence.stop_stage"] == "decide"
    assert payload["next_actions"][0]["id"] == "decision.status"
    bundle = load_artifact_bundle(job / "package_bundle.json")
    assert bundle.mode == "legacy_compatibility"
    assert (job / "03_cover_letter_draft.md").is_file()
    metadata = yaml.safe_load((job / "job.yaml").read_text(encoding="utf-8"))
    assert metadata["status"] == "advert_imported"


def test_cli_legacy_projection_is_noop_and_preserves_edited_typst(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    runner = CliRunner()
    arguments = [
        "run",
        "--workspace",
        str(workspace),
        "--job",
        "example-role",
        "--no-git-add-materials",
    ]
    first = runner.invoke(app, arguments)
    primary = job / "typst" / "cover_letter.typ"
    bundle = job / "package_bundle.json"
    journal = job / "workflow" / "projections" / "package.json"
    current_mtimes = {
        path: path.stat().st_mtime_ns
        for path in (primary, bundle, journal, job / "03_cover_letter_draft.md")
    }

    second = runner.invoke(app, arguments)

    assert first.exit_code == 0
    assert second.exit_code == 0
    assert "Executed 0 stage task(s); projected 0 file(s)" in second.output
    assert {
        path: path.stat().st_mtime_ns
        for path in (primary, bundle, journal, job / "03_cover_letter_draft.md")
    } == current_mtimes

    edited = primary.read_text(encoding="utf-8") + "\n// USER EDIT\n"
    primary.write_text(edited, encoding="utf-8")
    advert = job / "job_advert.md"
    advert.write_text(
        advert.read_text(encoding="utf-8").replace(
            "Lecturer in Economics",
            "Senior Lecturer in Economics",
        ),
        encoding="utf-8",
    )
    changed = runner.invoke(app, arguments)
    candidate = job / "typst" / "cover_letter.generated.typ"

    assert changed.exit_code == 0
    assert primary.read_text(encoding="utf-8") == edited
    assert candidate.is_file()
    assert "Senior Lecturer in Economics" in candidate.read_text(encoding="utf-8")
    assert "Pending Typst candidate" in changed.output
