from __future__ import annotations

from pathlib import Path

from canisend.resource_files import copy_resource_tree


VALID_EXPORT_KINDS = {"codex-plugin", "skills-only"}
CANONICAL_SKILL_RESOURCE = "skills"


def install_workspace_skill_pack(
    destination: Path,
    *,
    overwrite: bool = False,
) -> list[Path]:
    """Install the canonical skill pack into a workspace-local agent-skills directory."""
    return copy_resource_tree(
        CANONICAL_SKILL_RESOURCE,
        destination,
        overwrite=overwrite,
    )


def export_skill_distribution(target: Path, *, kind: str, overwrite: bool = False) -> list[Path]:
    if kind not in VALID_EXPORT_KINDS:
        allowed = ", ".join(sorted(VALID_EXPORT_KINDS))
        raise ValueError(f"Unknown export kind '{kind}'. Expected one of: {allowed}.")

    target = target.expanduser()
    if _is_non_empty_directory(target) and not overwrite:
        raise ValueError(f"{target} is not empty. Use --overwrite to replace existing skill files.")

    target.mkdir(parents=True, exist_ok=True)
    copied: list[Path] = []
    if kind == "codex-plugin":
        copied.extend(copy_resource_tree(".codex-plugin", target / ".codex-plugin", overwrite=True))
        copied.extend(copy_resource_tree(CANONICAL_SKILL_RESOURCE, target / "skills", overwrite=True))
    else:
        copied.extend(copy_resource_tree(CANONICAL_SKILL_RESOURCE, target, overwrite=True))
    return copied


def _is_non_empty_directory(path: Path) -> bool:
    return path.exists() and path.is_dir() and any(path.iterdir())
