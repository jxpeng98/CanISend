from __future__ import annotations

from pathlib import Path

import pytest

from scripts.test_policy import (
    FAST_TEST_MODULES,
    RELEASE_TEST_MODULES,
    SLOW_TEST_MODULES,
)


_FAST = frozenset(FAST_TEST_MODULES)
_SLOW = frozenset(SLOW_TEST_MODULES)
_RELEASE = frozenset(RELEASE_TEST_MODULES)


def pytest_collection_modifyitems(
    config: pytest.Config,
    items: list[pytest.Item],
) -> None:
    """Assign every test to the repository-owned execution lanes."""

    root = Path(str(config.rootpath)).resolve()
    for item in items:
        try:
            relative = Path(str(item.path)).resolve().relative_to(root).as_posix()
        except ValueError:
            continue
        if relative in _FAST:
            item.add_marker(pytest.mark.fast)
        else:
            item.add_marker(pytest.mark.integration)
        if relative in _SLOW:
            item.add_marker(pytest.mark.slow)
        if relative in _RELEASE:
            item.add_marker(pytest.mark.release)
