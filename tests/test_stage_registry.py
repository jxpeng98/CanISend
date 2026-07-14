from __future__ import annotations

import pytest

from canisend.stage_registry import (
    DEFAULT_STAGE_REGISTRY,
    StageDefinition,
    StageRegistry,
    StageRegistryError,
)


def _ids(stages: tuple[StageDefinition, ...]) -> tuple[str, ...]:
    return tuple(stage.id for stage in stages)


def test_default_registry_exposes_complete_application_dag() -> None:
    assert _ids(DEFAULT_STAGE_REGISTRY.topological_order()) == (
        "intake",
        "evidence",
        "parse",
        "confirm",
        "match",
        "decide",
        "brief",
        "draft",
        "review",
        "package",
        "verify",
        "render",
    )
    assert DEFAULT_STAGE_REGISTRY.get("evidence").depends_on == ("intake",)
    assert DEFAULT_STAGE_REGISTRY.get("parse").depends_on == ("intake",)
    assert DEFAULT_STAGE_REGISTRY.get("confirm").depends_on == ("parse",)
    assert DEFAULT_STAGE_REGISTRY.get("match").depends_on == ("confirm", "evidence")
    assert DEFAULT_STAGE_REGISTRY.get("decide").depends_on == ("match", "confirm")
    assert DEFAULT_STAGE_REGISTRY.get("brief").depends_on == ("decide", "match", "confirm")
    assert DEFAULT_STAGE_REGISTRY.get("draft").depends_on == ("brief", "match", "evidence")
    assert DEFAULT_STAGE_REGISTRY.get("review").depends_on == ("draft",)
    assert DEFAULT_STAGE_REGISTRY.get("package").depends_on == ("review",)
    assert DEFAULT_STAGE_REGISTRY.get("verify").depends_on == ("package",)
    assert DEFAULT_STAGE_REGISTRY.get("render").depends_on == ("verify",)


def test_decision_spine_stages_are_implemented_in_the_registry() -> None:
    assert _ids(DEFAULT_STAGE_REGISTRY.implemented_stages()) == (
        "evidence",
        "parse",
        "confirm",
        "match",
        "brief",
        "draft",
        "review",
    )

    evidence = DEFAULT_STAGE_REGISTRY.get("evidence")
    assert evidence.execution_modes == ("deterministic",)
    assert evidence.authoritative_outputs == ("evidence_catalog.json",)
    parse = DEFAULT_STAGE_REGISTRY.get("parse")
    assert parse.execution_modes == ("deterministic", "host_agent")
    assert parse.authoritative_outputs == ("parsed_job.json",)
    confirm = DEFAULT_STAGE_REGISTRY.get("confirm")
    assert confirm.execution_modes == ("deterministic",)
    assert confirm.authoritative_outputs == ("criteria.json",)
    match = DEFAULT_STAGE_REGISTRY.get("match")
    assert match.execution_modes == ("deterministic",)
    assert match.authoritative_outputs == ("criterion_matches.json",)
    brief = DEFAULT_STAGE_REGISTRY.get("brief")
    assert brief.execution_modes == ("deterministic",)
    assert brief.authoritative_outputs == ("required_document_plan.json",)
    draft = DEFAULT_STAGE_REGISTRY.get("draft")
    assert draft.execution_modes == ("host_agent", "configured_provider")
    assert draft.authoritative_outputs == ("cover_letter_draft.json",)
    review = DEFAULT_STAGE_REGISTRY.get("review")
    assert review.execution_modes == ("deterministic",)
    assert review.authoritative_outputs == ("review_findings.json",)


def test_descendants_are_transitive_and_topologically_ordered() -> None:
    assert _ids(DEFAULT_STAGE_REGISTRY.descendants("parse")) == (
        "confirm",
        "match",
        "decide",
        "brief",
        "draft",
        "review",
        "package",
        "verify",
        "render",
    )
    assert _ids(DEFAULT_STAGE_REGISTRY.descendants("render")) == ()


def test_registry_rejects_duplicate_stage_ids() -> None:
    with pytest.raises(StageRegistryError, match="duplicate stage id: parse"):
        StageRegistry(
            [
                StageDefinition(id="parse"),
                StageDefinition(id="parse"),
            ]
        )


def test_registry_rejects_unknown_dependencies() -> None:
    with pytest.raises(StageRegistryError, match="parse.*unknown dependency: intake"):
        StageRegistry([StageDefinition(id="parse", depends_on=("intake",))])


def test_registry_rejects_dependency_cycles() -> None:
    with pytest.raises(StageRegistryError, match=r"dependency cycle.*parse.*match.*parse"):
        StageRegistry(
            [
                StageDefinition(id="parse", depends_on=("match",)),
                StageDefinition(id="match", depends_on=("parse",)),
            ]
        )


def test_registry_rejects_duplicate_authoritative_output_ownership() -> None:
    with pytest.raises(
        StageRegistryError,
        match=r"parsed_job\.json.*owned by both parse and review",
    ):
        StageRegistry(
            [
                StageDefinition(id="parse", authoritative_outputs=("parsed_job.json",)),
                StageDefinition(id="review", authoritative_outputs=("parsed_job.json",)),
            ]
        )
