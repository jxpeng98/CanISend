from __future__ import annotations

from pathlib import Path

import pytest

from tests.workflow_fixtures import (
    PrebuiltWorkflowError,
    clone_prebuilt_workspace,
)


def _build_minimal_workflow(root: Path) -> tuple[Path, Path]:
    workspace = root / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text("jobs_dir: jobs\n", encoding="utf-8")
    (job / "private-candidate.json").write_text(
        '{"sentinel":"private-test-value"}\n',
        encoding="utf-8",
    )
    return workspace, job


def test_prebuilt_workflow_clones_are_independent(tmp_path: Path) -> None:
    first_root = tmp_path / "first"
    second_root = tmp_path / "second"

    _first_workspace, first_job = clone_prebuilt_workspace(
        first_root,
        key="minimal",
        builder=_build_minimal_workflow,
    )
    (first_job / "private-candidate.json").write_text(
        '{"sentinel":"changed"}\n',
        encoding="utf-8",
    )
    _second_workspace, second_job = clone_prebuilt_workspace(
        second_root,
        key="minimal",
        builder=_build_minimal_workflow,
    )

    assert (second_job / "private-candidate.json").read_text(encoding="utf-8") == (
        '{"sentinel":"private-test-value"}\n'
    )


def test_prebuilt_workflow_refuses_an_existing_destination(tmp_path: Path) -> None:
    destination = tmp_path / "destination"
    (destination / "workspace").mkdir(parents=True)

    with pytest.raises(PrebuiltWorkflowError):
        clone_prebuilt_workspace(
            destination,
            key="existing-destination",
            builder=_build_minimal_workflow,
        )


def test_prebuilt_workflow_detects_template_mutation(tmp_path: Path) -> None:
    first_root = tmp_path / "first"
    clone_prebuilt_workspace(
        first_root,
        key="mutation-check",
        builder=_build_minimal_workflow,
    )
    cache_root = next(tmp_path.glob(".canisend-prebuilt-*"))
    template_file = (
        cache_root
        / "mutation-check"
        / "workspace"
        / "jobs"
        / "example-role"
        / "private-candidate.json"
    )
    template_file.write_text('{"sentinel":"tampered"}\n', encoding="utf-8")

    with pytest.raises(PrebuiltWorkflowError):
        clone_prebuilt_workspace(
            tmp_path / "second",
            key="mutation-check",
            builder=_build_minimal_workflow,
        )
