from __future__ import annotations

from importlib import resources as importlib_resources
from importlib.resources.abc import Traversable
from pathlib import Path


RESOURCE_PACKAGE = "canisend.resources"
REPO_ROOT = Path(__file__).resolve().parents[2]


def read_resource_text(relative_path: str, *, local_path: Path | None = None) -> str:
    """Read a user override if present, otherwise read a packaged default."""
    if local_path is not None and local_path.exists():
        return local_path.read_text(encoding="utf-8")

    return _default_resource(relative_path).read_text(encoding="utf-8")


def copy_resource_tree(relative_path: str, destination: Path, *, overwrite: bool = False) -> list[Path]:
    """Copy a packaged/default resource directory into a user workspace."""
    source = _default_resource(relative_path)
    copied: list[Path] = []

    if source.is_dir():
        _copy_directory(source, destination, overwrite=overwrite, copied=copied)
    else:
        _copy_file(source, destination, overwrite=overwrite, copied=copied)

    return copied


def _default_resource(relative_path: str) -> Traversable | Path:
    package_root = importlib_resources.files(RESOURCE_PACKAGE)
    packaged = package_root / relative_path
    if packaged.exists():
        return packaged

    source_tree = REPO_ROOT / relative_path
    if source_tree.exists():
        return source_tree

    raise FileNotFoundError(f"Default resource not found: {relative_path}")


def _copy_directory(source: Traversable | Path, destination: Path, *, overwrite: bool, copied: list[Path]) -> None:
    source_path = _resolve_filesystem_path(source)
    destination_path = destination.resolve()
    if source_path is not None:
        if destination_path == source_path:
            return
        if source_path in destination_path.parents:
            raise ValueError(
                f"Cannot copy resource directory into itself: {source_path} -> {destination_path}"
            )

    destination.mkdir(parents=True, exist_ok=True)
    for child in source.iterdir():
        child_destination = destination / child.name
        if child.is_dir():
            _copy_directory(child, child_destination, overwrite=overwrite, copied=copied)
        else:
            _copy_file(child, child_destination, overwrite=overwrite, copied=copied)


def _copy_file(source: Traversable | Path, destination: Path, *, overwrite: bool, copied: list[Path]) -> None:
    if destination.exists() and not overwrite:
        return

    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(source.read_bytes())
    copied.append(destination)


def _resolve_filesystem_path(source: Traversable | Path) -> Path | None:
    if not isinstance(source, Path):
        return None
    return source.resolve()
