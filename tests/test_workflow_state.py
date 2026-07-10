from __future__ import annotations

import json
import os
from pathlib import Path

import yaml

from canisend.workflow_state import derive_workflow_snapshot


def test_snapshot_blocks_missing_job_directory(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    workspace.mkdir()

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/missing-role"))

    assert snapshot.error_code == "job.not_found"
    assert snapshot.workflow.phase == "unknown"
    assert snapshot.workflow.readiness == "blocked"
    assert snapshot.job is None


def test_snapshot_blocks_missing_or_invalid_job_metadata(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    missing = workspace / "jobs" / "missing-metadata"
    missing.mkdir(parents=True)
    invalid = workspace / "jobs" / "invalid-metadata"
    invalid.mkdir()
    (invalid / "job.yaml").write_text("- not\n- a\n- mapping\n", encoding="utf-8")

    missing_snapshot = derive_workflow_snapshot(workspace, Path("jobs/missing-metadata"))
    invalid_snapshot = derive_workflow_snapshot(workspace, Path("jobs/invalid-metadata"))

    assert missing_snapshot.error_code == "job.invalid_metadata"
    assert invalid_snapshot.error_code == "job.invalid_metadata"
    assert missing_snapshot.workflow.readiness == "blocked"
    assert invalid_snapshot.workflow.readiness == "blocked"


def test_snapshot_blocks_lead_only_advert(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(tmp_path, status="lead_imported")
    (job_dir / "job_advert.md").write_text(
        "# Lecturer\n\n> Feed lead only (RSS/Atom).\n\n"
        "## Full Advert\n\nPaste the full advert manually here before relying on it.\n",
        encoding="utf-8",
    )

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "intake"
    assert snapshot.workflow.readiness == "blocked"
    assert "job_advert.md" in snapshot.missing_fields
    assert snapshot.next_actions[0].id == "job.import_advert"


def test_snapshot_blocks_new_job_with_empty_advert(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(tmp_path, status="new")
    (job_dir / "job_advert.md").write_text("", encoding="utf-8")

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "intake"
    assert snapshot.workflow.readiness == "blocked"
    assert snapshot.next_actions[0].id == "job.import_advert"


def test_snapshot_moves_to_evidence_when_full_advert_is_available(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(tmp_path, status="advert_imported")
    (job_dir / "job_advert.md").write_text("# Full role\n\nEssential: PhD.\n", encoding="utf-8")
    _write_profile(workspace, with_evidence=False)

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "evidence"
    assert snapshot.workflow.readiness == "action_required"
    assert {consent.id for consent in snapshot.required_consents} >= {
        "read-full-job-advert",
        "read-profile-sources",
    }
    evidence_action = next(action for action in snapshot.next_actions if action.id == "profile.extract_evidence")
    assert evidence_action.requires_consent is True
    assert evidence_action.consent_ids == ["read-profile-sources"]


def test_snapshot_returns_parse_as_next_phase_after_current_evidence(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(tmp_path, status="advert_imported")
    (job_dir / "job_advert.md").write_text("# Full role\n\nEssential: PhD.\n", encoding="utf-8")
    _write_profile(workspace, with_evidence=True)

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "parse"
    assert snapshot.workflow.readiness == "ready_for_next_stage"
    parse_action = next(action for action in snapshot.next_actions if action.id == "job.parse")
    assert parse_action.requires_consent is True
    assert parse_action.consent_ids == ["read-full-job-advert"]


def test_snapshot_requires_preferences_before_package_generation(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(
        tmp_path,
        status="advert_imported",
        deadline="unknown",
        english_variant="needs_confirmation",
        writing_style="needs_confirmation",
    )
    (job_dir / "job_advert.md").write_text("# Full role\n\nEssential: PhD.\n", encoding="utf-8")
    (job_dir / "parsed_job.json").write_text("{}\n", encoding="utf-8")
    _write_profile(workspace, with_evidence=True)

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "package"
    assert snapshot.workflow.readiness == "action_required"
    assert set(snapshot.missing_fields) >= {
        "job.deadline",
        "job.english_variant",
        "job.writing_style",
    }
    assert any(action.id == "job.confirm_preferences" for action in snapshot.next_actions)


def test_snapshot_requires_package_regeneration_when_review_checklist_is_missing(tmp_path: Path) -> None:
    workspace, job_dir = _complete_packaged_job(tmp_path)
    (job_dir / "07_material_review_checklist.md").unlink()

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "package"
    assert snapshot.workflow.readiness == "action_required"
    assert any(action.id == "package.generate" for action in snapshot.next_actions)


def test_snapshot_marks_pending_typst_candidate_as_review_required(tmp_path: Path) -> None:
    workspace, job_dir = _complete_packaged_job(tmp_path)
    candidate = job_dir / "typst" / "cover_letter.generated.typ"
    candidate.parent.mkdir(parents=True, exist_ok=True)
    candidate.write_text("candidate body\n", encoding="utf-8")

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert snapshot.workflow.phase == "render"
    assert snapshot.workflow.readiness == "review_required"
    assert any(action.id == "typst.review_candidate" for action in snapshot.next_actions)
    assert any(artifact.kind == "typst_candidate" for artifact in snapshot.artifacts)


def test_snapshot_derives_pass_fail_and_stale_gate_states(tmp_path: Path) -> None:
    workspace, job_dir = _complete_packaged_job(tmp_path)
    report_path = job_dir / "application_gate_report.json"

    expected = {
        "PASS": ("render", "review_required"),
        "FAIL": ("verify", "blocked"),
        "STALE": ("verify", "action_required"),
    }
    for status, (phase, readiness) in expected.items():
        report_path.write_text(
            json.dumps(
                {
                    "schema_version": "1.0.0",
                    "status": status,
                    "issues": [] if status == "PASS" else [{"path": "example", "message": "review"}],
                }
            ),
            encoding="utf-8",
        )

        snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

        assert snapshot.gate is not None
        assert snapshot.gate.status == status
        assert snapshot.workflow.phase == phase
        assert snapshot.workflow.readiness == readiness


def test_snapshot_marks_external_configured_path_without_leaking_name(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(tmp_path, status="advert_imported")
    (job_dir / "job_advert.md").write_text("# Full role\n\nEssential: PhD.\n", encoding="utf-8")
    private_source = tmp_path / "Peng_Private_CV.typ"
    private_source.write_text("PRIVATE PROFILE BODY", encoding="utf-8")
    _write_profile(workspace, with_evidence=True, source_path=private_source)

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))
    payload = json.dumps(
        [artifact.model_dump(mode="json") for artifact in snapshot.artifacts],
        sort_keys=True,
    )

    profile_source = next(artifact for artifact in snapshot.artifacts if artifact.kind == "profile_source")
    assert profile_source.path is None
    assert profile_source.opaque_id is not None
    assert "Peng_Private_CV" not in payload
    assert str(tmp_path) not in payload


def test_snapshot_never_includes_job_advert_body(tmp_path: Path) -> None:
    workspace, job_dir = _workspace_with_job(tmp_path, status="advert_imported")
    secret_body = "PRIVATE ADVERT BODY SHOULD NEVER APPEAR"
    (job_dir / "job_advert.md").write_text(f"# Full role\n\n{secret_body}\n", encoding="utf-8")
    _write_profile(workspace, with_evidence=True)

    snapshot = derive_workflow_snapshot(workspace, Path("jobs/example-role"))

    assert secret_body not in repr(snapshot)
    assert secret_body not in json.dumps(
        [artifact.model_dump(mode="json") for artifact in snapshot.artifacts],
        sort_keys=True,
    )


def _workspace_with_job(
    tmp_path: Path,
    *,
    status: str,
    deadline: str = "2026-08-01",
    english_variant: str = "uk",
    writing_style: str = "direct and evidence-led",
) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "deadline": deadline,
                "source_url": "https://example.edu/jobs/1?private=redacted",
                "status": status,
                "english_variant": english_variant,
                "writing_style": writing_style,
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job_dir / "job_advert.md").write_text("", encoding="utf-8")
    return workspace, job_dir


def _write_profile(
    workspace: Path,
    *,
    with_evidence: bool,
    source_path: Path | None = None,
) -> None:
    profile_dir = workspace / "profile"
    source = source_path or profile_dir / "typst" / "cv.typ"
    source.parent.mkdir(parents=True, exist_ok=True)
    if not source.exists():
        source.write_text("profile source\n", encoding="utf-8")
    evidence = profile_dir / "generated" / "cv.evidence.md"
    if with_evidence:
        evidence.parent.mkdir(parents=True, exist_ok=True)
        evidence.write_text("generated evidence\n", encoding="utf-8")
        os.utime(source, (100, 100))
        os.utime(evidence, (200, 200))
    relative_or_absolute_source = (
        source.relative_to(profile_dir).as_posix()
        if source.is_relative_to(profile_dir)
        else str(source)
    )
    (profile_dir / "profile.yaml").write_text(
        yaml.safe_dump(
            {
                "sources": {"cv": relative_or_absolute_source},
                "generated": {"cv_evidence": "generated/cv.evidence.md"},
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )


def _complete_packaged_job(tmp_path: Path) -> tuple[Path, Path]:
    workspace, job_dir = _workspace_with_job(tmp_path, status="packaged")
    (job_dir / "job_advert.md").write_text("# Full role\n\nEssential: PhD.\n", encoding="utf-8")
    (job_dir / "parsed_job.json").write_text("{}\n", encoding="utf-8")
    (job_dir / "07_material_review_checklist.md").write_text("# Reviewed\n", encoding="utf-8")
    _write_profile(workspace, with_evidence=True)
    return workspace, job_dir
