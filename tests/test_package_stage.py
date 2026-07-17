from __future__ import annotations

import json
from pathlib import Path

import pytest

from canisend.bundle_projection import load_artifact_bundle, project_artifact_bundle
from canisend.stage_runtime import StageRuntimeError, inspect_stage_status, run_deterministic_stage
from canisend.stages.render_stage import (
    build_render_bundle_candidate,
    render_input_fingerprint,
    validate_render_bundle_candidate,
)
from canisend.stages.verify_stage import ApplicationGateReportV1
from canisend.user_mutations import (
    SetPackageFindingDispositionPatch,
    apply_user_patch,
    initialize_package_review_dispositions,
    inspect_package_review_dispositions,
)
from tests.test_package_review_disposition_mutations import _current_reviewable_package


def _review_package(workspace: Path, job: Path, review: dict[str, object]) -> None:
    initialized = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    snapshot = initialized.snapshot
    for finding in review["findings"]:
        updated = apply_user_patch(
            workspace,
            job,
            SetPackageFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=snapshot.sha256,
            expected_revision=snapshot.revision,
            consent_confirmed=True,
        )
        snapshot = updated.snapshot
    readiness = inspect_package_review_dispositions(workspace, job)
    assert readiness.readiness is not None
    assert readiness.readiness.state == "reviewed"


def test_package_stage_promotes_one_bundle_then_projects_legacy_files(
    tmp_path: Path,
) -> None:
    workspace, job, review = _current_reviewable_package(tmp_path)
    _review_package(workspace, job, review)
    metadata_before = (job / "job.yaml").read_bytes()

    outcome = run_deterministic_stage(workspace, job, stage="package")
    bundle = load_artifact_bundle(job / "package_bundle.json")

    assert outcome.cache_hit is False
    assert bundle.stage == "package"
    assert bundle.mode == "guarded"
    assert bundle.input_fingerprint == outcome.manifest.input_fingerprint
    paths = {entry.path for entry in bundle.entries}
    assert "03_cover_letter_draft.md" in paths
    assert "08_research_statement.md" in paths
    assert "typst/cover_letter.typ" in paths
    assert "typst/research_statement.typ" in paths
    assert not (job / "03_cover_letter_draft.md").exists()

    journal = project_artifact_bundle(job, bundle)

    assert journal.stage == "package"
    assert (job / "03_cover_letter_draft.md").is_file()
    assert (job / "08_research_statement.md").is_file()
    assert (job / "typst" / "cover_letter.typ").is_file()
    assert (job / "typst" / "research_statement.typ").is_file()
    assert (job / "job.yaml").read_bytes() == metadata_before
    assert inspect_stage_status(workspace, job, stage="package").stage.status == "succeeded"


def test_package_stage_cache_and_projection_are_true_noops(tmp_path: Path) -> None:
    workspace, job, review = _current_reviewable_package(tmp_path)
    _review_package(workspace, job, review)
    first = run_deterministic_stage(workspace, job, stage="package")
    bundle = load_artifact_bundle(first.authoritative_path)
    project_artifact_bundle(job, bundle)
    target = job / "03_cover_letter_draft.md"
    bundle_mtime = first.authoritative_path.stat().st_mtime_ns
    target_mtime = target.stat().st_mtime_ns

    cached = run_deterministic_stage(workspace, job, stage="package")
    journal = project_artifact_bundle(job, bundle)

    assert cached.cache_hit is True
    assert first.authoritative_path.stat().st_mtime_ns == bundle_mtime
    assert target.stat().st_mtime_ns == target_mtime
    assert all(entry.outcome == "unchanged" for entry in journal.entries)


def test_verify_stage_rederives_and_promotes_one_gate_report(tmp_path: Path) -> None:
    workspace, job, review = _current_reviewable_package(tmp_path)
    _review_package(workspace, job, review)
    package = run_deterministic_stage(workspace, job, stage="package")
    project_artifact_bundle(job, load_artifact_bundle(package.authoritative_path))

    verified = run_deterministic_stage(workspace, job, stage="verify")
    report = json.loads(verified.authoritative_path.read_text())

    assert verified.cache_hit is False
    assert report["schema_version"] == "1.0.0"
    assert report["status"] == "FAIL"
    assert report["issues"][0]["gate"] == "APP-Q4"
    assert report["input_fingerprint"] == verified.manifest.input_fingerprint
    assert len(report["package_bundle_sha256"]) == 64
    assert len(report["projection_journal_sha256"]) == 64
    assert inspect_stage_status(workspace, job, stage="verify").stage.status == "succeeded"

    cached = run_deterministic_stage(workspace, job, stage="verify")
    assert cached.cache_hit is True

    render = inspect_stage_status(workspace, job, stage="render")
    assert render.stage.status == "blocked"
    assert render.reasons == ("input_not_ready:verify_failed",)
    with pytest.raises(StageRuntimeError) as captured:
        run_deterministic_stage(workspace, job, stage="render")
    assert captured.value.code == "stage.dependency_not_current"


def test_render_builder_compiles_to_bundle_before_pdf_projection(tmp_path: Path) -> None:
    job = tmp_path / "job"
    typst = job / "typst"
    typst.mkdir(parents=True)
    (typst / "cover_letter.typ").write_text("#text[Cover letter]\n")
    report = ApplicationGateReportV1(
        job_id=job.name,
        package_bundle_sha256="a" * 64,
        projection_journal_sha256="b" * 64,
        status="PASS",
        input_hashes={},
        issues=(),
        generated_at="2026-07-17T10:00:00Z",
        input_fingerprint="c" * 64,
    )
    (job / "application_gate_report.json").write_text(
        json.dumps(report.model_dump(mode="json")) + "\n"
    )
    fake_typst = tmp_path / "fake-typst.py"
    fake_typst.write_text(
        "#!/usr/bin/env python3\n"
        "import pathlib, sys\n"
        "pathlib.Path(sys.argv[3]).write_bytes(b'%PDF-1.7 fake')\n"
    )
    fake_typst.chmod(0o755)
    fingerprint = render_input_fingerprint(job)

    bundle = build_render_bundle_candidate(
        job,
        input_fingerprint=fingerprint,
        typst_bin=str(fake_typst),
    )
    validated = validate_render_bundle_candidate(
        bundle.model_dump(mode="json"),
        job_dir=job,
        input_fingerprint=fingerprint,
    )

    assert validated.stage == "render"
    assert validated.entries[0].path == "pdf/cover_letter.pdf"
    assert not (job / "pdf" / "cover_letter.pdf").exists()
    project_artifact_bundle(job, validated)
    assert (job / "pdf" / "cover_letter.pdf").read_bytes().startswith(b"%PDF-")
