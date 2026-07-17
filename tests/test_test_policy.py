from __future__ import annotations

from pathlib import Path
import tomllib

from scripts.test_policy import (
    FAST_TEST_MODULES,
    RELEASE_TEST_MODULES,
    SLOW_TEST_MODULES,
)


ROOT = Path(__file__).resolve().parents[1]


def test_test_lane_manifests_are_sorted_unique_and_resolve_to_modules():
    for manifest in (
        FAST_TEST_MODULES,
        SLOW_TEST_MODULES,
        RELEASE_TEST_MODULES,
    ):
        assert manifest == tuple(sorted(manifest))
        assert len(manifest) == len(set(manifest))
        assert manifest
        for relative_path in manifest:
            assert relative_path.startswith("tests/test_")
            assert (ROOT / relative_path).is_file()


def test_fast_gate_owns_critical_contract_surfaces():
    required = {
        "tests/test_agent_protocol.py",
        "tests/test_bundle_projection.py",
        "tests/test_release_productization.py",
        "tests/test_repository_contract.py",
        "tests/test_stage_models.py",
        "tests/test_stage_registry.py",
        "tests/test_stage_runtime.py",
        "tests/test_stage_store.py",
        "tests/test_test_policy.py",
        "tests/test_workflow_sequence.py",
    }

    assert required <= set(FAST_TEST_MODULES)


def test_pytest_registers_every_repository_lane():
    pytest_config = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))[
        "tool"
    ]["pytest"]["ini_options"]
    markers = tuple(pytest_config["markers"])

    for marker in ("fast", "integration", "slow", "release"):
        assert any(entry.startswith(f"{marker}:") for entry in markers)
    assert pytest_config["addopts"] == "--strict-markers"


def test_ordinary_ci_runs_one_python_312_fast_gate():
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")

    assert 'python-version: "3.12"' in workflow
    assert 'python-version: ["3.11", "3.12", "3.13"]' not in workflow
    assert "python -m pytest -q -m fast" in workflow
    assert "github.event_name == 'workflow_dispatch'" in workflow
    assert "github.ref == 'refs/heads/main'" in workflow
    assert "os: [ubuntu-latest, macos-latest, windows-latest]" not in workflow


def test_release_keeps_full_and_cross_platform_gates_before_publication():
    workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")

    assert "python -m pytest -q\n" in workflow
    assert "cross-os-smoke:" in workflow
    assert "os: [ubuntu-latest, macos-latest, windows-latest]" in workflow
    assert "Smoke test Stage 5 workflow contract" in workflow
    assert "Smoke test Stage 4 discovery contract" in workflow
    assert "needs: [build, cross-os-smoke]" in workflow
