from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
from threading import Barrier

import pytest
import yaml

from canisend.decision_models import (
    ApplicationDecisionV1,
    ConfirmedCorrectionsV1,
    CriteriaCatalogV1,
)
from canisend.agent_protocol import KNOWN_AGENT_ERROR_CODES
from canisend.stage_runtime import run_deterministic_stage
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    UserFileStoreError,
    has_interrupted_safe_publication,
    load_strict_yaml,
    load_strict_json,
    read_safe_bytes,
    repair_interrupted_safe_publication,
    write_safe_immutable_file,
)
from canisend.user_mutations import (
    ConfirmCriterionPatch,
    CorrectCriterionPatch,
    ConfirmEmptyCriteriaPatch,
    ResetDecisionPatch,
    SetDecisionPatch,
    UserMutationError,
    USER_MUTATION_ERROR_CODES,
    WithdrawCriterionPatch,
    apply_user_patch,
    initialize_application_decision,
    initialize_confirmed_corrections,
    inspect_application_decision,
    inspect_current_artifact_mutation,
    inspect_user_artifact,
    inspect_user_mutation,
    recover_user_mutation,
)


def _write_workspace(tmp_path: Path, *, empty_criteria: bool = False) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "example-role"
    job_dir.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (job_dir / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": "example-role",
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/job",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct",
                "created_at": "2026-07-11T10:00:00Z",
                "updated_at": "2026-07-11T10:00:00Z",
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    advert = (
        """# Lecturer in Economics

Department: Economics
Location: London
Required documents: CV, Cover letter

The appointment supports the department's teaching and research.
"""
        if empty_criteria
        else """# Lecturer in Economics

Department: Economics
Location: London
Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics

Desirable criteria:
- Experience teaching econometrics
"""
    )
    (job_dir / "job_advert.md").write_text(advert, encoding="utf-8")
    return workspace, job_dir


def _run_to_match(workspace: Path, job_dir: Path) -> None:
    run_deterministic_stage(workspace, job_dir, stage="parse")
    run_deterministic_stage(workspace, job_dir, stage="confirm")
    run_deterministic_stage(workspace, job_dir, stage="evidence")
    run_deterministic_stage(workspace, job_dir, stage="match")


def test_strict_yaml_rejects_duplicate_alias_anchor_merge_and_tag() -> None:
    invalid = (
        b"a: 1\na: 2\n",
        b"a: &value 1\nb: *value\n",
        b"base: &base {a: 1}\nvalue: {<<: *base}\n",
        b"value: !!str text\n",
        b"!!python/object:unsafe {}\n",
    )
    for payload in invalid:
        with pytest.raises(InvalidUserFileError):
            load_strict_yaml(payload)


def test_strict_json_rejects_duplicate_keys() -> None:
    with pytest.raises(InvalidUserFileError):
        load_strict_json(b'{"revision": 1, "revision": 2}\n')


def test_safe_reader_rejects_symlink_hardlink_nonregular_and_oversize(tmp_path: Path) -> None:
    job = tmp_path / "job"
    job.mkdir()
    target = job / "target.yaml"
    target.write_text("value: 1\n", encoding="utf-8")

    with pytest.raises(UnsafeUserFileError):
        read_safe_bytes(job, "target.yaml", max_bytes=2)

    if os.name != "nt":
        link = job / "link.yaml"
        link.symlink_to(target)
        with pytest.raises(UnsafeUserFileError):
            read_safe_bytes(job, "link.yaml")
        link.unlink()

        alias = job / "alias.yaml"
        os.link(target, alias)
        with pytest.raises(UnsafeUserFileError):
            read_safe_bytes(job, "target.yaml")


@pytest.mark.skipif(os.name == "nt", reason="hard-link publication fault requires POSIX")
def test_interrupted_link_publication_is_read_only_discoverable_and_repairable(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    job = tmp_path / "job"
    job.mkdir()
    relative = "workflow/user-mutations/events/mutation_" + "8" * 32 + "/candidate.yaml"
    target = job / relative
    from canisend import user_file_store

    original_unlink = user_file_store.os.unlink
    skipped = False

    def interrupt_after_link(path: object, *args: object, **kwargs: object) -> None:
        nonlocal skipped
        name = os.fsdecode(os.fspath(path))
        if not skipped and Path(name).name.startswith(".canisend-"):
            skipped = True
            return
        original_unlink(path, *args, **kwargs)

    monkeypatch.setattr(user_file_store.os, "unlink", interrupt_after_link)
    monkeypatch.setattr(user_file_store, "_supports_descriptor_write", lambda: True)
    with pytest.raises(UnsafeUserFileError):
        write_safe_immutable_file(job, relative, b"private: accepted\n")
    monkeypatch.setattr(user_file_store.os, "unlink", original_unlink)

    aliases = tuple(target.parent.glob(".canisend-*.tmp"))
    assert skipped is True
    assert len(aliases) == 1
    assert target.stat().st_nlink == aliases[0].stat().st_nlink == 2
    with pytest.raises(UnsafeUserFileError):
        read_safe_bytes(job, relative)

    snapshot = read_safe_bytes(
        job,
        relative,
        allow_interrupted_publication=True,
    )
    assert snapshot.interrupted_publication is True
    assert snapshot.data == b"private: accepted\n"
    assert aliases[0].exists()
    assert has_interrupted_safe_publication(job, relative) is True

    assert repair_interrupted_safe_publication(job, relative) is True
    assert not aliases[0].exists()
    assert target.stat().st_nlink == 1
    assert read_safe_bytes(job, relative).data == snapshot.data
    assert repair_interrupted_safe_publication(job, relative) is False


@pytest.mark.skipif(os.name == "nt", reason="hard-link safety check requires POSIX")
def test_interrupted_publication_repair_never_accepts_an_ordinary_hardlink(
    tmp_path: Path,
) -> None:
    job = tmp_path / "job"
    job.mkdir()
    target = job / "target.yaml"
    target.write_bytes(b"private: value\n")
    target.chmod(0o600)
    alias = job / "ordinary-alias.yaml"
    os.link(target, alias)

    assert has_interrupted_safe_publication(job, "target.yaml") is False
    with pytest.raises(UnsafeUserFileError):
        read_safe_bytes(job, "target.yaml", allow_interrupted_publication=True)
    with pytest.raises(UnsafeUserFileError):
        repair_interrupted_safe_publication(job, "target.yaml")
    with pytest.raises(UnsafeUserFileError):
        write_safe_immutable_file(job, "target.yaml", target.read_bytes())
    assert target.exists() and alias.exists()
    assert target.stat().st_nlink == alias.stat().st_nlink == 2


@pytest.mark.skipif(os.name == "nt", reason="hard-link publication fault requires POSIX")
def test_fresh_session_recovers_a_claim_interrupted_after_link_before_unlink(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    mutation_id = "mutation_" + "c" * 32
    from canisend import user_file_store

    original_unlink = user_file_store.os.unlink
    publication_unlinks = 0

    def interrupt_claim_publication(path: object, *args: object, **kwargs: object) -> None:
        nonlocal publication_unlinks
        name = os.fsdecode(os.fspath(path))
        if Path(name).name.startswith(".canisend-"):
            publication_unlinks += 1
            if publication_unlinks == 2:
                return
        original_unlink(path, *args, **kwargs)

    monkeypatch.setattr(user_file_store.os, "unlink", interrupt_claim_publication)
    monkeypatch.setattr(user_file_store, "_supports_descriptor_write", lambda: True)
    with pytest.raises(UserMutationError) as interrupted:
        initialize_application_decision(
            workspace,
            job,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_file_store.os, "unlink", original_unlink)

    assert interrupted.value.code == "user_input.recovery_required"
    assert interrupted.value.mutation_id == mutation_id
    assert publication_unlinks == 2
    audit = inspect_current_artifact_mutation(workspace, job, "decision")
    assert (audit.status, audit.mutation_id) == ("promotion_pending", mutation_id)
    claim_aliases = tuple(
        (job / "workflow/user-mutations/claims/decision").glob(".canisend-*.tmp")
    )
    assert len(claim_aliases) == 1
    assert claim_aliases[0].stat().st_nlink == 2

    recovered = recover_user_mutation(
        workspace,
        job,
        mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"
    assert recovered.changed is True
    assert not claim_aliases[0].exists()
    assert inspect_current_artifact_mutation(workspace, job, "decision").status == "committed"


@pytest.mark.skipif(os.name == "nt", reason="hard-link publication fallback requires POSIX")
def test_portable_interrupted_publication_repair_uses_the_same_strict_marker(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    job = tmp_path / "job"
    job.mkdir()
    target = job / "target.yaml"
    target.write_bytes(b"private: value\n")
    target.chmod(0o600)
    alias = job / ".canisend-999-dddddddddddddddd.tmp"
    os.link(target, alias)
    from canisend import user_file_store

    monkeypatch.setattr(user_file_store, "_supports_descriptor_repair", lambda: False)
    assert repair_interrupted_safe_publication(job, "target.yaml") is True
    assert not alias.exists()
    assert target.stat().st_nlink == 1


@pytest.mark.skipif(os.name == "nt", reason="portable hard-link fault fixture requires POSIX")
def test_real_portable_writer_leaves_a_recoverable_marker_when_unlink_is_interrupted(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    job = tmp_path / "job"
    job.mkdir()
    relative = "workflow/private.json"
    target = job / relative
    from canisend import user_file_store

    original_unlink = user_file_store.os.unlink
    skipped = False

    def interrupt_portable_unlink(path: object, *args: object, **kwargs: object) -> None:
        nonlocal skipped
        name = os.fsdecode(os.fspath(path))
        if not skipped and Path(name).name.startswith(".canisend-"):
            skipped = True
            return
        original_unlink(path, *args, **kwargs)

    monkeypatch.setattr(user_file_store, "_supports_descriptor_write", lambda: False)
    monkeypatch.setattr(user_file_store.os, "unlink", interrupt_portable_unlink)
    with pytest.raises(UnsafeUserFileError):
        write_safe_immutable_file(job, relative, b'{"private": true}\n')
    monkeypatch.setattr(user_file_store.os, "unlink", original_unlink)

    aliases = tuple(target.parent.glob(".canisend-*.tmp"))
    assert skipped is True
    assert len(aliases) == 1
    assert target.stat().st_nlink == aliases[0].stat().st_nlink == 2
    assert has_interrupted_safe_publication(job, relative) is True
    assert repair_interrupted_safe_publication(job, relative) is True
    assert target.stat().st_nlink == 1
    assert not aliases[0].exists()


@pytest.mark.skipif(not hasattr(os, "mkfifo") or os.name == "nt", reason="FIFO requires POSIX")
def test_safe_reader_rejects_fifo_without_blocking(tmp_path: Path) -> None:
    job = tmp_path / "job"
    job.mkdir()
    os.mkfifo(job / "input.yaml")
    with pytest.raises(UnsafeUserFileError):
        read_safe_bytes(job, "input.yaml")


@pytest.mark.skipif(os.name == "nt", reason="descriptor-relative writes require POSIX")
def test_descriptor_write_symlink_replacement_does_not_redirect_write(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    job = tmp_path / "job"
    job.mkdir()
    outside = tmp_path / "outside"
    outside.mkdir()
    from canisend import user_file_store

    original = user_file_store._open_safe_parent
    swapped = False

    def swap_after_open(job_dir: Path, relative_path: str) -> tuple[int, str]:
        nonlocal swapped
        descriptor, name = original(job_dir, relative_path)
        if not swapped:
            swapped = True
            (job / "workflow").rename(job / "workflow-original")
            (job / "workflow").symlink_to(outside, target_is_directory=True)
        return descriptor, name

    monkeypatch.setattr(user_file_store, "_open_safe_parent", swap_after_open)
    with pytest.raises((UnsafeUserFileError, UserFileStoreError)):
        write_safe_immutable_file(
            job,
            "workflow/user-mutations/events/mutation_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/candidate.yaml",
            b"private: sentinel\n",
        )

    assert not (outside / "user-mutations").exists()


def test_portable_write_rechecks_parent_before_dispatch(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    job = tmp_path / "job"
    job.mkdir()
    (job / "workflow").mkdir()
    outside = tmp_path / "outside"
    outside.mkdir()
    from canisend import user_file_store

    original = user_file_store._safe_write_target
    calls = 0

    def swap_after_initial_check(
        job_dir: Path,
        relative_path: str,
        *,
        require_existing: bool,
    ) -> Path:
        nonlocal calls
        target = original(
            job_dir,
            relative_path,
            require_existing=require_existing,
        )
        calls += 1
        if calls == 1:
            (job / "workflow").rename(job / "workflow-original")
            (job / "workflow").symlink_to(outside, target_is_directory=True)
        return target

    monkeypatch.setattr(user_file_store, "_supports_descriptor_write", lambda: False)
    monkeypatch.setattr(user_file_store, "_safe_write_target", swap_after_initial_check)

    with pytest.raises(UnsafeUserFileError):
        write_safe_immutable_file(
            job,
            "workflow/user-mutations/events/mutation_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/candidate.yaml",
            b"private: sentinel\n",
        )

    assert not (outside / "user-mutations").exists()


def test_initialize_is_create_only_idempotent_and_requires_consent(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    with pytest.raises(UserMutationError) as denied:
        initialize_confirmed_corrections(workspace, job, consent_confirmed=False)
    assert denied.value.code == "user_input.consent_required"
    assert not (job / "confirmed_corrections.yaml").exists()

    first = initialize_confirmed_corrections(
        workspace,
        job,
        consent_confirmed=True,
        mutation_id="mutation_" + "1" * 32,
    )
    original = first.snapshot.raw_bytes
    second = initialize_confirmed_corrections(
        workspace,
        job,
        consent_confirmed=True,
    )

    assert first.status == "committed"
    assert first.snapshot.revision == 0
    assert first.receipt_path is not None
    assert second.status == "reused"
    assert second.changed is False
    assert (job / "confirmed_corrections.yaml").read_bytes() == original


def test_initialize_refuses_invalid_existing_yaml_without_rewrite(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    path = job / "application_decision.yaml"
    path.write_text("decision: apply\ndecision: skip\n", encoding="utf-8")
    before = path.read_bytes()

    with pytest.raises(UserMutationError) as captured:
        initialize_application_decision(workspace, job, consent_confirmed=True)

    assert captured.value.code == "user_input.invalid"
    assert path.read_bytes() == before


@pytest.mark.skipif(os.name == "nt", reason="symlink creation may require elevated privileges")
def test_initialize_refuses_dangling_target_and_parent_symlink(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    target = job / "confirmed_corrections.yaml"
    target.symlink_to(job / "missing.yaml")
    with pytest.raises(UserMutationError) as captured:
        initialize_confirmed_corrections(workspace, job, consent_confirmed=True)
    assert captured.value.code == "user_input.unsafe_path"

    target.unlink()
    outside = tmp_path / "outside"
    outside.mkdir()
    (job / "workflow").symlink_to(outside, target_is_directory=True)
    with pytest.raises(UserMutationError):
        initialize_confirmed_corrections(workspace, job, consent_confirmed=True)
    assert not (outside / "user-mutations").exists()


def test_corrections_patch_uses_catalog_receipts_and_preserves_private_body(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    initialized = initialize_confirmed_corrections(
        workspace,
        job,
        consent_confirmed=True,
    )
    criteria = CriteriaCatalogV1.model_validate(
        json.loads((job / "criteria.json").read_text(encoding="utf-8"))
    )
    marker = "PRIVATE-CORRECTION-BODY-7319"

    updated = apply_user_patch(
        workspace,
        job,
        CorrectCriterionPatch(
            criterion_id=criteria.criteria[0].criterion_id,
            corrected_text=marker,
        ),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=0,
        consent_confirmed=True,
    )
    overlay = ConfirmedCorrectionsV1.model_validate(updated.snapshot.model)
    record = overlay.criteria[-1]

    assert record.criterion_id == criteria.criteria[0].criterion_id
    assert record.target_criterion_sha256 == criteria.criteria[0].parsed_text_sha256
    assert record.confirmation == "corrected"
    assert record.corrected_text == marker
    assert updated.receipt_path is not None
    control = updated.claim_path.read_text(encoding="utf-8") + updated.receipt_path.read_text(
        encoding="utf-8"
    )
    assert marker not in control
    assert criteria.criteria[0].criterion_id not in control


def test_corrections_supersede_and_withdraw_active_record(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    initialized = initialize_confirmed_corrections(workspace, job, consent_confirmed=True)
    criteria = CriteriaCatalogV1.model_validate_json((job / "criteria.json").read_text(encoding="utf-8"))
    criterion_id = criteria.criteria[0].criterion_id
    first = apply_user_patch(
        workspace,
        job,
        ConfirmCriterionPatch(criterion_id=criterion_id),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=0,
        consent_confirmed=True,
    )
    run_deterministic_stage(workspace, job, stage="confirm")
    second = apply_user_patch(
        workspace,
        job,
        CorrectCriterionPatch(criterion_id=criterion_id, corrected_text="Reviewed wording"),
        expected_sha256=first.snapshot.sha256,
        expected_revision=1,
        consent_confirmed=True,
    )
    overlay = second.snapshot.model
    assert isinstance(overlay, ConfirmedCorrectionsV1)
    assert overlay.criteria[0].record_state == "superseded"
    assert overlay.criteria[0].superseded_by == overlay.criteria[1].correction_id

    run_deterministic_stage(workspace, job, stage="confirm")
    third = apply_user_patch(
        workspace,
        job,
        WithdrawCriterionPatch(criterion_id=criterion_id),
        expected_sha256=second.snapshot.sha256,
        expected_revision=2,
        consent_confirmed=True,
    )
    assert isinstance(third.snapshot.model, ConfirmedCorrectionsV1)
    assert third.snapshot.model.criteria[0].record_state == "superseded"
    assert (
        third.snapshot.model.criteria[0].superseded_by
        == third.snapshot.model.criteria[1].correction_id
    )
    assert third.snapshot.model.criteria[-1].record_state == "withdrawn"


def test_correction_patch_rejects_stale_confirm_after_advert_reparse(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    initialized = initialize_confirmed_corrections(
        workspace,
        job,
        consent_confirmed=True,
    )
    criteria = CriteriaCatalogV1.model_validate_json(
        (job / "criteria.json").read_text(encoding="utf-8")
    )
    before = initialized.snapshot.raw_bytes
    advert = (job / "job_advert.md").read_text(encoding="utf-8")
    (job / "job_advert.md").write_text(
        advert.replace("PhD in Economics", "PhD in Economics or Finance"),
        encoding="utf-8",
    )
    run_deterministic_stage(workspace, job, stage="parse")

    with pytest.raises(UserMutationError) as captured:
        apply_user_patch(
            workspace,
            job,
            ConfirmCriterionPatch(criterion_id=criteria.criteria[0].criterion_id),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            consent_confirmed=True,
        )

    assert captured.value.code == "user_input.dependency_not_current"
    assert (job / "confirmed_corrections.yaml").read_bytes() == before


def test_correction_rejects_catalog_drift_after_runtime_checks(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    initialized = initialize_confirmed_corrections(workspace, job, consent_confirmed=True)
    criteria = CriteriaCatalogV1.model_validate_json(
        (job / "criteria.json").read_text(encoding="utf-8")
    )
    before = initialized.snapshot.raw_bytes
    from canisend import user_mutations

    original = user_mutations._require_stage_current
    calls = 0

    def drift_after_final_status(workspace_path: Path, job_path: Path, stage: str) -> None:
        nonlocal calls
        original(workspace_path, job_path, stage)
        calls += 1
        if calls == 3:
            path = job / "criteria.json"
            path.write_text(path.read_text(encoding="utf-8") + " ", encoding="utf-8")

    monkeypatch.setattr(user_mutations, "_require_stage_current", drift_after_final_status)

    with pytest.raises(UserMutationError) as captured:
        apply_user_patch(
            workspace,
            job,
            ConfirmCriterionPatch(criterion_id=criteria.criteria[0].criterion_id),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            consent_confirmed=True,
        )

    assert captured.value.code == "user_input.dependency_not_current"
    assert (job / "confirmed_corrections.yaml").read_bytes() == before


def test_confirm_empty_patch_creates_bound_extraction_confirmation(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path, empty_criteria=True)
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    catalog = CriteriaCatalogV1.model_validate_json((job / "criteria.json").read_text(encoding="utf-8"))
    assert catalog.extraction_state == "unknown"
    initialized = initialize_confirmed_corrections(workspace, job, consent_confirmed=True)

    updated = apply_user_patch(
        workspace,
        job,
        ConfirmEmptyCriteriaPatch(),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=0,
        consent_confirmed=True,
    )
    overlay = updated.snapshot.model
    assert isinstance(overlay, ConfirmedCorrectionsV1)
    assert overlay.criteria_extraction_confirmations[-1].confirmation == "confirmed_empty"

    run_deterministic_stage(workspace, job, stage="confirm")
    confirmed = CriteriaCatalogV1.model_validate_json((job / "criteria.json").read_text(encoding="utf-8"))
    assert confirmed.extraction_state == "confirmed_empty"


def test_hash_and_revision_are_both_required_for_cas(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    before = initialized.snapshot.raw_bytes
    for expected_hash, expected_revision in (
        ("0" * 64, 0),
        (initialized.snapshot.sha256, 1),
    ):
        with pytest.raises(UserMutationError) as captured:
            apply_user_patch(
                workspace,
                job,
                ResetDecisionPatch(),
                expected_sha256=expected_hash,
                expected_revision=expected_revision,
                consent_confirmed=True,
            )
        assert captured.value.code == "user_input.conflict"
        assert (job / "application_decision.yaml").read_bytes() == before


@pytest.mark.parametrize("portable_writer", (False, True))
def test_concurrent_cooperative_writers_have_one_claim_winner(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    portable_writer: bool,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations
    if portable_writer:
        from canisend import user_file_store

        monkeypatch.setattr(user_file_store, "_supports_descriptor_write", lambda: False)

    original = user_mutations._store_claim
    barrier = Barrier(2)

    def synchronized_store(job_path: Path, claim: object) -> Path:
        barrier.wait(timeout=5)
        return original(job_path, claim)

    monkeypatch.setattr(user_mutations, "_store_claim", synchronized_store)

    def attempt(mutation_id: str) -> tuple[str, str | None]:
        try:
            outcome = apply_user_patch(
                workspace,
                job,
                ResetDecisionPatch(),
                expected_sha256=initialized.snapshot.sha256,
                expected_revision=0,
                mutation_id=mutation_id,
                consent_confirmed=True,
            )
            return outcome.status, outcome.mutation_id
        except UserMutationError as exc:
            return exc.code, exc.mutation_id

    with ThreadPoolExecutor(max_workers=2) as executor:
        results = tuple(
            executor.map(
                attempt,
                ("mutation_" + "6" * 32, "mutation_" + "7" * 32),
            )
        )

    assert sorted(status for status, _mutation_id in results) == [
        "committed",
        "user_input.recovery_required",
    ]
    committed_id = next(
        mutation_id for status, mutation_id in results if status == "committed"
    )
    recovery_id = next(
        mutation_id
        for status, mutation_id in results
        if status == "user_input.recovery_required"
    )
    assert recovery_id == committed_id
    snapshot = inspect_user_artifact(workspace, job, "decision")
    assert snapshot is not None
    assert snapshot.revision == 1
    claim_names = {
        path.name
        for path in (job / "workflow/user-mutations/claims/decision").glob("*.json")
    }
    assert "absent.json" in claim_names
    assert len(tuple(name for name in claim_names if name.startswith("r0-"))) == 1


def test_historical_baseline_claim_does_not_mask_later_revision_conflict(
    tmp_path: Path,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    first = apply_user_patch(
        workspace,
        job,
        ResetDecisionPatch(),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=0,
        consent_confirmed=True,
    )
    apply_user_patch(
        workspace,
        job,
        ResetDecisionPatch(),
        expected_sha256=first.snapshot.sha256,
        expected_revision=1,
        consent_confirmed=True,
    )

    with pytest.raises(UserMutationError) as captured:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            consent_confirmed=True,
        )

    assert captured.value.code == "user_input.conflict"
    assert captured.value.mutation_id is None


def test_lost_response_after_claim_is_discoverable_from_baseline(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations._finish_claim
    mutation_id = "mutation_" + "8" * 32
    monkeypatch.setattr(
        user_mutations,
        "_finish_claim",
        lambda *args, **kwargs: (_ for _ in ()).throw(SystemExit("simulated crash")),
    )
    with pytest.raises(SystemExit):
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "_finish_claim", original)

    with pytest.raises(UserMutationError) as discovered:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            consent_confirmed=True,
        )

    assert discovered.value.code == "user_input.recovery_required"
    assert discovered.value.mutation_id == mutation_id
    recovered = recover_user_mutation(
        workspace,
        job,
        mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"


def test_orphan_candidate_without_claim_requires_a_new_mutation_id(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations._store_claim
    mutation_id = "mutation_" + "a" * 32
    monkeypatch.setattr(
        user_mutations,
        "_store_claim",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated claim failure")
        ),
    )
    with pytest.raises(UserMutationError) as failed:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    assert failed.value.code == "user_input.store_failed"
    monkeypatch.setattr(user_mutations, "_store_claim", original)

    with pytest.raises(UserMutationError) as reused:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    assert reused.value.code == "user_input.conflict"
    assert reused.value.mutation_id is None

    fresh = apply_user_patch(
        workspace,
        job,
        ResetDecisionPatch(),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=0,
        consent_confirmed=True,
    )
    assert fresh.status == "committed"


def test_lost_response_after_promotion_is_discoverable_from_old_baseline(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations._store_receipt
    mutation_id = "mutation_" + "9" * 32
    monkeypatch.setattr(
        user_mutations,
        "_store_receipt",
        lambda *args, **kwargs: (_ for _ in ()).throw(SystemExit("simulated crash")),
    )
    with pytest.raises(SystemExit):
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "_store_receipt", original)

    with pytest.raises(UserMutationError) as discovered:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            consent_confirmed=True,
        )

    assert discovered.value.code == "user_input.recovery_required"
    assert discovered.value.mutation_id == mutation_id
    recovered = recover_user_mutation(
        workspace,
        job,
        mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"
    assert recovered.changed is False


def test_decision_is_explicitly_bound_and_staleness_is_read_only(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    marker = "PRIVATE-DECISION-RATIONALE-9921"
    decided = apply_user_patch(
        workspace,
        job,
        SetDecisionPatch(
            decision="apply",
            rationale_mode="set",
            rationale=marker,
        ),
        expected_sha256=initialized.snapshot.sha256,
        expected_revision=0,
        consent_confirmed=True,
    )
    decision = decided.snapshot.model
    assert isinstance(decision, ApplicationDecisionV1)
    assert decision.decision == "apply"
    assert decision.confirmation_state == "confirmed"
    assert inspect_application_decision(workspace, job).basis_status == "current"
    control = decided.claim_path.read_text(encoding="utf-8") + decided.receipt_path.read_text(
        encoding="utf-8"
    )
    assert marker not in control
    before = (job / "application_decision.yaml").read_bytes()

    criteria_path = job / "criteria.json"
    criteria_path.write_text(criteria_path.read_text(encoding="utf-8") + " ", encoding="utf-8")
    inspection = inspect_application_decision(workspace, job)
    assert inspection.basis_status == "review_required"
    assert (job / "application_decision.yaml").read_bytes() == before


def test_decision_rejects_basis_drift_after_runtime_checks(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    before = initialized.snapshot.raw_bytes
    from canisend import user_mutations

    original = user_mutations._require_stage_current
    calls = 0

    def drift_after_final_status(workspace_path: Path, job_path: Path, stage: str) -> None:
        nonlocal calls
        original(workspace_path, job_path, stage)
        calls += 1
        if calls == 4:
            for name in ("criteria.json", "criterion_matches.json"):
                path = job / name
                path.write_text(path.read_text(encoding="utf-8") + " ", encoding="utf-8")

    monkeypatch.setattr(user_mutations, "_require_stage_current", drift_after_final_status)

    with pytest.raises(UserMutationError) as captured:
        apply_user_patch(
            workspace,
            job,
            SetDecisionPatch(decision="apply"),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            consent_confirmed=True,
        )

    assert captured.value.code == "user_input.dependency_not_current"
    assert (job / "application_decision.yaml").read_bytes() == before


def test_receipt_failure_is_committed_pending_then_fresh_recovery(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    from canisend import user_mutations

    original = user_mutations._store_receipt

    def fail_receipt(*args: object, **kwargs: object) -> Path:
        raise UserFileStoreError("simulated receipt failure")

    monkeypatch.setattr(user_mutations, "_store_receipt", fail_receipt)
    outcome = initialize_application_decision(
        workspace,
        job,
        consent_confirmed=True,
        mutation_id="mutation_" + "2" * 32,
    )
    assert outcome.status == "committed_receipt_pending"
    assert outcome.snapshot.path.is_file()
    assert inspect_user_mutation(workspace, job, outcome.mutation_id).status == "receipt_pending"
    assert inspect_current_artifact_mutation(workspace, job, "decision").status == "receipt_pending"

    with pytest.raises(UserMutationError) as next_write:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=outcome.snapshot.sha256,
            expected_revision=outcome.snapshot.revision,
            consent_confirmed=True,
        )
    assert next_write.value.code == "user_input.recovery_required"
    assert next_write.value.mutation_id == outcome.mutation_id

    monkeypatch.setattr(user_mutations, "_store_receipt", original)
    recovered = recover_user_mutation(
        workspace,
        job,
        outcome.mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"
    assert recovered.receipt_path is not None
    assert recovered.changed is False
    assert inspect_current_artifact_mutation(workspace, job, "decision").status == "committed"


def test_claim_survives_pre_promotion_failure_and_blocks_competing_writer(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations.replace_safe_file

    def fail_replace(*args: object, **kwargs: object) -> Path:
        raise UserFileStoreError("simulated replace failure")

    monkeypatch.setattr(user_mutations, "replace_safe_file", fail_replace)
    mutation_id = "mutation_" + "3" * 32
    with pytest.raises(UserMutationError) as failed:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    assert failed.value.code == "user_input.recovery_required"
    assert failed.value.mutation_id == mutation_id
    assert inspect_user_mutation(workspace, job, mutation_id).status == "promotion_pending"
    fresh = inspect_current_artifact_mutation(workspace, job, "decision")
    assert (fresh.status, fresh.mutation_id) == ("promotion_pending", mutation_id)

    with pytest.raises(UserMutationError) as competing:
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id="mutation_" + "4" * 32,
            consent_confirmed=True,
        )
    assert competing.value.code == "user_input.recovery_required"
    assert competing.value.mutation_id == mutation_id

    monkeypatch.setattr(user_mutations, "replace_safe_file", original)
    recovered = recover_user_mutation(
        workspace,
        job,
        mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"
    assert recovered.changed is True


@pytest.mark.parametrize(
    "candidate_state",
    ("missing", "invalid", "wrong_artifact", "wrong_revision", "wrong_hash"),
)
def test_current_audit_never_reports_promotion_pending_for_invalid_candidate(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    candidate_state: str,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    mutation_id = "mutation_" + "6" * 32
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=initialized.snapshot.revision,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)

    candidate = (
        job
        / "workflow"
        / "user-mutations"
        / "events"
        / mutation_id
        / "candidate.yaml"
    )
    if candidate_state == "missing":
        candidate.unlink()
    elif candidate_state == "invalid":
        candidate.write_text("decision: [\n", encoding="utf-8")
    elif candidate_state == "wrong_artifact":
        wrong = ConfirmedCorrectionsV1(
            job_id=job.name,
            revision=1,
            updated_at="2026-07-11T12:00:00Z",
        )
        candidate.write_text(
            yaml.safe_dump(wrong.model_dump(mode="json"), sort_keys=False),
            encoding="utf-8",
        )
    elif candidate_state == "wrong_revision":
        payload = yaml.safe_load(candidate.read_text(encoding="utf-8"))
        payload["revision"] = 7
        candidate.write_text(
            yaml.safe_dump(payload, sort_keys=False),
            encoding="utf-8",
        )
    else:
        candidate.write_bytes(candidate.read_bytes() + b"# changed bytes\n")

    audit = inspect_current_artifact_mutation(workspace, job, "decision")
    direct = inspect_user_mutation(workspace, job, mutation_id)

    assert (audit.status, audit.mutation_id) == ("conflict", mutation_id)
    assert direct.status == "conflict"


@pytest.mark.skipif(os.name == "nt", reason="symlink candidate safety check requires POSIX")
def test_current_audit_never_reports_unsafe_candidate_as_promotion_pending(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    mutation_id = "mutation_" + "7" * 32
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=initialized.snapshot.revision,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)

    candidate = (
        job
        / "workflow"
        / "user-mutations"
        / "events"
        / mutation_id
        / "candidate.yaml"
    )
    outside = tmp_path / "outside.yaml"
    outside.write_bytes(candidate.read_bytes())
    candidate.unlink()
    candidate.symlink_to(outside)

    audit = inspect_current_artifact_mutation(workspace, job, "decision")

    assert (audit.status, audit.mutation_id) == ("conflict", mutation_id)


@pytest.mark.skipif(os.name == "nt", reason="hard-link publication recovery requires POSIX")
@pytest.mark.parametrize("interrupted_path", ("claim", "candidate", "target", "receipt"))
def test_fresh_audit_and_recover_clean_every_interrupted_publication_link(
    tmp_path: Path,
    interrupted_path: str,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    mutation_id = "mutation_" + "9" * 32
    outcome = initialize_application_decision(
        workspace,
        job,
        mutation_id=mutation_id,
        consent_confirmed=True,
    )
    assert outcome.claim_path is not None
    assert outcome.receipt_path is not None
    candidate = job / f"workflow/user-mutations/events/{mutation_id}/candidate.yaml"
    selected = {
        "claim": outcome.claim_path,
        "candidate": candidate,
        "target": outcome.snapshot.path,
        "receipt": outcome.receipt_path,
    }[interrupted_path]
    alias = selected.parent / f".canisend-999-{interrupted_path.encode().hex()[:16]:0<16}.tmp"
    os.link(selected, alias)

    relative = selected.relative_to(job).as_posix()
    assert selected.stat().st_nlink == alias.stat().st_nlink == 2
    assert has_interrupted_safe_publication(job, relative) is True
    audit = inspect_current_artifact_mutation(workspace, job, "decision")
    direct = inspect_user_mutation(workspace, job, mutation_id)
    assert (audit.status, audit.mutation_id) == ("receipt_pending", mutation_id)
    assert direct.status == "receipt_pending"
    assert alias.exists()

    recovered = recover_user_mutation(
        workspace,
        job,
        mutation_id,
        consent_confirmed=True,
    )

    assert recovered.status == "committed"
    assert recovered.changed is False
    assert not alias.exists()
    assert selected.stat().st_nlink == 1
    assert inspect_current_artifact_mutation(workspace, job, "decision").status == "committed"


@pytest.mark.skipif(os.name == "nt", reason="hard-link publication recovery requires POSIX")
def test_promotion_recovery_cleans_an_interrupted_private_candidate_link(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            UserFileStoreError("simulated promotion failure")
        ),
    )
    mutation_id = "mutation_" + "a" * 32
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=initialized.snapshot.revision,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)
    candidate = job / f"workflow/user-mutations/events/{mutation_id}/candidate.yaml"
    alias = candidate.parent / ".canisend-999-aaaaaaaaaaaaaaaa.tmp"
    os.link(candidate, alias)

    audit = inspect_current_artifact_mutation(workspace, job, "decision")
    assert (audit.status, audit.mutation_id) == ("promotion_pending", mutation_id)
    assert alias.exists()

    recovered = recover_user_mutation(
        workspace,
        job,
        mutation_id,
        consent_confirmed=True,
    )
    assert recovered.status == "committed"
    assert recovered.changed is True
    assert not alias.exists()
    assert candidate.stat().st_nlink == 1


def test_recovery_conflict_never_overwrites_manual_valid_change(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    initialized = initialize_application_decision(workspace, job, consent_confirmed=True)
    from canisend import user_mutations

    original = user_mutations.replace_safe_file
    monkeypatch.setattr(
        user_mutations,
        "replace_safe_file",
        lambda *args, **kwargs: (_ for _ in ()).throw(UserFileStoreError("simulated")),
    )
    mutation_id = "mutation_" + "5" * 32
    with pytest.raises(UserMutationError):
        apply_user_patch(
            workspace,
            job,
            ResetDecisionPatch(),
            expected_sha256=initialized.snapshot.sha256,
            expected_revision=0,
            mutation_id=mutation_id,
            consent_confirmed=True,
        )
    monkeypatch.setattr(user_mutations, "replace_safe_file", original)

    manual = ApplicationDecisionV1(
        job_id=job.name,
        revision=7,
        updated_at="2026-07-12T12:00:00Z",
    )
    (job / "application_decision.yaml").write_text(
        yaml.safe_dump(manual.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )
    before = (job / "application_decision.yaml").read_bytes()
    assert inspect_user_mutation(workspace, job, mutation_id).status == "conflict"
    fresh = inspect_current_artifact_mutation(workspace, job, "decision")
    assert (fresh.status, fresh.mutation_id) == ("conflict", mutation_id)
    with pytest.raises(UserMutationError) as captured:
        recover_user_mutation(
            workspace,
            job,
            mutation_id,
            consent_confirmed=True,
        )
    assert captured.value.code == "user_input.recovery_required"
    assert (job / "application_decision.yaml").read_bytes() == before


def test_completed_receipt_history_does_not_mask_the_current_commit(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    first = initialize_application_decision(workspace, job, consent_confirmed=True)
    second = apply_user_patch(
        workspace,
        job,
        ResetDecisionPatch(),
        expected_sha256=first.snapshot.sha256,
        expected_revision=first.snapshot.revision,
        consent_confirmed=True,
    )

    audit = inspect_current_artifact_mutation(workspace, job, "decision")

    assert audit.status == "committed"
    assert audit.mutation_id == second.mutation_id


def test_user_mutation_error_codes_are_frozen_agent_protocol_codes() -> None:
    assert USER_MUTATION_ERROR_CODES <= KNOWN_AGENT_ERROR_CODES
