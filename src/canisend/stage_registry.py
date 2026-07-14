from __future__ import annotations

from dataclasses import dataclass
from pathlib import PurePosixPath
import re
from typing import Iterable, Literal


ExecutionMode = Literal["deterministic", "host_agent", "configured_provider"]

_SAFE_STAGE_ID = re.compile(r"^[a-z][a-z0-9_]*$")


class StageRegistryError(ValueError):
    """Raised when stage definitions do not form a safe dependency graph."""


@dataclass(frozen=True)
class StageDefinition:
    """Static ownership and dependency metadata for one workflow stage."""

    id: str
    depends_on: tuple[str, ...] = ()
    implemented: bool = False
    execution_modes: tuple[ExecutionMode, ...] = ()
    authoritative_outputs: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        object.__setattr__(self, "depends_on", tuple(self.depends_on))
        object.__setattr__(self, "execution_modes", tuple(self.execution_modes))
        object.__setattr__(self, "authoritative_outputs", tuple(self.authoritative_outputs))

        _validate_stage_id(self.id, label="stage id")
        for dependency in self.depends_on:
            _validate_stage_id(dependency, label=f"dependency of {self.id}")
        if len(set(self.depends_on)) != len(self.depends_on):
            raise StageRegistryError(f"stage {self.id} declares a duplicate dependency")
        if self.id in self.depends_on:
            raise StageRegistryError(f"dependency cycle: {self.id} -> {self.id}")

        if len(set(self.execution_modes)) != len(self.execution_modes):
            raise StageRegistryError(f"stage {self.id} declares a duplicate execution mode")
        for mode in self.execution_modes:
            if mode not in {"deterministic", "host_agent", "configured_provider"}:
                raise StageRegistryError(f"stage {self.id} has unsupported execution mode: {mode}")

        normalized_outputs = tuple(
            _normalize_authoritative_output(path, stage_id=self.id)
            for path in self.authoritative_outputs
        )
        if len(set(normalized_outputs)) != len(normalized_outputs):
            raise StageRegistryError(f"stage {self.id} declares a duplicate authoritative output")
        object.__setattr__(self, "authoritative_outputs", normalized_outputs)


class StageRegistry:
    """Validated, deterministic view of workflow-stage definitions."""

    def __init__(self, definitions: Iterable[StageDefinition]) -> None:
        ordered = tuple(definitions)
        by_id: dict[str, StageDefinition] = {}
        for definition in ordered:
            if definition.id in by_id:
                raise StageRegistryError(f"duplicate stage id: {definition.id}")
            by_id[definition.id] = definition

        for definition in ordered:
            for dependency in definition.depends_on:
                if dependency not in by_id:
                    raise StageRegistryError(
                        f"stage {definition.id} has unknown dependency: {dependency}"
                    )

        self._definitions = ordered
        self._by_id = by_id
        self._topological = self._derive_topological_order()
        self._validate_output_ownership()

    def get(self, stage_id: str) -> StageDefinition:
        """Return one definition, raising ``KeyError`` for an unknown stage."""

        try:
            return self._by_id[stage_id]
        except KeyError as exc:
            raise KeyError(f"unknown stage: {stage_id}") from exc

    def topological_order(self) -> tuple[StageDefinition, ...]:
        """Return every stage in stable dependency-first order."""

        return self._topological

    def descendants(self, stage_id: str) -> tuple[StageDefinition, ...]:
        """Return all transitive dependants in stable topological order."""

        self.get(stage_id)
        discovered: set[str] = set()
        pending = [stage_id]
        while pending:
            dependency = pending.pop()
            for definition in self._definitions:
                if dependency not in definition.depends_on or definition.id in discovered:
                    continue
                discovered.add(definition.id)
                pending.append(definition.id)
        return tuple(
            definition
            for definition in self._topological
            if definition.id in discovered
        )

    def implemented_stages(self) -> tuple[StageDefinition, ...]:
        """Return implemented stages in stable topological order."""

        return tuple(definition for definition in self._topological if definition.implemented)

    def _derive_topological_order(self) -> tuple[StageDefinition, ...]:
        visiting: set[str] = set()
        visited: set[str] = set()
        result: list[StageDefinition] = []

        def visit(stage_id: str, path: tuple[str, ...]) -> None:
            if stage_id in visiting:
                cycle_start = path.index(stage_id)
                cycle = (*path[cycle_start:], stage_id)
                raise StageRegistryError(f"dependency cycle: {' -> '.join(cycle)}")
            if stage_id in visited:
                return

            visiting.add(stage_id)
            definition = self._by_id[stage_id]
            for dependency in definition.depends_on:
                visit(dependency, (*path, stage_id))
            visiting.remove(stage_id)
            visited.add(stage_id)
            result.append(definition)

        for definition in self._definitions:
            visit(definition.id, ())
        return tuple(result)

    def _validate_output_ownership(self) -> None:
        owners: dict[str, str] = {}
        for definition in self._definitions:
            for output in definition.authoritative_outputs:
                owner = owners.get(output)
                if owner is not None:
                    raise StageRegistryError(
                        f"authoritative output {output} is owned by both {owner} and {definition.id}"
                    )
                owners[output] = definition.id


def _validate_stage_id(value: str, *, label: str) -> None:
    if _SAFE_STAGE_ID.fullmatch(value) is None:
        raise StageRegistryError(f"{label} must be a lowercase safe identifier: {value}")


def _normalize_authoritative_output(value: str, *, stage_id: str) -> str:
    path = PurePosixPath(value)
    if path.is_absolute() or value in {"", "."} or any(part in {"", ".", ".."} for part in path.parts):
        raise StageRegistryError(
            f"stage {stage_id} authoritative output must be a safe relative path: {value}"
        )
    return path.as_posix()


DEFAULT_STAGE_REGISTRY = StageRegistry(
    (
        StageDefinition(id="intake"),
        StageDefinition(
            id="evidence",
            depends_on=("intake",),
            implemented=True,
            execution_modes=("deterministic",),
            authoritative_outputs=("evidence_catalog.json",),
        ),
        StageDefinition(
            id="parse",
            depends_on=("intake",),
            implemented=True,
            execution_modes=("deterministic", "host_agent"),
            authoritative_outputs=("parsed_job.json",),
        ),
        StageDefinition(
            id="confirm",
            depends_on=("parse",),
            implemented=True,
            execution_modes=("deterministic",),
            authoritative_outputs=("criteria.json",),
        ),
        StageDefinition(
            id="match",
            depends_on=("confirm", "evidence"),
            implemented=True,
            execution_modes=("deterministic",),
            authoritative_outputs=("criterion_matches.json",),
        ),
        StageDefinition(id="decide", depends_on=("match", "confirm")),
        StageDefinition(
            id="brief",
            depends_on=("decide", "match", "confirm"),
            implemented=True,
            execution_modes=("deterministic",),
            authoritative_outputs=("required_document_plan.json",),
        ),
        StageDefinition(
            id="draft",
            depends_on=("brief", "match", "evidence"),
            implemented=True,
            execution_modes=("host_agent", "configured_provider"),
            authoritative_outputs=(
                "cover_letter_draft.json",
                "research_statement_draft.json",
            ),
        ),
        StageDefinition(
            id="review",
            depends_on=("draft",),
            implemented=True,
            execution_modes=("deterministic",),
            authoritative_outputs=(
                "review_findings.json",
                "research_statement_review_findings.json",
            ),
        ),
        StageDefinition(id="package", depends_on=("review",)),
        StageDefinition(id="verify", depends_on=("package",)),
        StageDefinition(id="render", depends_on=("verify",)),
    )
)
