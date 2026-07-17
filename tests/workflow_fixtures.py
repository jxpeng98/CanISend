"""Immutable, process-local workflow templates for integration tests."""

from __future__ import annotations

from collections.abc import Callable
from contextvars import ContextVar
from hashlib import sha256
import json
import os
from pathlib import Path
import shutil


WorkflowBuilder = Callable[[Path], tuple[Path, Path]]
_ACTIVE_CACHE_ROOT: ContextVar[Path | None] = ContextVar(
    "canisend_prebuilt_workflow_cache_root",
    default=None,
)


class PrebuiltWorkflowError(RuntimeError):
    """Raised when a prebuilt test workflow is incomplete or was modified."""


def clone_prebuilt_workspace(
    tmp_path: Path,
    *,
    key: str,
    builder: WorkflowBuilder,
) -> tuple[Path, Path]:
    """Clone a byte-verified template into one test-owned workspace."""

    tmp_path.mkdir(parents=True, exist_ok=True)
    cache_root = _ACTIVE_CACHE_ROOT.get() or (
        tmp_path.parent / f".canisend-prebuilt-{os.getpid()}"
    )
    cache_root.mkdir(parents=True, exist_ok=True)
    template_root = cache_root / key
    manifest_path = template_root / "manifest.json"
    template_workspace = template_root / "workspace"

    if not manifest_path.is_file():
        staging = cache_root / f".{key}.staging-{os.getpid()}"
        if staging.exists():
            shutil.rmtree(staging)
        staging.mkdir(parents=True)
        token = _ACTIVE_CACHE_ROOT.set(cache_root)
        try:
            workspace, job = builder(staging)
        finally:
            _ACTIVE_CACHE_ROOT.reset(token)
        expected_workspace = staging / "workspace"
        expected_job = expected_workspace / "jobs" / "example-role"
        if workspace != expected_workspace or job != expected_job:
            raise PrebuiltWorkflowError(
                "A prebuilt workflow builder returned an unexpected layout."
            )
        manifest = _workspace_manifest(workspace)
        (staging / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        try:
            staging.replace(template_root)
        except OSError as exc:
            raise PrebuiltWorkflowError(
                "The prebuilt workflow template could not be finalized."
            ) from exc

    expected_manifest = _load_manifest(manifest_path)
    if _workspace_manifest(template_workspace) != expected_manifest:
        raise PrebuiltWorkflowError(
            "A prebuilt workflow template changed after it was finalized."
        )

    destination = tmp_path / "workspace"
    if destination.exists():
        raise PrebuiltWorkflowError(
            "A prebuilt workflow cannot replace an existing test workspace."
        )
    shutil.copytree(
        template_workspace,
        destination,
        copy_function=shutil.copyfile,
    )
    if _workspace_manifest(template_workspace) != expected_manifest:
        raise PrebuiltWorkflowError(
            "Cloning unexpectedly modified the prebuilt workflow template."
        )
    return destination, destination / "jobs" / "example-role"


def _workspace_manifest(workspace: Path) -> dict[str, str]:
    if not workspace.is_dir():
        raise PrebuiltWorkflowError("A prebuilt workflow has no workspace directory.")
    manifest: dict[str, str] = {}
    for path in sorted(workspace.rglob("*")):
        if path.is_symlink():
            raise PrebuiltWorkflowError("Prebuilt workflow templates cannot contain links.")
        if path.is_file():
            manifest[path.relative_to(workspace).as_posix()] = sha256(
                path.read_bytes()
            ).hexdigest()
    if not manifest:
        raise PrebuiltWorkflowError("A prebuilt workflow template is empty.")
    return manifest


def _load_manifest(path: Path) -> dict[str, str]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise PrebuiltWorkflowError(
            "A prebuilt workflow manifest is unreadable."
        ) from exc
    if not isinstance(payload, dict) or not all(
        isinstance(key, str) and isinstance(value, str)
        for key, value in payload.items()
    ):
        raise PrebuiltWorkflowError("A prebuilt workflow manifest is invalid.")
    return payload
