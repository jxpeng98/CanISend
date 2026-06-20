from __future__ import annotations

from pathlib import Path

import pytest

from canisend.resource_files import _copy_directory


def test_copy_directory_rejects_destination_inside_source(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "prompt.md").write_text("default prompt\n", encoding="utf-8")
    destination = source / "prompts"
    copied: list[Path] = []

    with pytest.raises(ValueError, match="Cannot copy resource directory into itself"):
        _copy_directory(source, destination, overwrite=False, copied=copied)

    assert copied == []
    assert not destination.exists()


def test_copy_directory_noops_when_source_and_destination_match(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    existing = source / "prompt.md"
    existing.write_text("default prompt\n", encoding="utf-8")
    copied: list[Path] = []

    _copy_directory(source, source, overwrite=False, copied=copied)

    assert existing.read_text(encoding="utf-8") == "default prompt\n"
    assert copied == []


def test_copy_directory_copies_to_sibling_directory(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "prompt.md").write_text("default prompt\n", encoding="utf-8")
    destination = tmp_path / "workspace" / "prompts"
    copied: list[Path] = []

    _copy_directory(source, destination, overwrite=False, copied=copied)

    assert (destination / "prompt.md").read_text(encoding="utf-8") == "default prompt\n"
    assert copied == [destination / "prompt.md"]
