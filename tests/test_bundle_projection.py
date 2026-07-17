from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator
from pydantic import ValidationError

from canisend.bundle_models import ArtifactBundleV1, BundleEntryV1, ProjectionJournalV1
from canisend.bundle_projection import (
    BundleProjectionError,
    canonical_bundle_bytes,
    inspect_artifact_projection,
    project_artifact_bundle,
)
from canisend.stages.verify_stage import (
    ApplicationGateReportV1,
    VerifyStageError,
    build_verify_basis,
)


def _bundle(job: Path, entries: dict[str, bytes], *, stage: str = "package") -> ArtifactBundleV1:
    return ArtifactBundleV1(
        job_id=job.name,
        stage=stage,
        input_fingerprint="a" * 64,
        entries=tuple(
            BundleEntryV1.from_bytes(
                path=path,
                media_type=(
                    "application/pdf"
                    if path.endswith(".pdf")
                    else "text/plain"
                ),
                data=data,
            )
            for path, data in sorted(entries.items())
        ),
    )


def test_bundle_entries_reject_path_hash_and_order_tampering(tmp_path: Path) -> None:
    job = tmp_path / "job"
    job.mkdir()
    entry = BundleEntryV1.from_bytes(path="01_job_summary.md", media_type="text/markdown", data=b"ok\n")

    with pytest.raises(ValidationError):
        BundleEntryV1(**{**entry.model_dump(), "path": "../escape"})
    with pytest.raises(ValidationError):
        BundleEntryV1(**{**entry.model_dump(), "sha256": "0" * 64})
    with pytest.raises(ValidationError):
        ArtifactBundleV1(
            job_id=job.name,
            stage="package",
            input_fingerprint="a" * 64,
            entries=(
                BundleEntryV1.from_bytes(path="z.md", media_type="text/markdown", data=b"z"),
                entry,
            ),
        )
    with pytest.raises(ValidationError):
        ArtifactBundleV1(
            job_id=job.name,
            stage="package",
            input_fingerprint="a" * 64,
            entries=(
                BundleEntryV1.from_bytes(
                    path="job.yaml",
                    media_type="text/yaml",
                    data=b"status: packaged\n",
                ),
            ),
        )


def test_projection_is_idempotent_and_journaled(tmp_path: Path) -> None:
    job = tmp_path / "job"
    job.mkdir()
    bundle = _bundle(
        job,
        {
            "01_job_summary.md": b"summary\n",
            "typst/cover_letter_content.json": b"{}\n",
        },
    )

    first = project_artifact_bundle(job, bundle)
    first_mtime = (job / "01_job_summary.md").stat().st_mtime_ns
    second = project_artifact_bundle(job, bundle)

    assert [entry.outcome for entry in first.entries] == ["created", "created"]
    assert [entry.outcome for entry in second.entries] == ["unchanged", "unchanged"]
    assert (job / "01_job_summary.md").stat().st_mtime_ns == first_mtime
    journal = json.loads(
        (job / "workflow" / "projections" / "package.json").read_text()
    )
    assert journal["bundle_sha256"] == second.bundle_sha256
    assert inspect_artifact_projection(job, bundle).current is True


def test_projection_preserves_edited_typst_and_writes_candidate(tmp_path: Path) -> None:
    job = tmp_path / "job"
    (job / "typst").mkdir(parents=True)
    primary = job / "typst" / "cover_letter.typ"
    first = _bundle(job, {"typst/cover_letter.typ": b"generated v1\n"})
    project_artifact_bundle(job, first)
    primary.write_text("user edit\n", encoding="utf-8")
    second = _bundle(job, {"typst/cover_letter.typ": b"generated v2\n"})

    journal = project_artifact_bundle(job, second)

    assert primary.read_text() == "user edit\n"
    assert (job / "typst" / "cover_letter.generated.typ").read_bytes() == b"generated v2\n"
    assert journal.entries[0].outcome == "candidate_created"
    assert inspect_artifact_projection(job, second).current is True


def test_projection_rejects_edits_to_both_primary_and_candidate(tmp_path: Path) -> None:
    job = tmp_path / "job"
    (job / "typst").mkdir(parents=True)
    primary = job / "typst" / "cover_letter.typ"
    primary.write_text("user primary\n")
    candidate = job / "typst" / "cover_letter.generated.typ"
    candidate.write_text("user candidate\n")
    bundle = _bundle(job, {"typst/cover_letter.typ": b"generated\n"})

    with pytest.raises(BundleProjectionError) as captured:
        project_artifact_bundle(job, bundle)

    assert captured.value.code == "projection.output_conflict"
    assert primary.read_text() == "user primary\n"
    assert candidate.read_text() == "user candidate\n"


def test_projection_resumes_after_failure_before_journal(tmp_path: Path) -> None:
    job = tmp_path / "job"
    job.mkdir()
    bundle = _bundle(job, {"01_job_summary.md": b"summary\n"})

    def fail(point: str) -> None:
        if point == "before_projection_journal":
            raise RuntimeError("injected projection crash")

    with pytest.raises(RuntimeError, match="injected"):
        project_artifact_bundle(job, bundle, failure_injector=fail)

    assert (job / "01_job_summary.md").read_bytes() == b"summary\n"
    assert not (job / "workflow" / "projections" / "package.json").exists()
    resumed = project_artifact_bundle(job, bundle)
    assert resumed.entries[0].outcome == "unchanged"


def test_render_bundle_accepts_only_pdf_entries(tmp_path: Path) -> None:
    job = tmp_path / "job"
    job.mkdir()
    valid = _bundle(job, {"pdf/cover_letter.pdf": b"%PDF-fake"}, stage="render")
    assert valid.entries[0].media_type == "application/pdf"

    with pytest.raises(ValidationError):
        _bundle(job, {"typst/cover_letter.typ": b"source"}, stage="render")


def test_legacy_compatibility_bundle_cannot_enter_verify_readiness(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    bundle = _bundle(job, {"01_job_summary.md": b"legacy\n"}).model_copy(
        update={"mode": "legacy_compatibility"}
    )
    (job / "package_bundle.json").write_bytes(canonical_bundle_bytes(bundle))
    project_artifact_bundle(job, bundle)

    with pytest.raises(VerifyStageError, match="guarded"):
        build_verify_basis(workspace, job)


@pytest.mark.parametrize(
    ("schema_name", "payload"),
    [
        (
            "artifact-bundle.schema.json",
            ArtifactBundleV1(
                job_id="example-role",
                stage="package",
                input_fingerprint="a" * 64,
                entries=(
                    BundleEntryV1.from_bytes(
                        path="01_job_summary.md",
                        media_type="text/markdown",
                        data=b"summary\n",
                    ),
                ),
            ).model_dump(mode="json"),
        ),
        (
            "projection-journal.schema.json",
            ProjectionJournalV1(
                job_id="example-role",
                stage="package",
                bundle_sha256="b" * 64,
                entries=(),
            ).model_dump(mode="json"),
        ),
        (
            "application-gate-report.schema.json",
            ApplicationGateReportV1(
                job_id="example-role",
                package_bundle_sha256="a" * 64,
                projection_journal_sha256="b" * 64,
                status="PASS",
                input_hashes={},
                issues=(),
                generated_at="2026-07-17T10:00:00Z",
                input_fingerprint="c" * 64,
            ).model_dump(mode="json"),
        ),
    ],
)
def test_bundle_contract_models_match_packaged_json_schemas(
    schema_name: str,
    payload: dict[str, object],
) -> None:
    schema = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(payload)
