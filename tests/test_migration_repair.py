from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator
import pytest
from typer.testing import CliRunner
import yaml

from canisend.bundle_models import ArtifactBundleV1, BundleEntryV1
from canisend.bundle_projection import (
    canonical_bundle_bytes,
    inspect_artifact_projection,
    project_artifact_bundle,
)
from canisend.cli import app
from canisend.migration import (
    MIGRATION_PLAN_PATH,
    MIGRATION_RECEIPT_PATH,
    MigrationError,
    apply_migration,
    inspect_migration,
    rollback_migration,
)
from canisend.recovery_models import (
    MigrationPlanV1,
    MigrationReceiptV1,
    MigrationRollbackReceiptV1,
    RepairReceiptV1,
)
from canisend.stage_models import WorkflowStateV1
from canisend.stage_runtime import inspect_stage_status, run_deterministic_stage
from canisend.stage_store import read_json_object, sha256_bytes
from canisend.workflow_repair import (
    inspect_projection_repair,
    inspect_state_repair,
    repair_projection,
    repair_state,
)


NOW = "2026-07-17T10:00:00Z"


@pytest.mark.parametrize(
    ("schema_name", "model"),
    [
        ("migration-plan.schema.json", MigrationPlanV1),
        ("migration-receipt.schema.json", MigrationReceiptV1),
        ("migration-rollback-receipt.schema.json", MigrationRollbackReceiptV1),
        ("repair-receipt.schema.json", RepairReceiptV1),
    ],
)
def test_recovery_schemas_match_strict_models(schema_name: str, model: type) -> None:
    stored = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))
    expected = model.model_json_schema(mode="validation")
    expected.update(
        {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": f"https://github.com/jxpeng98/CanISend/schemas/{schema_name}",
        }
    )

    Draft202012Validator.check_schema(stored)
    assert stored == expected


def _workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    generated = workspace / "profile" / "generated"
    job.mkdir(parents=True)
    generated.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (generated / "cv.evidence.md").write_text(
        "# Evidence: CV\n\n## Education\n\n"
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
                "created_at": NOW,
                "updated_at": NOW,
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    (job / "job_advert.md").write_text(
        "# Lecturer in Economics\n\nEssential criteria:\n- PhD in Economics\n",
        encoding="utf-8",
    )
    return workspace, job


def _package_bundle(job: Path) -> ArtifactBundleV1:
    entries = tuple(
        sorted(
            (
                BundleEntryV1.from_bytes(
                    path="01_job_summary.md",
                    media_type="text/markdown",
                    data=b"# Job Summary\n",
                ),
                BundleEntryV1.from_bytes(
                    path="typst/cover_letter.typ",
                    media_type="text/plain",
                    data=b"// generated cover\n",
                ),
            ),
            key=lambda entry: entry.path,
        )
    )
    bundle = ArtifactBundleV1(
        job_id=job.name,
        stage="package",
        mode="legacy_compatibility",
        input_fingerprint="a" * 64,
        entries=entries,
    )
    (job / "package_bundle.json").write_bytes(canonical_bundle_bytes(bundle))
    return bundle


def test_pre_workflow_migration_inspection_is_read_only_and_rollback_removes_state(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    before = {
        path.relative_to(job).as_posix(): path.read_bytes()
        for path in job.rglob("*")
        if path.is_file()
    }

    inspection = inspect_migration(workspace, job)

    assert inspection.status == "needed"
    assert inspection.source_shape == "pre_workflow"
    assert inspection.planned_changes[0].action == "created"
    assert not (job / "workflow").exists()
    assert {
        path.relative_to(job).as_posix(): path.read_bytes()
        for path in job.rglob("*")
        if path.is_file()
    } == before

    applied = apply_migration(workspace, job)
    state_path = job / "workflow" / "state.json"
    state_hash = sha256_bytes(state_path.read_bytes())

    assert applied.cache_hit is False
    assert applied.receipt.changes[0].after_sha256 == state_hash
    assert MigrationReceiptV1.model_validate(
        read_json_object(job / MIGRATION_RECEIPT_PATH)
    ) == applied.receipt

    rolled_back = rollback_migration(workspace, job)

    assert rolled_back.receipt.status == "complete"
    assert rolled_back.receipt.entries[0].outcome == "removed"
    assert not state_path.exists()
    assert inspect_migration(workspace, job).status == "rolled_back"
    assert (job / "job.yaml").is_file()
    assert (job / "job_advert.md").is_file()


def test_legacy_workspace_without_config_remains_readable_before_and_after_migration(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    (workspace / "canisend.yaml").unlink()

    inspection = inspect_migration(workspace, job)
    applied = apply_migration(workspace, job)

    assert inspection.source_shape == "pre_workflow"
    assert applied.receipt.job_id == job.name
    assert not (workspace / "canisend.yaml").exists()
    assert yaml.safe_load((job / "job.yaml").read_text(encoding="utf-8"))["title"] == (
        "Lecturer in Economics"
    )


def test_prior_schema_migration_replaces_canonical_state_and_restores_exact_bytes(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    state_path = job / "workflow" / "state.json"
    state_path.parent.mkdir(parents=True)
    legacy = {
        "schema_version": "1.0.0",
        "job_id": job.name,
        "revision": 0,
        "created_at": NOW,
        "updated_at": NOW,
        "active_run_id": None,
        "stages": [{"stage": "parse", "status": "ready", "attempt_count": 0, "inputs": [], "outputs": [], "error_code": None}],
    }
    legacy_bytes = json.dumps(legacy, separators=(",", ":")).encode("utf-8")
    state_path.write_bytes(legacy_bytes)

    inspection = inspect_migration(workspace, job)
    applied = apply_migration(workspace, job)

    assert inspection.source_shape == "prior_schema"
    assert applied.receipt.changes[0].action == "replaced"
    assert applied.receipt.changes[0].before_sha256 == sha256_bytes(legacy_bytes)
    assert state_path.read_bytes() != legacy_bytes
    assert WorkflowStateV1.model_validate(read_json_object(state_path)).schema_version == "1.0.0"

    rolled_back = rollback_migration(workspace, job)

    assert rolled_back.receipt.entries[0].outcome == "restored"
    assert state_path.read_bytes() == legacy_bytes


def test_interrupted_migration_resumes_from_immutable_plan(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)

    def fail_after_change(point: str) -> None:
        if point == "after_change:workflow/state.json":
            raise RuntimeError("injected migration interruption")

    try:
        apply_migration(workspace, job, failure_injector=fail_after_change)
    except RuntimeError as exc:
        assert "injected" in str(exc)
    else:  # pragma: no cover
        raise AssertionError("failure injector did not interrupt migration")

    assert (job / MIGRATION_PLAN_PATH).is_file()
    assert (job / "workflow" / "state.json").is_file()
    assert not (job / MIGRATION_RECEIPT_PATH).exists()

    resumed = apply_migration(workspace, job)

    assert resumed.cache_hit is False
    assert resumed.receipt.plan_sha256
    assert (job / MIGRATION_RECEIPT_PATH).is_file()


def test_migration_receipt_is_rejected_when_immutable_plan_hash_changes(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    apply_migration(workspace, job)
    plan_path = job / MIGRATION_PLAN_PATH
    plan_path.write_bytes(plan_path.read_bytes() + b"\n")

    with pytest.raises(MigrationError, match="immutable plan") as captured:
        inspect_migration(workspace, job)

    assert captured.value.code == "migration.invalid_receipt"


def test_rollback_preserves_hash_conflict_and_records_it(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    apply_migration(workspace, job)
    state_path = job / "workflow" / "state.json"
    conflicting = state_path.read_bytes() + b"\n"
    state_path.write_bytes(conflicting)

    outcome = rollback_migration(workspace, job)

    assert outcome.receipt.status == "conflict"
    assert outcome.conflicts == ("workflow/state.json",)
    assert state_path.read_bytes() == conflicting
    assert (job / "job.yaml").is_file()


def test_projection_repair_restores_missing_output_and_preserves_edited_typst(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    bundle = _package_bundle(job)
    project_artifact_bundle(job, bundle)
    markdown = job / "01_job_summary.md"
    primary = job / "typst" / "cover_letter.typ"
    markdown.unlink()
    edited = primary.read_bytes() + b"// USER EDIT\n"
    primary.write_bytes(edited)

    inspection = inspect_projection_repair(workspace, job, stage="package")
    outcome = repair_projection(workspace, job, stage="package")
    candidate = job / "typst" / "cover_letter.generated.typ"

    assert inspection.status == "repairable"
    assert "01_job_summary.md" in inspection.missing
    assert "typst/cover_letter.typ" in inspection.drifted
    assert outcome.cache_hit is False
    assert markdown.read_bytes() == b"# Job Summary\n"
    assert primary.read_bytes() == edited
    assert candidate.read_bytes() == b"// generated cover\n"
    assert inspect_artifact_projection(job, bundle).current is True
    assert RepairReceiptV1.model_validate(read_json_object(outcome.receipt_path))


def test_projection_repair_recovers_partial_projection_and_invalid_journal(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    bundle = _package_bundle(job)

    def interrupt_after_first(point: str) -> None:
        if point == "after_projection:01_job_summary.md":
            raise RuntimeError("injected partial projection")

    try:
        project_artifact_bundle(job, bundle, failure_injector=interrupt_after_first)
    except RuntimeError:
        pass
    else:  # pragma: no cover
        raise AssertionError("failure injector did not interrupt projection")

    first = repair_projection(workspace, job, stage="package")
    journal = job / "workflow" / "projections" / "package.json"
    journal.write_text("{\"schema_version\": \"999.0.0\"}\n", encoding="utf-8")
    second = repair_projection(workspace, job, stage="package")

    assert first.cache_hit is False
    assert second.cache_hit is False
    assert inspect_artifact_projection(job, bundle).current is True
    assert read_json_object(journal)["schema_version"] == "1.0.0"


def test_current_projection_repair_is_noop_with_truthful_receipt(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    bundle = _package_bundle(job)
    project_artifact_bundle(job, bundle)
    paths = (
        job / "01_job_summary.md",
        job / "typst" / "cover_letter.typ",
        job / "workflow" / "projections" / "package.json",
    )
    mtimes = {path: path.stat().st_mtime_ns for path in paths}

    outcome = repair_projection(workspace, job, stage="package")

    assert outcome.cache_hit is True
    assert {entry.outcome for entry in outcome.receipt.entries} == {"unchanged"}
    assert {path: path.stat().st_mtime_ns for path in paths} == mtimes


def test_state_repair_reconstructs_only_from_immutable_run_evidence(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    run_deterministic_stage(workspace, job, stage="evidence")
    state_path = job / "workflow" / "state.json"
    state_path.write_text("{\"invalid\": true}\n", encoding="utf-8")

    inspection = inspect_state_repair(workspace, job)
    outcome = repair_state(workspace, job)
    state = WorkflowStateV1.model_validate(read_json_object(state_path))

    assert inspection.status == "repairable"
    assert outcome.cache_hit is False
    assert state.stages[0].stage == "evidence"
    assert state.stages[0].status == "succeeded"
    assert RepairReceiptV1.model_validate(read_json_object(outcome.receipt_path))


def test_state_repair_never_overwrites_authoritative_output_drift(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    run_deterministic_stage(workspace, job, stage="evidence")
    output = job / "evidence_catalog.json"
    drifted = output.read_bytes() + b"\n"
    output.write_bytes(drifted)
    (job / "workflow" / "state.json").write_text(
        "{\"invalid\": true}\n",
        encoding="utf-8",
    )

    repair_state(workspace, job)
    status = inspect_stage_status(workspace, job, stage="evidence")

    assert output.read_bytes() == drifted
    assert status.output_drift is True
    assert "output_drift" in status.reasons


def test_state_repair_without_evidence_is_blocked_and_does_not_create_state(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)

    inspection = inspect_state_repair(workspace, job)

    assert inspection.status == "blocked"
    assert inspection.reason_codes == ("repair.migration_required",)
    assert not (job / "workflow").exists()


def test_migration_inventory_excludes_private_candidate_and_prepared_input_bodies(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    run_root = job / "workflow" / "runs" / ("run_" + "a" * 32)
    candidate = run_root / "candidates" / "candidate.json"
    prepared = run_root / "inputs" / "snapshot.json"
    metadata = run_root / "manifest.json"
    candidate.parent.mkdir(parents=True)
    prepared.parent.mkdir(parents=True)
    candidate.write_text('{"private_body": "candidate secret"}\n', encoding="utf-8")
    prepared.write_text('{"private_body": "input secret"}\n', encoding="utf-8")
    metadata.write_text('{"schema_version": "1.0.0"}\n', encoding="utf-8")

    outcome = apply_migration(workspace, job)
    observed = {item.path for item in outcome.receipt.observed_metadata}
    rendered = json.dumps(outcome.receipt.model_dump(mode="json"), sort_keys=True)

    assert "workflow/runs/run_" + "a" * 32 + "/manifest.json" in observed
    assert all("/candidates/" not in path and "/inputs/" not in path for path in observed)
    assert "candidate secret" not in rendered
    assert "input secret" not in rendered


def test_mutating_recovery_cli_commands_use_guarded_services(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    runner = CliRunner()

    applied = runner.invoke(
        app,
        [
            "migration",
            "apply",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--format",
            "json",
        ],
    )
    applied_payload = json.loads(applied.stdout)
    rolled_back = runner.invoke(
        app,
        [
            "migration",
            "rollback",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--format",
            "json",
        ],
    )
    rollback_payload = json.loads(rolled_back.stdout)

    assert applied.exit_code == 0
    assert applied_payload["operation"] == "workflow.migration_apply"
    assert applied_payload["extensions"]["canisend.migration.cache_hit"] is False
    assert applied_payload["artifacts"][0]["kind"] == "migration_receipt"
    assert rolled_back.exit_code == 0
    assert rollback_payload["operation"] == "workflow.migration_rollback"
    assert rollback_payload["extensions"]["canisend.migration.rollback_status"] == "complete"

    bundle = _package_bundle(job)
    project_artifact_bundle(job, bundle)
    (job / "01_job_summary.md").unlink()
    repaired = runner.invoke(
        app,
        [
            "repair",
            "projection",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--stage",
            "package",
            "--format",
            "json",
        ],
    )
    repaired_payload = json.loads(repaired.stdout)

    assert repaired.exit_code == 0
    assert repaired_payload["operation"] == "workflow.repair_projection"
    assert repaired_payload["artifacts"][0]["kind"] == "repair_receipt"
    assert inspect_artifact_projection(job, bundle).current is True


def test_migration_and_repair_cli_emit_body_free_agent_responses(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    runner = CliRunner()

    migration = runner.invoke(
        app,
        [
            "migration",
            "inspect",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--format",
            "json",
        ],
    )
    repair = runner.invoke(
        app,
        [
            "repair",
            "state",
            "--workspace",
            str(workspace),
            "--job",
            "example-role",
            "--dry-run",
            "--format",
            "json",
        ],
    )
    migration_payload = json.loads(migration.stdout)
    repair_payload = json.loads(repair.stdout)

    assert migration.exit_code == 0
    assert migration_payload["operation"] == "workflow.migration_inspect"
    assert migration_payload["extensions"]["canisend.migration.source_shape"] == "pre_workflow"
    assert repair.exit_code == 0
    assert repair_payload["operation"] == "workflow.repair_state"
    rendered = json.dumps((migration_payload, repair_payload), sort_keys=True)
    assert "Lecturer in Economics" not in rendered
    assert "PhD in Economics" not in rendered
