#!/usr/bin/env python3
"""Run the packaged decision-spine smoke test without echoing private bodies."""

from __future__ import annotations

import argparse
from collections import Counter
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
from typing import Any, Mapping, Sequence

from canisend.draft_models import ClaimKind, stable_claim_id
from canisend.stages.draft_stage import (
    DRAFT_GENERATOR_STRATEGY,
    DRAFT_GENERATOR_VERSION,
    HOST_AGENT_RESEARCH_STATEMENT_GENERATOR_STRATEGY,
    draft_input_fingerprint,
    draft_input_projection,
    research_statement_draft_input_fingerprint,
    research_statement_draft_input_projection,
)


EXAMPLE_JOB = "jobs/2026-06-15_example-university_lecturer-in-applied-economics"
EXPECTED_STAGE_RUN_COUNTS = {
    "evidence": 1,
    "parse": 1,
    "confirm": 1,
    "match": 1,
    "brief": 2,
    "draft": 2,
    "review": 2,
    "package_review": 1,
}
EXPECTED_USER_MUTATION_RECEIPTS = 19
STRUCTURED_DRAFT_SENTINEL = "I hold a PhD in Economics."
RESEARCH_DRAFT_SENTINEL = "My research applies policy-evaluation methods."
USER_AND_STRUCTURED_ARTIFACTS = (
    "parsed_job.json",
    "criteria.json",
    "evidence_catalog.json",
    "criterion_matches.json",
    "confirmed_corrections.yaml",
    "application_decision.yaml",
    "application_brief.yaml",
    "required_document_plan.json",
    "cover_letter_draft.json",
    "research_statement_draft.json",
    "review_findings.json",
    "research_statement_review_findings.json",
    "review_dispositions.yaml",
    "research_statement_review_dispositions.yaml",
    "package_review_findings.json",
    "package_review_dispositions.yaml",
)


class SmokeFailure(RuntimeError):
    """A body-free failure raised by the release smoke test."""


def _run(
    canisend: str,
    arguments: Sequence[str],
    *,
    expect_json: bool,
    expected_returncodes: tuple[int, ...] = (0,),
    environment: Mapping[str, str] | None = None,
) -> dict[str, Any] | None:
    completed = subprocess.run(
        [canisend, *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="strict",
        env=(None if environment is None else {**os.environ, **environment}),
    )
    operation = " ".join(arguments[:2])
    if completed.returncode not in expected_returncodes:
        raise SmokeFailure(
            f"CanISend smoke operation {operation!r} failed "
            f"with exit code {completed.returncode}."
        )
    if not expect_json:
        return None
    try:
        payload = json.loads(completed.stdout)
    except (json.JSONDecodeError, TypeError) as exc:
        raise SmokeFailure(
            f"CanISend smoke operation {operation!r} did not return one JSON value."
        ) from exc
    if not isinstance(payload, dict) or payload.get("protocol") != "canisend.agent/v1":
        raise SmokeFailure(
            f"CanISend smoke operation {operation!r} returned an unexpected contract."
        )
    return payload


def _job_arguments(workspace: Path) -> list[str]:
    return [
        "--workspace",
        str(workspace),
        "--job",
        EXAMPLE_JOB,
        "--format",
        "json",
    ]


def _artifact(payload: dict[str, Any], kind: str) -> dict[str, Any]:
    artifacts = payload.get("artifacts")
    if not isinstance(artifacts, list):
        raise SmokeFailure("A user-mutation response omitted its artifact references.")
    matching = [
        item
        for item in artifacts
        if isinstance(item, dict) and item.get("kind") == kind
    ]
    if len(matching) != 1:
        raise SmokeFailure(f"Expected exactly one {kind!r} artifact reference.")
    return matching[0]


def _job_relative_artifact(payload: dict[str, Any], kind: str) -> str:
    path = _artifact(payload, kind).get("path")
    if not isinstance(path, str):
        raise SmokeFailure(f"The {kind!r} artifact omitted its path.")
    try:
        return (Path(path).relative_to(EXAMPLE_JOB)).as_posix()
    except ValueError as exc:
        raise SmokeFailure(f"The {kind!r} artifact escaped the selected job.") from exc


def _brief_patch(
    canisend: str,
    workspace: Path,
    current: dict[str, Any],
    patch: dict[str, Any],
) -> dict[str, Any]:
    extensions = current.get("extensions")
    if not isinstance(extensions, dict):
        raise SmokeFailure("A Brief response omitted its control metadata.")
    revision = extensions.get("canisend.user_artifact_revision")
    sha256 = _artifact(current, "application_brief").get("sha256")
    if not isinstance(revision, int) or not isinstance(sha256, str):
        raise SmokeFailure("A Brief response omitted its compare-and-swap baseline.")

    patch_path = workspace / ".canisend-smoke-brief-patch.json"
    patch_path.write_text(json.dumps(patch) + "\n", encoding="utf-8")
    try:
        updated = _run(
            canisend,
            [
                "brief",
                "update",
                *_job_arguments(workspace),
                "--patch-file",
                str(patch_path),
                "--expected-revision",
                str(revision),
                "--expected-sha256",
                sha256,
                "--confirm-user-owned-write",
            ],
            expect_json=True,
        )
    finally:
        patch_path.unlink(missing_ok=True)
    if updated is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Brief update returned no response.")
    return updated


def _review_disposition_patch(
    canisend: str,
    workspace: Path,
    document_id: str,
    artifact_kind: str,
    current: dict[str, Any],
    patch: dict[str, Any],
) -> dict[str, Any]:
    extensions = current.get("extensions")
    if not isinstance(extensions, dict):
        raise SmokeFailure("A Review disposition response omitted its control metadata.")
    revision = extensions.get("canisend.user_artifact_revision")
    sha256 = _artifact(current, artifact_kind).get("sha256")
    if not isinstance(revision, int) or not isinstance(sha256, str):
        raise SmokeFailure(
            "A Review disposition response omitted its compare-and-swap baseline."
        )

    patch_path = workspace / ".canisend-smoke-review-disposition-patch.json"
    patch_path.write_text(json.dumps(patch) + "\n", encoding="utf-8")
    try:
        updated = _run(
            canisend,
            [
                "review-dispositions",
                "update",
                *_job_arguments(workspace),
                "--document-id",
                document_id,
                "--patch-file",
                str(patch_path),
                "--expected-revision",
                str(revision),
                "--expected-sha256",
                sha256,
                "--confirm-user-owned-write",
            ],
            expect_json=True,
        )
    finally:
        patch_path.unlink(missing_ok=True)
    if updated is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Review disposition update returned no response.")
    return updated


def _package_review_disposition_patch(
    canisend: str,
    workspace: Path,
    current: dict[str, Any],
    patch: dict[str, Any],
    *,
    expected_returncodes: tuple[int, ...] = (0,),
) -> dict[str, Any]:
    extensions = current.get("extensions")
    if not isinstance(extensions, dict):
        raise SmokeFailure("A package disposition response omitted its control metadata.")
    revision = extensions.get("canisend.user_artifact_revision")
    sha256 = _artifact(current, "package_review_dispositions").get("sha256")
    if not isinstance(revision, int) or not isinstance(sha256, str):
        raise SmokeFailure(
            "A package disposition response omitted its compare-and-swap baseline."
        )

    patch_path = workspace / ".canisend-smoke-package-disposition-patch.json"
    patch_path.write_text(json.dumps(patch) + "\n", encoding="utf-8")
    try:
        updated = _run(
            canisend,
            [
                "package-review",
                "update",
                *_job_arguments(workspace),
                "--patch-file",
                str(patch_path),
                "--expected-revision",
                str(revision),
                "--expected-sha256",
                sha256,
                "--confirm-user-owned-write",
            ],
            expect_json=True,
            expected_returncodes=expected_returncodes,
        )
    finally:
        patch_path.unlink(missing_ok=True)
    if updated is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Package disposition update returned no response.")
    return updated


def _structured_draft_candidate(workspace: Path) -> dict[str, Any]:
    job = workspace / EXAMPLE_JOB
    projection = draft_input_projection(workspace, job)
    fingerprint = draft_input_fingerprint(workspace, job)
    document_id = projection.get("cover_letter_document_id")
    if not isinstance(document_id, str):
        raise SmokeFailure("The Draft projection omitted its Cover Letter document ID.")
    try:
        criteria = json.loads((job / "criteria.json").read_text(encoding="utf-8"))
        evidence = json.loads(
            (job / "evidence_catalog.json").read_text(encoding="utf-8")
        )
        criterion_id = next(
            item["criterion_id"]
            for item in criteria["criteria"]
            if "phd" in item["text"].casefold()
        )
        evidence_id = next(
            item["evidence_id"]
            for item in evidence["items"]
            if item.get("kind") == "education" and "phd" in item["text"].casefold()
        )
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        KeyError,
        StopIteration,
        TypeError,
    ) as exc:
        raise SmokeFailure("The smoke fixture cannot supply one supported Draft claim.") from exc
    if not isinstance(criterion_id, str) or not isinstance(evidence_id, str):
        raise SmokeFailure("The smoke fixture produced invalid Draft references.")

    def claim(
        text: str,
        kind: ClaimKind,
        *,
        criterion_ids: list[str] | None = None,
        evidence_ref_ids: list[str] | None = None,
        job_field_refs: list[str] | None = None,
    ) -> dict[str, Any]:
        return {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind=kind,
                text=text,
            ),
            "text": text,
            "kind": kind,
            "support_strength": "strong" if kind == "factual" else "not_applicable",
            "criterion_ids": criterion_ids or [],
            "evidence_ref_ids": evidence_ref_ids or [],
            "brief_field_refs": [],
            "job_field_refs": job_field_refs or [],
            "blockers": [],
            "review_state": "proposed",
        }

    return {
        "schema_version": "1.0.0",
        "job_id": job.name,
        "document_id": document_id,
        "input_fingerprint": fingerprint,
        "basis": {
            key: projection[key]
            for key in (
                "parsed_job_sha256",
                "criteria_sha256",
                "evidence_catalog_sha256",
                "criterion_matches_sha256",
                "application_decision_sha256",
                "application_brief_sha256",
                "required_document_plan_sha256",
            )
        },
        "generation_mode": "host_agent",
        "generator_strategy": DRAFT_GENERATOR_STRATEGY,
        "generator_version": DRAFT_GENERATOR_VERSION,
        "review_state": "proposed",
        "sections": [
            {
                "section_id": "opening",
                "heading": None,
                "claims": [
                    claim(
                        "I am applying for the advertised role.",
                        "role_context",
                        job_field_refs=["title"],
                    )
                ],
            },
            {
                "section_id": "body",
                "heading": None,
                "claims": [
                    claim(
                        STRUCTURED_DRAFT_SENTINEL,
                        "factual",
                        criterion_ids=[criterion_id],
                        evidence_ref_ids=[evidence_id],
                    )
                ],
            },
            {
                "section_id": "closing",
                "heading": None,
                "claims": [
                    claim("Thank you for considering my application.", "administrative")
                ],
            },
        ],
        "blockers": [],
    }


def _structured_draft_provider_proposal(workspace: Path) -> dict[str, Any]:
    candidate = _structured_draft_candidate(workspace)
    sections = candidate.get("sections")
    if not isinstance(sections, list):
        raise SmokeFailure("The smoke Draft candidate omitted its sections.")
    proposal_sections: list[dict[str, Any]] = []
    claim_keys = (
        "text",
        "kind",
        "support_strength",
        "criterion_ids",
        "evidence_ref_ids",
        "brief_field_refs",
        "job_field_refs",
        "blockers",
    )
    for section in sections:
        if not isinstance(section, dict) or not isinstance(section.get("claims"), list):
            raise SmokeFailure("The smoke Draft candidate contains an invalid section.")
        proposal_sections.append(
            {
                "section_id": section.get("section_id"),
                "claims": [
                    {key: claim.get(key) for key in claim_keys}
                    for claim in section["claims"]
                    if isinstance(claim, dict)
                ],
            }
        )
    return {"sections": proposal_sections}


def _research_statement_candidate(workspace: Path) -> dict[str, Any]:
    job = workspace / EXAMPLE_JOB
    projection = research_statement_draft_input_projection(workspace, job)
    fingerprint = research_statement_draft_input_fingerprint(workspace, job)
    document_id = projection.get("research_statement_document_id")
    if not isinstance(document_id, str):
        raise SmokeFailure(
            "The Draft projection omitted its Research Statement document ID."
        )
    try:
        criteria = json.loads((job / "criteria.json").read_text(encoding="utf-8"))
        evidence = json.loads(
            (job / "evidence_catalog.json").read_text(encoding="utf-8")
        )
        criterion_id = criteria["criteria"][0]["criterion_id"]
        evidence_id = evidence["items"][0]["evidence_id"]
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        IndexError,
        KeyError,
        TypeError,
    ) as exc:
        raise SmokeFailure(
            "The smoke fixture cannot supply Research Statement references."
        ) from exc
    if not isinstance(criterion_id, str) or not isinstance(evidence_id, str):
        raise SmokeFailure("The smoke fixture produced invalid Research references.")

    def claim(text: str, kind: ClaimKind) -> dict[str, Any]:
        factual = kind == "factual"
        return {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind=kind,
                text=text,
            ),
            "text": text,
            "kind": kind,
            "support_strength": "strong" if factual else "not_applicable",
            "criterion_ids": [criterion_id],
            "evidence_ref_ids": [evidence_id] if factual else [],
            "brief_field_refs": [],
            "job_field_refs": [],
            "blockers": [],
            "review_state": "proposed",
        }

    return {
        "schema_version": "1.0.0",
        "job_id": job.name,
        "document_id": document_id,
        "input_fingerprint": fingerprint,
        "basis": {
            key: projection[key]
            for key in (
                "parsed_job_sha256",
                "criteria_sha256",
                "evidence_catalog_sha256",
                "criterion_matches_sha256",
                "application_decision_sha256",
                "application_brief_sha256",
                "required_document_plan_sha256",
            )
        },
        "generation_mode": "host_agent",
        "generator_strategy": HOST_AGENT_RESEARCH_STATEMENT_GENERATOR_STRATEGY,
        "generator_version": DRAFT_GENERATOR_VERSION,
        "review_state": "proposed",
        "sections": [
            {
                "section_id": "research_overview",
                "heading": None,
                "claims": [claim(RESEARCH_DRAFT_SENTINEL, "factual")],
            },
            {
                "section_id": "research_contributions",
                "heading": None,
                "claims": [
                    claim(
                        "My current work develops applied empirical contributions.",
                        "factual",
                    )
                ],
            },
            {
                "section_id": "future_agenda",
                "heading": None,
                "claims": [
                    claim(
                        "I will extend this agenda to new policy settings.",
                        "future_intent",
                    )
                ],
            },
        ],
        "blockers": [],
    }


def _promote_host_candidate(
    canisend: str,
    workspace: Path,
    *,
    document_id: str,
    candidate: dict[str, Any],
) -> None:
    prepared = _run(
        canisend,
        [
            "stage",
            "prepare",
            *_job_arguments(workspace),
            "--stage", "draft",
            "--document-id",
            document_id,
            "--mode",
            "host-agent",
        ],
        expect_json=True,
    )
    if prepared is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Research Statement Draft preparation returned no response.")
    task_workspace_path = _artifact(prepared, "stage_task_spec").get("path")
    if not isinstance(task_workspace_path, str):
        raise SmokeFailure("Research Statement Draft omitted its TaskSpec path.")
    try:
        task_job_path = Path(task_workspace_path).relative_to(EXAMPLE_JOB).as_posix()
        task_spec = json.loads(
            (workspace / task_workspace_path).read_text(encoding="utf-8")
        )
        result_path = task_spec["result_output"]
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, ValueError) as exc:
        raise SmokeFailure("Research Statement Draft TaskSpec is invalid.") from exc
    if not isinstance(result_path, str):
        raise SmokeFailure("Research Statement Draft TaskSpec omitted its result path.")

    candidate_path = workspace / ".canisend-smoke-research-candidate.json"
    candidate_path.write_text(json.dumps(candidate) + "\n", encoding="utf-8")
    try:
        _run(
            canisend,
            [
                "stage",
                "submit",
                *_job_arguments(workspace),
                "--task",
                task_job_path,
                "--candidate-file",
                str(candidate_path),
            ],
            expect_json=True,
        )
    finally:
        candidate_path.unlink(missing_ok=True)
    applied = _run(
        canisend,
        [
            "stage",
            "apply",
            *_job_arguments(workspace),
            "--task",
            task_job_path,
            "--result",
            result_path,
        ],
        expect_json=True,
    )
    if (
        applied is None
        or not isinstance(applied.get("extensions"), dict)
        or applied["extensions"].get("canisend.document_id") != document_id
    ):
        raise SmokeFailure("Research Statement Draft promotion lost document identity.")


def _assert_workspace_contract(workspace: Path) -> None:
    job = workspace / EXAMPLE_JOB
    expected_artifacts = {
        *USER_AND_STRUCTURED_ARTIFACTS,
        "02_fit_report.md",
        "03_cover_letter_draft.md",
        "05_criteria_checklist.md",
        "07_material_review_checklist.md",
        "08_research_statement.md",
        "typst/cover_letter_content.json",
        "typst/cover_letter.typ",
        "typst/research_statement_content.json",
        "typst/research_statement.typ",
        "typst/application_package_content.json",
        "typst/application_package.typ",
        "application_gate_report.json",
    }
    missing = sorted(name for name in expected_artifacts if not (job / name).is_file())
    if missing:
        raise SmokeFailure("The decision-spine smoke test did not create every expected artifact.")

    receipt_paths = sorted(
        (job / "workflow" / "user-mutations" / "events").glob("*/receipt.json")
    )
    if len(receipt_paths) != EXPECTED_USER_MUTATION_RECEIPTS:
        raise SmokeFailure(
            "The decision-spine smoke test created an unexpected mutation receipt count."
        )

    manifest_paths = sorted((job / "workflow" / "runs").glob("*/manifest.json"))
    if len(manifest_paths) != sum(EXPECTED_STAGE_RUN_COUNTS.values()):
        raise SmokeFailure("The decision-spine smoke test created an unexpected run count.")
    stage_counts: Counter[str] = Counter()
    draft_modes: Counter[str] = Counter()
    for manifest_path in manifest_paths:
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as exc:
            raise SmokeFailure("A decision-spine run manifest could not be validated.") from exc
        stage = manifest.get("stage") if isinstance(manifest, dict) else None
        if not isinstance(stage, str) or manifest.get("status") != "succeeded":
            raise SmokeFailure("A decision-spine stage did not finish successfully.")
        stage_counts[stage] += 1
        if stage == "draft":
            execution_mode = manifest.get("execution_mode")
            if not isinstance(execution_mode, str):
                raise SmokeFailure("A Draft manifest omitted its execution mode.")
            draft_modes[execution_mode] += 1
        if not (manifest_path.parent / "preparation.json").is_file() or not (
            manifest_path.parent / "submission.json"
        ).is_file():
            raise SmokeFailure("A decision-spine run omitted its preparation or submission record.")
    if dict(stage_counts) != EXPECTED_STAGE_RUN_COUNTS:
        raise SmokeFailure("The decision-spine smoke test ran unexpected stage counts.")
    if draft_modes != Counter({"configured_provider": 1, "host_agent": 1}):
        raise SmokeFailure("The release smoke did not exercise both Draft modes.")

    try:
        plan = json.loads((job / "required_document_plan.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise SmokeFailure("The required-document plan is not valid JSON.") from exc
    if not isinstance(plan, dict) or plan.get("job_id") != job.name:
        raise SmokeFailure("The required-document plan does not belong to the example job.")
    requirements = plan.get("requirements")
    if (
        not isinstance(requirements, list)
        or not requirements
        or any(
            not isinstance(item, dict) or item.get("source_state") != "known"
            for item in requirements
        )
    ):
        raise SmokeFailure(
            "The required-document plan did not retain resolvable advert source receipts."
        )
    if plan.get("requirements_state") != "confirmed" or plan.get("blockers") != []:
        raise SmokeFailure(
            "The smoke plan did not retain the confirmed blocker-free document set."
        )

    fit_report = (job / "02_fit_report.md").read_text(encoding="utf-8")
    checklist = (job / "05_criteria_checklist.md").read_text(encoding="utf-8")
    material_review = (job / "07_material_review_checklist.md").read_text(
        encoding="utf-8"
    )
    package_content = json.loads(
        (job / "typst" / "application_package_content.json").read_text(
            encoding="utf-8"
        )
    )
    cover_content = json.loads(
        (job / "typst" / "cover_letter_content.json").read_text(encoding="utf-8")
    )
    research_content = json.loads(
        (job / "typst" / "research_statement_content.json").read_text(
            encoding="utf-8"
        )
    )
    draft_markdown = (job / "03_cover_letter_draft.md").read_text(encoding="utf-8")
    research_markdown = (job / "08_research_statement.md").read_text(
        encoding="utf-8"
    )
    cover_source = (job / "typst" / "cover_letter.typ").read_text(encoding="utf-8")
    research_source = (job / "typst" / "research_statement.typ").read_text(
        encoding="utf-8"
    )
    package_source = (job / "typst" / "application_package.typ").read_text(
        encoding="utf-8"
    )
    if "Deterministic proposal" not in fit_report or "(PROPOSED)" not in fit_report:
        raise SmokeFailure("The installed wheel did not render the current structured Match view.")
    if "Deterministic Match proposals only" not in checklist:
        raise SmokeFailure("The installed wheel did not render the structured criteria checklist.")
    if "Criterion is unresolved" not in material_review:
        raise SmokeFailure("The installed wheel did not fail closed on unresolved HR criteria.")
    if not isinstance(package_content, dict) or (
        package_content.get("fit_report") != fit_report
        or package_content.get("criteria_checklist") != checklist
    ):
        raise SmokeFailure("The installed-wheel Markdown and Typst content projections diverged.")
    if draft_markdown.count(STRUCTURED_DRAFT_SENTINEL) != 1:
        raise SmokeFailure("The installed wheel did not project the structured Draft to Markdown.")
    projection = cover_content.get("projection") if isinstance(cover_content, dict) else None
    if (
        not isinstance(projection, dict)
        or projection.get("source") != "cover_letter_draft.json"
        or projection.get("blocker_count") != 0
        or projection.get("document_readiness_state") != "reviewed"
        or projection.get("requires_human_review") is not False
    ):
        raise SmokeFailure("The installed wheel lost structured Draft review provenance.")
    if package_content.get("cover_letter_projection") != projection:
        raise SmokeFailure("The installed-wheel package projection lost Draft provenance.")
    for source in (cover_source, package_source):
        if source.count(STRUCTURED_DRAFT_SENTINEL) != 1:
            raise SmokeFailure("The installed-wheel Typst projection duplicated or lost a Claim.")
    if "// CANISEND: structured-draft projection" not in cover_source:
        raise SmokeFailure("The installed-wheel Cover Letter omitted its structured marker.")
    research_projection = (
        research_content.get("projection")
        if isinstance(research_content, dict)
        else None
    )
    if (
        research_markdown.count(RESEARCH_DRAFT_SENTINEL) != 1
        or research_source.count(RESEARCH_DRAFT_SENTINEL) != 1
        or not isinstance(research_projection, dict)
        or research_projection.get("source")
        != "research_statement_draft.json"
        or research_projection.get("integration_scope")
        != "standalone_document"
        or research_projection.get("document_readiness_state") != "reviewed"
        or research_projection.get("requires_human_review") is not False
        or "// CANISEND: structured-research-statement projection"
        not in research_source
    ):
        raise SmokeFailure(
            "The installed wheel lost the reviewed Research Statement projection."
        )
    if (
        "research_statement_projection" in package_content
        or "structured_research_statement_sections" in package_content
        or RESEARCH_DRAFT_SENTINEL in package_source
    ):
        raise SmokeFailure(
            "The standalone Research Statement leaked into the application package."
        )

    gate_report = json.loads(
        (job / "application_gate_report.json").read_text(encoding="utf-8")
    )
    gate_issues = gate_report.get("issues") if isinstance(gate_report, dict) else None
    if (
        gate_report.get("status") != "FAIL"
        or not isinstance(gate_issues, list)
        or any(
            isinstance(issue, dict)
            and "not document readiness"
            in str(issue.get("message", ""))
            for issue in gate_issues
        )
        or not any(
            isinstance(issue, dict)
            and "material review contains an explicit BLOCKER"
            in str(issue.get("message", ""))
            for issue in gate_issues
        )
        or not any(
            isinstance(issue, dict)
            and issue.get("gate") == "APP-Q5"
            and "unresolved required-document or aggregate blockers"
            in str(issue.get("message", ""))
            for issue in gate_issues
        )
    ):
        raise SmokeFailure(
            "The installed wheel did not separate document readiness from package readiness."
        )
    input_hashes = gate_report.get("input_hashes")
    if not isinstance(input_hashes, dict) or not {
        "job/cover_letter_draft.json",
        "job/review_findings.json",
        "job/review_dispositions.yaml",
        "job/package_review_findings.json",
        "job/package_review_dispositions.yaml",
    }.issubset(input_hashes):
        raise SmokeFailure("The package gate did not bind the structured Draft and Review inputs.")
    if any(
        key.startswith("job/") and "research_statement" in key
        for key in input_hashes
    ):
        raise SmokeFailure(
            "The package gate incorrectly absorbed the standalone Research Statement."
        )


def run_smoke(canisend: str, workspace: Path) -> None:
    workspace = workspace.resolve()
    if workspace.exists():
        raise SmokeFailure("The smoke workspace must not already exist.")
    workspace.parent.mkdir(parents=True, exist_ok=True)

    _run(canisend, ["--help"], expect_json=False)
    _run(
        canisend,
        ["run-example", "--workspace", str(workspace), "--overwrite"],
        expect_json=False,
    )
    _run(
        canisend,
        ["doctor", "--workspace", str(workspace), "--format", "json"],
        expect_json=False,
    )
    _run(canisend, ["agent", "capabilities", "--format", "json"], expect_json=True)

    job_args = _job_arguments(workspace)
    for stage in ("evidence", "parse", "confirm"):
        _run(
            canisend,
            ["stage", "run", *job_args, "--stage", stage],
            expect_json=True,
        )

    _run(canisend, ["corrections", "status", *job_args], expect_json=True)
    _run(
        canisend,
        ["corrections", "init", *job_args, "--confirm-user-owned-write"],
        expect_json=True,
    )
    _run(
        canisend,
        ["stage", "run", *job_args, "--stage", "match"],
        expect_json=True,
    )

    _run(canisend, ["decision", "status", *job_args], expect_json=True)
    initialized = _run(
        canisend,
        ["decision", "init", *job_args, "--confirm-user-owned-write"],
        expect_json=True,
    )
    if initialized is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Decision initialization returned no response.")
    decision = _artifact(initialized, "application_decision")
    extensions = initialized.get("extensions")
    if not isinstance(extensions, dict):
        raise SmokeFailure("Decision initialization omitted its control metadata.")
    revision = extensions.get("canisend.user_artifact_revision")
    sha256 = decision.get("sha256")
    if not isinstance(revision, int) or not isinstance(sha256, str):
        raise SmokeFailure("Decision initialization omitted its compare-and-swap baseline.")

    patch_path = workspace / ".canisend-smoke-decision-patch.json"
    patch_path.write_text(
        json.dumps({"operation": "set_decision", "decision": "apply"}) + "\n",
        encoding="utf-8",
    )
    try:
        _run(
            canisend,
            [
                "decision",
                "update",
                *job_args,
                "--patch-file",
                str(patch_path),
                "--expected-revision",
                str(revision),
                "--expected-sha256",
                sha256,
                "--confirm-user-owned-write",
            ],
            expect_json=True,
        )
    finally:
        patch_path.unlink(missing_ok=True)

    _run(canisend, ["brief", "status", *job_args], expect_json=True)
    brief_initialized = _run(
        canisend,
        ["brief", "init", *job_args, "--confirm-user-owned-write"],
        expect_json=True,
    )
    if brief_initialized is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Brief initialization returned no response.")
    _run(
        canisend,
        ["stage", "run", *job_args, "--stage", "brief"],
        expect_json=True,
    )
    job = workspace / EXAMPLE_JOB
    try:
        initial_plan = json.loads(
            (job / "required_document_plan.json").read_text(encoding="utf-8")
        )
        requirements_basis = initial_plan["requirements_basis_sha256"]
        cover_document_id = next(
            item["document_id"]
            for item in initial_plan["requirements"]
            if item["normalized_kind"] == "cover_letter"
        )
        research_document_id = next(
            item["document_id"]
            for item in initial_plan["requirements"]
            if item["normalized_kind"] == "research_statement"
        )
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        KeyError,
        StopIteration,
        TypeError,
    ) as exc:
        raise SmokeFailure("The initial document plan omitted its requirements basis.") from exc
    if not isinstance(requirements_basis, str):
        raise SmokeFailure("The initial document requirements basis is invalid.")
    if not isinstance(cover_document_id, str):
        raise SmokeFailure("The initial document plan omitted its Cover Letter identity.")
    if not isinstance(research_document_id, str):
        raise SmokeFailure("The initial document plan omitted its Research identity.")

    current_brief = brief_initialized
    for patch in (
        {
            "operation": "confirm_document_requirements",
            "state": "confirmed",
            "requirements_basis_sha256": requirements_basis,
        },
        {"operation": "set_brief_language", "value": "uk"},
        {
            "operation": "set_brief_text",
            "field": "writing_style",
            "value": "direct and evidence-led",
        },
        {
            "operation": "set_brief_text",
            "field": "motivation",
            "value": "Contribute to the advertised teaching and research priorities.",
        },
        {
            "operation": "set_brief_emphasis",
            "criterion_ids": [],
            "evidence_ref_ids": [],
        },
        {"operation": "set_brief_exclusions", "items": []},
    ):
        current_brief = _brief_patch(canisend, workspace, current_brief, patch)

    _run(
        canisend,
        ["stage", "run", *job_args, "--stage", "brief"],
        expect_json=True,
    )

    denied = _run(
        canisend,
        [
            "stage",
            "run",
            *job_args,
            "--stage",
            "draft",
            "--document-id",
            cover_document_id,
            "--mode",
            "configured-provider",
        ],
        expect_json=True,
        expected_returncodes=(1,),
    )
    if (
        denied is None
        or not isinstance(denied.get("error"), dict)
        or denied["error"].get("code") != "stage.provider_consent_required"
    ):
        raise SmokeFailure("Configured-provider Draft did not enforce Tier 3 consent.")

    provider_path = workspace / ".canisend-smoke-structured-provider.py"
    provider_path.write_text(
        "print("
        + repr(json.dumps(_structured_draft_provider_proposal(workspace)))
        + ")\n",
        encoding="utf-8",
    )
    try:
        drafted = _run(
            canisend,
            [
                "stage",
                "run",
                *job_args,
                "--stage",
                "draft",
                "--document-id",
                cover_document_id,
                "--mode",
                "configured-provider",
                "--allow-provider-backed",
            ],
            expect_json=True,
            environment={
                "ACADEMIC_PREP_LLM_PROVIDER": "command",
                "ACADEMIC_PREP_LLM_COMMAND": shlex.join(
                    [sys.executable, str(provider_path)]
                ),
            },
        )
    finally:
        provider_path.unlink(missing_ok=True)
    if (
        drafted is None
        or not isinstance(drafted.get("extensions"), dict)
        or drafted["extensions"].get("canisend.execution_mode")
        != "configured_provider"
    ):
        raise SmokeFailure("Configured-provider Draft omitted its execution receipt.")
    reviewed = _run(
        canisend,
        [
            "stage",
            "run",
            *job_args,
            "--stage", "review",
            "--document-id",
            cover_document_id,
        ],
        expect_json=True,
    )
    if reviewed is None or reviewed.get("blockers") != []:
        raise SmokeFailure("The deterministic Review did not produce a blocker-free projection.")

    _run(
        canisend,
        ["review-dispositions", "status", *job_args, "--document-id", cover_document_id],
        expect_json=True,
    )
    current_dispositions = _run(
        canisend,
        [
            "review-dispositions",
            "init",
            *job_args,
            "--document-id",
            cover_document_id,
            "--confirm-user-owned-write",
        ],
        expect_json=True,
    )
    if current_dispositions is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Review disposition initialization returned no response.")
    try:
        review_payload = json.loads(
            (job / "review_findings.json").read_text(encoding="utf-8")
        )
        finding_ids = [item["finding_id"] for item in review_payload["findings"]]
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise SmokeFailure("The deterministic Review omitted stable finding IDs.") from exc
    for finding_id in finding_ids:
        if not isinstance(finding_id, str):
            raise SmokeFailure("The deterministic Review emitted an invalid finding ID.")
        current_dispositions = _review_disposition_patch(
            canisend,
            workspace,
            cover_document_id,
            "review_dispositions",
            current_dispositions,
            {
                "operation": "set_finding_disposition",
                "finding_id": finding_id,
                "disposition": "accepted",
            },
        )
    extensions = current_dispositions.get("extensions")
    if not isinstance(extensions, dict) or extensions.get(
        "canisend.document_readiness"
    ) != "reviewed":
        raise SmokeFailure("Complete user dispositions did not review the Cover Letter.")

    _promote_host_candidate(
        canisend,
        workspace,
        document_id=research_document_id,
        candidate=_research_statement_candidate(workspace),
    )
    research_reviewed = _run(
        canisend,
        [
            "stage",
            "run",
            *job_args,
            "--stage",
            "review",
            "--document-id",
            research_document_id,
        ],
        expect_json=True,
    )
    if research_reviewed is None or research_reviewed.get("blockers") != []:
        raise SmokeFailure("Research Statement Review was not blocker-free.")

    ambiguous = _run(
        canisend,
        ["review-dispositions", "status", *job_args],
        expect_json=True,
        expected_returncodes=(1,),
    )
    if (
        ambiguous is None
        or not isinstance(ambiguous.get("error"), dict)
        or ambiguous["error"].get("code") != "user_input.document_ambiguous"
    ):
        raise SmokeFailure("Review disposition selection did not fail closed.")

    research_dispositions = _run(
        canisend,
        [
            "review-dispositions",
            "init",
            *job_args,
            "--document-id",
            research_document_id,
            "--confirm-user-owned-write",
        ],
        expect_json=True,
    )
    if research_dispositions is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Research disposition initialization returned no response.")
    try:
        research_review = json.loads(
            (job / "research_statement_review_findings.json").read_text(
                encoding="utf-8"
            )
        )
        research_finding_ids = [
            item["finding_id"] for item in research_review["findings"]
        ]
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise SmokeFailure("Research Review omitted stable finding IDs.") from exc
    for finding_id in research_finding_ids:
        if not isinstance(finding_id, str):
            raise SmokeFailure("Research Review emitted an invalid finding ID.")
        research_dispositions = _review_disposition_patch(
            canisend,
            workspace,
            research_document_id,
            "research_statement_review_dispositions",
            research_dispositions,
            {
                "operation": "set_finding_disposition",
                "finding_id": finding_id,
                "disposition": "accepted",
            },
        )
    research_extensions = research_dispositions.get("extensions")
    if (
        not isinstance(research_extensions, dict)
        or research_extensions.get("canisend.document_readiness") != "reviewed"
        or research_extensions.get("canisend.document_kind")
        != "research_statement"
    ):
        raise SmokeFailure(
            "Complete user dispositions did not review the Research Statement."
        )

    package_review = _run(
        canisend,
        ["stage", "run", *job_args, "--stage", "package_review"],
        expect_json=True,
    )
    package_extensions = (
        package_review.get("extensions")
        if isinstance(package_review, dict)
        else None
    )
    package_blockers = (
        package_extensions.get("canisend.package_review_blocker_count")
        if isinstance(package_extensions, dict)
        else None
    )
    if not isinstance(package_blockers, int) or package_blockers < 1:
        raise SmokeFailure(
            "Aggregate Package Review did not preserve the unsupported required-document blocker."
        )

    _run(
        canisend,
        ["package-review", "status", *job_args],
        expect_json=True,
    )
    package_dispositions = _run(
        canisend,
        [
            "package-review",
            "init",
            *job_args,
            "--confirm-user-owned-write",
        ],
        expect_json=True,
    )
    if package_dispositions is None:  # pragma: no cover - guarded by expect_json
        raise SmokeFailure("Package disposition initialization returned no response.")
    try:
        package_review_payload = json.loads(
            (job / "package_review_findings.json").read_text(encoding="utf-8")
        )
        package_blocker_id = package_review_payload["blocker_finding_ids"][0]
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        IndexError,
        KeyError,
        TypeError,
    ) as exc:
        raise SmokeFailure("Package Review omitted its non-waivable blocker ID.") from exc
    rejected = _package_review_disposition_patch(
        canisend,
        workspace,
        package_dispositions,
        {
            "operation": "set_package_finding_disposition",
            "finding_id": package_blocker_id,
            "disposition": "accepted",
        },
        expected_returncodes=(1,),
    )
    if (
        not isinstance(rejected.get("error"), dict)
        or rejected["error"].get("code") != "user_input.invalid"
    ):
        raise SmokeFailure("Package disposition incorrectly waived an aggregate blocker.")
    package_status = _run(
        canisend,
        ["package-review", "status", *job_args],
        expect_json=True,
    )
    package_status_extensions = (
        package_status.get("extensions")
        if isinstance(package_status, dict)
        else None
    )
    if (
        not isinstance(package_status_extensions, dict)
        or package_status_extensions.get("canisend.application_package_readiness")
        != "blocked"
    ):
        raise SmokeFailure("Package readiness did not remain blocked after rejection.")

    before_run = {
        name: (job / name).read_bytes()
        for name in USER_AND_STRUCTURED_ARTIFACTS
    }
    _run(
        canisend,
        [
            "run",
            "--workspace",
            str(workspace),
            "--job",
            EXAMPLE_JOB,
            "--no-git-add-materials",
        ],
        expect_json=False,
    )
    if any(
        (job / name).read_bytes() != content
        for name, content in before_run.items()
    ):
        raise SmokeFailure("The compatible pipeline rewrote a Decision Spine artifact.")
    _run(
        canisend,
        [
            "check-package",
            "--workspace",
            str(workspace),
            "--job",
            EXAMPLE_JOB,
            "--write-report",
            "--format",
            "json",
        ],
        expect_json=True,
        expected_returncodes=(1,),
    )
    _assert_workspace_contract(workspace)


def _parse_args(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--canisend",
        required=True,
        help="CanISend console executable to test.",
    )
    parser.add_argument(
        "--workspace",
        required=True,
        type=Path,
        help="Fresh workspace path used for the smoke run.",
    )
    return parser.parse_args(arguments)


def main(arguments: Sequence[str] | None = None) -> int:
    args = _parse_args(arguments)
    try:
        run_smoke(args.canisend, args.workspace)
    except (OSError, UnicodeError, SmokeFailure) as exc:
        print(f"decision-spine smoke failed: {exc}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "status": "ok",
                "successful_stage_count": sum(EXPECTED_STAGE_RUN_COUNTS.values()),
                "mutation_receipt_count": EXPECTED_USER_MUTATION_RECEIPTS,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
