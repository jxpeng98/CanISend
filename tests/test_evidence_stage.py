from __future__ import annotations

import json
from hashlib import sha256
import os
from pathlib import Path

import pytest
import yaml

from canisend.stages import evidence_stage
from canisend.stages.evidence_stage import (
    EVIDENCE_MAX_SOURCE_BYTES_V1,
    EvidenceStageError,
    EvidenceStageValidationError,
    build_deterministic_evidence_candidate,
    canonical_evidence_kind,
    evidence_content_sha256,
    evidence_input_fingerprint,
    stable_evidence_id,
    validate_evidence_candidate,
)


SCHEMA_PATH = Path("schemas/evidence-catalog.schema.json")


def test_evidence_identity_uses_normalized_kind_and_body_only() -> None:
    first = stable_evidence_id(
        kind="dated-entry",
        text="Led  Econometrics\nseminars",
    )
    second = stable_evidence_id(
        kind="DATED ENTRY",
        text="led econometrics seminars",
    )

    assert first == second
    assert canonical_evidence_kind("llm-augmented") == "llm_augmented"
    assert evidence_content_sha256(" Evidence\nbody ") == evidence_content_sha256(
        "evidence body"
    )
    assert first != stable_evidence_id(kind="teaching", text="led econometrics seminars")
    assert first != stable_evidence_id(kind="dated-entry", text="led statistics seminars")


def test_evidence_projection_deduplicates_semantic_items_and_keeps_canonical_locator(
    tmp_path: Path,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(
        profile,
        generated={
            "z_evidence": "generated/z.evidence.md",
            "a_evidence": "generated/a.evidence.md",
        },
    )
    _write_evidence(
        profile / "generated" / "z.evidence.md",
        section="Research",
        locator="z-009",
        kind="dated-entry",
        text="Led applied econometrics seminars.",
    )
    _write_evidence(
        profile / "generated" / "a.evidence.md",
        section="Teaching",
        locator="a-002",
        kind="DATED ENTRY",
        text=" led  applied econometrics seminars. ",
    )

    fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    catalog = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )

    assert catalog.state == "available"
    assert len(catalog.items) == 1
    assert catalog.items[0].path == "profile/generated/a.evidence.md"
    assert catalog.items[0].section == "Teaching"
    assert catalog.items[0].item_locator == "a-002"
    assert catalog.items[0].kind == "dated_entry"
    assert catalog.items[0].reference.citation == (
        "profile/generated/a.evidence.md#Teaching/a-002"
    )
    generated_receipts = [
        receipt
        for receipt in catalog.source_receipts
        if receipt.source_type == "generated_evidence"
    ]
    assert [receipt.path for receipt in generated_receipts] == [
        "profile/generated/a.evidence.md",
        "profile/generated/z.evidence.md",
    ]
    assert [receipt.item_count for receipt in generated_receipts] == [1, 1]


def test_evidence_state_distinguishes_unavailable_empty_and_available(tmp_path: Path) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})

    unavailable_fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    unavailable = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=unavailable_fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert unavailable.state == "unavailable"
    assert unavailable.unavailable_reason == "evidence.generated_missing"
    assert unavailable.items == ()

    generated = profile / "generated" / "cv.evidence.md"
    generated.parent.mkdir()
    generated.write_text("# Evidence: cv\n\n_No evidence extracted._\n", encoding="utf-8")
    empty_fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    empty = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=empty_fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert empty.state == "empty"
    assert empty.unavailable_reason is None
    assert empty.items == ()

    _write_evidence(
        generated,
        section="Teaching",
        locator="cv-001",
        kind="teaching",
        text="Led seminars.",
    )
    available_fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    available = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=available_fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert available.state == "available"
    assert len(available.items) == 1
    assert len({unavailable_fingerprint, empty_fingerprint, available_fingerprint}) == 3


def test_evidence_binds_generated_catalog_to_current_profile_source(tmp_path: Path) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    source = profile / "typst" / "cv.typ"
    source.parent.mkdir()
    source_bytes = b"= CV\nCurrent source evidence.\n"
    source.write_bytes(source_bytes)
    (profile / "profile.yaml").write_text(
        "sources:\n"
        "  cv: typst/cv.typ\n"
        "generated:\n"
        "  cv_evidence: generated/cv.evidence.md\n",
        encoding="utf-8",
    )
    generated = profile / "generated" / "cv.evidence.md"
    _write_evidence(
        generated,
        section="Teaching",
        locator="cv-001",
        kind="teaching",
        text="Led seminars.",
    )

    missing_receipt_fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    missing_receipt = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=missing_receipt_fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert missing_receipt.state == "unavailable"
    assert missing_receipt.unavailable_reason == "evidence.source_receipt_missing"

    generated.write_text(
        generated.read_text(encoding="utf-8")
        + "<!-- canisend-source-sha256: "
        + sha256(source_bytes).hexdigest()
        + " -->\n",
        encoding="utf-8",
    )
    current_fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    current = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=current_fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert current.state == "available"
    assert any(
        receipt.source_type == "profile_source"
        for receipt in current.source_receipts
    )

    source.write_text("= CV\nChanged source evidence.\n", encoding="utf-8")
    stale_fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    stale = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=stale_fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert stale_fingerprint != current_fingerprint
    assert stale.state == "unavailable"
    assert stale.unavailable_reason == "evidence.source_receipt_stale"


def test_evidence_accepts_windows_crlf_source_receipt(tmp_path: Path) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    source = profile / "typst" / "cv.typ"
    source.parent.mkdir()
    source_bytes = b"= CV\r\nCurrent source evidence.\r\n"
    source.write_bytes(source_bytes)
    (profile / "profile.yaml").write_text(
        "sources:\n"
        "  cv: typst/cv.typ\n"
        "generated:\n"
        "  cv_evidence: generated/cv.evidence.md\n",
        encoding="utf-8",
    )
    generated = profile / "generated" / "cv.evidence.md"
    generated.parent.mkdir()
    generated.write_bytes(
        b"# Evidence: cv\r\n\r\n"
        + f"<!-- canisend-source-sha256: {sha256(source_bytes).hexdigest()} -->\r\n\r\n".encode()
        + b"## Teaching\r\n\r\n- [cv-001] `teaching`: Led seminars.\r\n"
    )

    fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    catalog = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )

    assert catalog.state == "available"
    assert catalog.unavailable_reason is None
    assert len(catalog.items) == 1


def test_evidence_fingerprint_ignores_timestamps_and_binds_schema_receipts_and_limits(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    generated = profile / "generated" / "cv.evidence.md"
    _write_evidence(
        generated,
        section="Teaching",
        locator="cv-001",
        kind="teaching",
        text="Led seminars.",
    )
    first = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )

    os.utime(profile / "profile.yaml", (100, 100))
    os.utime(generated, (200, 200))
    assert evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    ) == first

    schema_copy = tmp_path / "evidence.schema.json"
    schema_copy.write_text(SCHEMA_PATH.read_text(encoding="utf-8") + "\n", encoding="utf-8")
    assert evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=schema_copy,
    ) != first

    monkeypatch.setattr(
        evidence_stage,
        "EVIDENCE_MAX_SOURCE_BYTES_V1",
        EVIDENCE_MAX_SOURCE_BYTES_V1 - 1,
    )
    assert evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    ) != first


def test_evidence_candidate_validation_is_strict_semantic_and_stale_safe(
    tmp_path: Path,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    generated = profile / "generated" / "cv.evidence.md"
    _write_evidence(
        generated,
        section="Teaching",
        locator="cv-001",
        kind="teaching",
        text="Led seminars.",
    )
    fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    catalog = build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    payload = catalog.model_dump(mode="json")

    validated = validate_evidence_candidate(
        payload,
        workspace=workspace,
        job_dir=job_dir,
        input_fingerprint=fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert validated == catalog

    with pytest.raises(EvidenceStageValidationError):
        validate_evidence_candidate(
            {**payload, "unexpected": True},
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=fingerprint,
            evidence_schema_path=SCHEMA_PATH,
        )
    tampered = json.loads(json.dumps(payload))
    tampered["items"][0]["text"] = "Fabricated body"
    with pytest.raises(EvidenceStageValidationError):
        validate_evidence_candidate(
            tampered,
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=fingerprint,
            evidence_schema_path=SCHEMA_PATH,
        )

    generated.write_text(generated.read_text(encoding="utf-8") + "\nChanged\n", encoding="utf-8")
    with pytest.raises(EvidenceStageValidationError, match="stale"):
        validate_evidence_candidate(
            payload,
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=fingerprint,
            evidence_schema_path=SCHEMA_PATH,
        )


@pytest.mark.parametrize(
    "unsafe_path",
    [
        "/private/tmp/outside.evidence.md",
        "../outside.evidence.md",
        "generated/../outside.evidence.md",
        r"generated\outside.evidence.md",
    ],
)
def test_evidence_manifest_rejects_unsafe_generated_paths_before_body_read(
    tmp_path: Path,
    unsafe_path: str,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": unsafe_path})

    with pytest.raises(EvidenceStageError, match="normalized relative POSIX"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_rejects_external_profile_root(tmp_path: Path) -> None:
    workspace, job_dir, _profile = _workspace(tmp_path)
    outside = tmp_path / "outside-profile"
    outside.mkdir()
    (workspace / "canisend.yaml").write_text(
        f"profile_dir: {outside}\njobs_dir: jobs\n",
        encoding="utf-8",
    )

    with pytest.raises(EvidenceStageError, match="inside the workspace"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_rejects_symlinked_profile_root(tmp_path: Path) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    relocated = workspace / "real-profile"
    profile.rename(relocated)
    profile.symlink_to(relocated, target_is_directory=True)

    with pytest.raises(EvidenceStageError, match="symlink"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


@pytest.mark.parametrize("alias_kind", ["symlink", "dangling", "hardlink", "directory"])
def test_evidence_rejects_aliased_or_nonregular_generated_inputs(
    tmp_path: Path,
    alias_kind: str,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    generated_dir = profile / "generated"
    generated_dir.mkdir()
    configured = generated_dir / "cv.evidence.md"
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    if alias_kind == "symlink":
        target = generated_dir / "target.md"
        target.write_text("PRIVATE BODY\n", encoding="utf-8")
        configured.symlink_to(target)
    elif alias_kind == "dangling":
        configured.symlink_to(generated_dir / "missing.md")
    elif alias_kind == "hardlink":
        target = generated_dir / "target.md"
        target.write_text("PRIVATE BODY\n", encoding="utf-8")
        os.link(target, configured)
    else:
        configured.mkdir()

    with pytest.raises(EvidenceStageError):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_rejects_a_source_that_changes_during_bounded_read(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    generated = profile / "generated" / "cv.evidence.md"
    _write_evidence(
        generated,
        section="Teaching",
        locator="cv-001",
        kind="teaching",
        text="Led seminars.",
    )
    generated_inode = generated.stat().st_ino
    original_read = os.read
    changed = False

    def racing_read(descriptor: int, size: int) -> bytes:
        nonlocal changed
        content = original_read(descriptor, size)
        if not changed and os.fstat(descriptor).st_ino == generated_inode:
            generated.write_text(
                generated.read_text(encoding="utf-8") + "changed",
                encoding="utf-8",
            )
            changed = True
        return content

    monkeypatch.setattr(os, "read", racing_read)
    with pytest.raises(EvidenceStageError, match="changed while"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_enforces_versioned_single_and_total_input_limits(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    monkeypatch.setattr(evidence_stage, "EVIDENCE_MAX_SOURCE_BYTES_V1", 256)
    monkeypatch.setattr(evidence_stage, "EVIDENCE_MAX_TOTAL_BYTES_V1", 800)
    _write_manifest(profile, generated={"a_evidence": "generated/a.evidence.md"})
    generated = profile / "generated" / "a.evidence.md"
    generated.parent.mkdir()
    generated.write_bytes(b"# Evidence\n" + b"x" * (256 - len(b"# Evidence\n")))

    fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )
    assert build_deterministic_evidence_candidate(
        workspace,
        job_dir,
        input_fingerprint=fingerprint,
        evidence_schema_path=SCHEMA_PATH,
    ).state == "empty"

    generated.write_bytes(generated.read_bytes() + b"x")
    with pytest.raises(EvidenceStageError, match="source size limit"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )

    monkeypatch.setattr(evidence_stage, "EVIDENCE_MAX_SOURCE_BYTES_V1", 512)
    monkeypatch.setattr(evidence_stage, "EVIDENCE_MAX_TOTAL_BYTES_V1", 600)
    _write_manifest(
        profile,
        generated={
            "a_evidence": "generated/a.evidence.md",
            "b_evidence": "generated/b.evidence.md",
        },
    )
    generated.write_bytes(b"x" * 300)
    (profile / "generated" / "b.evidence.md").write_bytes(b"y" * 300)
    with pytest.raises(EvidenceStageError, match="total size limit"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_safe_open_fallback_supports_platforms_without_dir_fd(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    _write_evidence(
        profile / "generated" / "cv.evidence.md",
        section="Teaching",
        locator="cv-001",
        kind="teaching",
        text="Led seminars.",
    )
    monkeypatch.setattr(evidence_stage, "_supports_descriptor_walk", lambda: False)

    fingerprint = evidence_input_fingerprint(
        workspace,
        job_dir,
        evidence_schema_path=SCHEMA_PATH,
    )

    assert len(fingerprint) == 64

@pytest.mark.parametrize(
    "manifest_text",
    [
        "generated:\n  cv_evidence: generated/a.md\n  cv_evidence: generated/b.md\n",
        "generated: []\n",
    ],
)
def test_evidence_rejects_malformed_profile_manifest(
    tmp_path: Path,
    manifest_text: str,
) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    (profile / "profile.yaml").write_text(manifest_text, encoding="utf-8")

    with pytest.raises(EvidenceStageError):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_rejects_malformed_or_duplicate_legacy_citations(tmp_path: Path) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    generated = profile / "generated" / "cv.evidence.md"
    generated.parent.mkdir()
    generated.write_text(
        "# Evidence\n\n"
        "## Teaching\n\n"
        "- [cv-001] `teaching`: First item\n"
        "- [cv-001] `teaching`: Second item\n",
        encoding="utf-8",
    )

    with pytest.raises(EvidenceStageError, match="duplicate item citation"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def test_evidence_rejects_multiple_unlocated_items_in_one_section(tmp_path: Path) -> None:
    workspace, job_dir, profile = _workspace(tmp_path)
    _write_manifest(profile, generated={"cv_evidence": "generated/cv.evidence.md"})
    generated = profile / "generated" / "cv.evidence.md"
    generated.parent.mkdir()
    generated.write_text(
        "# Evidence\n\n"
        "## Teaching\n\n"
        "- `teaching`: First item\n"
        "- `teaching`: Second item\n",
        encoding="utf-8",
    )

    with pytest.raises(EvidenceStageError, match="duplicate item citation"):
        evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=SCHEMA_PATH,
        )


def _workspace(tmp_path: Path) -> tuple[Path, Path, Path]:
    workspace = tmp_path / "workspace"
    profile = workspace / "profile"
    job_dir = workspace / "jobs" / "example-role"
    profile.mkdir(parents=True)
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\n",
        encoding="utf-8",
    )
    return workspace, job_dir, profile


def _write_manifest(profile: Path, *, generated: dict[str, str]) -> None:
    (profile / "profile.yaml").write_text(
        yaml.safe_dump({"generated": generated}, sort_keys=False),
        encoding="utf-8",
    )


def _write_evidence(
    path: Path,
    *,
    section: str,
    locator: str,
    kind: str,
    text: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        f"# Evidence\n\n## {section}\n\n- [{locator}] `{kind}`: {text}\n",
        encoding="utf-8",
    )
