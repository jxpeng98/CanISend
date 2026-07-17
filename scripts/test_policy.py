"""Repository-owned test-lane policy used by pytest and CI contract tests."""

from __future__ import annotations


# Keep this list small and high-signal. Ordinary push/PR feedback runs every test
# in these modules on the minimum supported Python version.
FAST_TEST_MODULES: tuple[str, ...] = (
    "tests/test_agent_protocol.py",
    "tests/test_bundle_projection.py",
    "tests/test_discovery_models.py",
    "tests/test_examples.py",
    "tests/test_release_productization.py",
    "tests/test_repository_contract.py",
    "tests/test_resource_files.py",
    "tests/test_schema_validation.py",
    "tests/test_stage4_resources.py",
    "tests/test_stage_models.py",
    "tests/test_stage_registry.py",
    "tests/test_stage_runtime.py",
    "tests/test_stage_store.py",
    "tests/test_test_policy.py",
    "tests/test_workflow_sequence.py",
)


# These modules remain part of the complete suite. The marker makes their cost
# visible and allows focused performance work without silently excluding them.
SLOW_TEST_MODULES: tuple[str, ...] = (
    "tests/test_draft_stage.py",
    "tests/test_draft_views.py",
    "tests/test_package_readiness_gate.py",
    "tests/test_package_review_disposition_cli.py",
    "tests/test_package_review_disposition_mutations.py",
    "tests/test_package_review_stage.py",
    "tests/test_package_stage.py",
    "tests/test_research_statement_projection.py",
    "tests/test_research_statement_readiness.py",
    "tests/test_review_disposition_cli.py",
    "tests/test_review_disposition_mutations.py",
)


# Release tests may also be fast or integration tests. Release tags run the
# complete suite before packaging, while this marker supports focused diagnosis.
RELEASE_TEST_MODULES: tuple[str, ...] = (
    "tests/test_release_productization.py",
    "tests/test_release_script.py",
    "tests/test_repository_contract.py",
    "tests/test_resource_files.py",
    "tests/test_skill_distribution.py",
    "tests/test_stage4_resources.py",
    "tests/test_versioning.py",
    "tests/test_workspace_productization.py",
)
