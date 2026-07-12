from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path

import pytest
import yaml

import canisend.stages.brief_stage as brief_stage_module
from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    ConfirmedIdSelectionV1,
    ConfirmedStringListV1,
    ConfirmedTextV1,
    DecisionBasisV1,
    DocumentChoiceV1,
    DocumentRequirementsConfirmationV1,
    LanguagePreferenceV1,
)
from canisend.parse import parse_job_advert
from canisend.stage_store import sha256_file
from canisend.stages.brief_stage import (
    BriefStageError,
    BriefStageValidationError,
    build_deterministic_brief_candidate,
    brief_input_fingerprint,
    canonical_document_kind,
    document_requirements_basis_sha256,
    stable_document_id,
    validate_brief_candidate,
)


NOW = "2026-07-12T10:00:00Z"


def _write_job(
    tmp_path: Path,
    *,
    required_documents: list[str] | None = None,
    advert: str | None = None,
) -> tuple[Path, Path, dict[str, object], str]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    advert_text = advert or """# Lecturer in Economics

Required documents: CV, Cover letter, Optional: Research statement

Essential criteria:
- PhD in Economics
"""
    metadata = {
        "title": "Lecturer in Economics",
        "institution": "Example University",
        "department": "Economics",
        "location": "London",
        "deadline": "2026-08-01",
        "source_url": "https://example.edu/job",
    }
    parsed = parse_job_advert(advert_text, metadata)
    if required_documents is not None:
        parsed["required_documents"] = required_documents
    (job / "job_advert.md").write_text(advert_text, encoding="utf-8")
    (job / "parsed_job.json").write_text(
        json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    # Brief runtime dependencies are validated separately. Unit projection still
    # binds their exact current bytes.
    (job / "criteria.json").write_text("{}\n", encoding="utf-8")
    (job / "criterion_matches.json").write_text("{}\n", encoding="utf-8")
    decision = ApplicationDecisionV1(
        job_id=job.name,
        revision=1,
        updated_at=NOW,
        decision="apply",
        confirmation_state="confirmed",
        confirmed_at=NOW,
        basis=DecisionBasisV1(
            criteria_sha256=sha256_file(job / "criteria.json"),
            matches_sha256=sha256_file(job / "criterion_matches.json"),
            status="current",
        ),
    )
    _write_yaml(job / "application_decision.yaml", decision)
    return workspace, job, parsed, advert_text


def _write_brief(
    job: Path,
    *,
    confirmation: DocumentRequirementsConfirmationV1 | None = None,
    choices: tuple[DocumentChoiceV1, ...] = (),
    confirm_fields: bool = False,
    private_marker: str | None = None,
) -> ApplicationBriefV1:
    confirmed = "confirmed" if confirm_fields else "unconfirmed"
    brief = ApplicationBriefV1(
        job_id=job.name,
        revision=0,
        updated_at=NOW,
        decision_sha256=sha256_file(job / "application_decision.yaml"),
        language=LanguagePreferenceV1(
            value="uk" if confirm_fields else None,
            confirmation_state=confirmed,
        ),
        writing_style=ConfirmedTextV1(
            value="direct" if confirm_fields else None,
            confirmation_state=confirmed,
        ),
        motivation=ConfirmedTextV1(
            value=private_marker or "" if confirm_fields else None,
            confirmation_state=confirmed,
        ),
        emphasis=ConfirmedIdSelectionV1(confirmation_state=confirmed),
        exclusions=ConfirmedStringListV1(
            items=((private_marker,) if private_marker and confirm_fields else ()),
            confirmation_state=confirmed,
        ),
        document_requirements_confirmation=(
            confirmation or DocumentRequirementsConfirmationV1()
        ),
        document_choices=choices,
    )
    _write_yaml(job / "application_brief.yaml", brief)
    return brief


def _write_yaml(path: Path, model: object) -> None:
    path.write_text(
        yaml.safe_dump(model.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )


def _candidate(workspace: Path, job: Path):
    fingerprint = brief_input_fingerprint(workspace, job)
    return build_deterministic_brief_candidate(
        workspace,
        job,
        input_fingerprint=fingerprint,
    )


def test_document_identity_and_confirmation_basis_ignore_order_and_unrelated_lines(
    tmp_path: Path,
) -> None:
    workspace, job, parsed, advert = _write_job(
        tmp_path,
        required_documents=["Cover letter", "CV"],
        advert=(
            "# Lecturer in Economics\n\nRequired documents: CV, Cover letter\n\n"
            "Essential criteria:\n- PhD in Economics\n"
        ),
    )
    basis = document_requirements_basis_sha256(parsed, advert)
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=basis,
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )
    first = _candidate(workspace, job)
    first_ids = {
        item.normalized_kind: item.document_id for item in first.requirements
    }

    parsed["required_documents"] = ["CV", "Cover letter"]
    (job / "parsed_job.json").write_text(json.dumps(parsed) + "\n", encoding="utf-8")
    changed_advert = advert.replace(
        "Essential criteria:",
        "An unrelated contextual line.\n\nEssential criteria:",
    )
    (job / "job_advert.md").write_text(changed_advert, encoding="utf-8")

    assert document_requirements_basis_sha256(parsed, changed_advert) == basis
    second = _candidate(workspace, job)
    assert second.requirements_state == "confirmed"
    assert {item.normalized_kind: item.document_id for item in second.requirements} == first_ids
    assert second.input_fingerprint != first.input_fingerprint
    assert [item.document_id for item in second.requirements] == sorted(first_ids.values())


def test_unconfirmed_requirements_create_only_unresolved_blocking_tasks(
    tmp_path: Path,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV", "Cover letter"],
    )
    _write_brief(job)

    plan = _candidate(workspace, job)

    assert plan.requirements_state == "unconfirmed"
    assert "documents.requirements_unconfirmed" in plan.blockers
    assert plan.unresolved_document_ids == tuple(
        item.document_id for item in plan.requirements
    )
    assert all(item.confirmation_state == "unconfirmed" for item in plan.requirements)
    assert all(task.action == "needs_confirmation" for task in plan.tasks)
    assert set(plan.blocking_document_ids) == set(plan.unresolved_document_ids)


def test_confirmed_requirements_default_to_prepare_and_required_omit_blocks(
    tmp_path: Path,
) -> None:
    workspace, job, parsed, advert = _write_job(
        tmp_path,
        required_documents=["CV", "Optional: Research statement"],
        advert=(
            "# Lecturer in Economics\n\n"
            "Required documents: CV, Optional: Research statement\n\n"
            "Essential criteria:\n- PhD in Economics\n"
        ),
    )
    basis = document_requirements_basis_sha256(parsed, advert)
    confirmation = DocumentRequirementsConfirmationV1(
        state="confirmed",
        basis_sha256=basis,
        confirmed_at=NOW,
    )
    _write_brief(job, confirmation=confirmation, confirm_fields=True)
    initial = _candidate(workspace, job)
    ids = {item.normalized_kind: item.document_id for item in initial.requirements}
    orphan = "document_" + "f" * 32
    _write_brief(
        job,
        confirmation=confirmation,
        confirm_fields=True,
        choices=(
            DocumentChoiceV1(
                document_id=ids["cv"],
                action="omit",
                confirmation_state="confirmed",
            ),
            DocumentChoiceV1(
                document_id=ids["research_statement"],
                action="prepare",
                confirmation_state="confirmed",
            ),
            DocumentChoiceV1(
                document_id=orphan,
                action="prepare",
                confirmation_state="confirmed",
            ),
        ),
    )

    plan = _candidate(workspace, job)
    tasks = {item.document_id: item for item in plan.tasks}

    assert tasks[ids["cv"]].action == "omit"
    assert tasks[ids["cv"]].blockers == ("documents.required_omitted",)
    assert tasks[ids["research_statement"]].action == "prepare"
    assert not tasks[ids["research_statement"]].blockers
    assert plan.blocking_document_ids == (ids["cv"],)
    assert plan.orphaned_document_choice_ids == (orphan,)
    assert "documents.choice_orphaned" in plan.blockers


def test_empty_required_documents_need_current_confirmed_empty_basis(
    tmp_path: Path,
) -> None:
    workspace, job, parsed, advert = _write_job(tmp_path, required_documents=[])
    _write_brief(job, confirm_fields=True)

    unresolved = _candidate(workspace, job)
    assert unresolved.requirements_state == "unconfirmed"
    assert unresolved.requirements == ()
    assert "documents.requirements_unconfirmed" in unresolved.blockers

    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed_empty",
            basis_sha256=document_requirements_basis_sha256(parsed, advert),
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )
    confirmed = _candidate(workspace, job)
    assert confirmed.requirements_state == "confirmed_empty"
    assert confirmed.requirements == ()
    assert confirmed.tasks == ()
    assert confirmed.blockers == ()


def test_manual_needs_confirmation_choice_remains_an_executable_blocker(
    tmp_path: Path,
) -> None:
    workspace, job, parsed, advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=(
            "# Lecturer in Economics\n\nRequired documents: CV\n\n"
            "Essential criteria:\n- PhD in Economics\n"
        ),
    )
    confirmation = DocumentRequirementsConfirmationV1(
        state="confirmed",
        basis_sha256=document_requirements_basis_sha256(parsed, advert),
        confirmed_at=NOW,
    )
    _write_brief(job, confirmation=confirmation, confirm_fields=True)
    document_id = _candidate(workspace, job).requirements[0].document_id
    _write_brief(
        job,
        confirmation=confirmation,
        confirm_fields=True,
        choices=(DocumentChoiceV1(document_id=document_id),),
    )

    plan = _candidate(workspace, job)

    assert plan.tasks[0].action == "needs_confirmation"
    assert plan.tasks[0].blockers == ("documents.choice_unconfirmed",)
    assert plan.unresolved_document_ids == (document_id,)
    assert plan.blocking_document_ids == (document_id,)


def test_plan_does_not_copy_private_brief_bodies_and_validation_is_exact(
    tmp_path: Path,
) -> None:
    marker = "PRIVATE-BRIEF-MOTIVATION-2841"
    workspace, job, parsed, advert = _write_job(
        tmp_path,
        required_documents=["CV"],
    )
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, advert),
            confirmed_at=NOW,
        ),
        confirm_fields=True,
        private_marker=marker,
    )
    plan = _candidate(workspace, job)
    payload = plan.model_dump(mode="json")

    assert marker not in json.dumps(payload)
    assert validate_brief_candidate(
        payload,
        workspace=workspace,
        job_dir=job,
        input_fingerprint=plan.input_fingerprint,
    ) == plan
    tampered = {**payload, "blockers": ["documents.injected"]}
    with pytest.raises(BriefStageValidationError):
        validate_brief_candidate(
            tampered,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=plan.input_fingerprint,
        )


def test_document_kind_aliases_share_stable_identity() -> None:
    assert canonical_document_kind("CV") == "cv"
    assert canonical_document_kind("Curriculum vitae") == "cv"
    assert canonical_document_kind("C.V.") == "cv"
    assert canonical_document_kind("Cover-letter") == "cover_letter"


def test_requirement_markers_do_not_treat_incidental_optional_text_as_optional() -> None:
    required_label, required = brief_stage_module._clean_document_label(
        "Mandatory: cover letter discussing optional modules"
    )
    optional_label, optional = brief_stage_module._clean_document_label(
        "Optional: writing sample"
    )
    conflicted_label, conflicted = brief_stage_module._clean_document_label(
        "Mandatory: writing sample (optional)"
    )

    assert required_label == "cover letter discussing optional modules"
    assert required == "required"
    assert optional_label == "writing sample"
    assert optional == "optional"
    assert conflicted_label == "writing sample"
    assert conflicted == "required"


def test_distinct_document_semantics_never_collapse_to_one_task(tmp_path: Path) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=[
            "Research proposal",
            "Research statement",
            "Data-plan",
            "Data plan",
        ],
        advert=(
            "# Role\n\nRequired documents: Research proposal, Research statement, "
            "Data-plan, Data plan\n"
        ),
    )
    _write_brief(job)

    candidate = _candidate(workspace, job)

    assert len(candidate.requirements) == 4
    assert len({item.document_id for item in candidate.requirements}) == 4
    kinds = {item.normalized_kind for item in candidate.requirements}
    assert {"research_proposal", "research_statement", "data_plan"} <= kinds


def test_document_source_span_rejects_an_unrelated_label_mention(tmp_path: Path) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=(
            "# Role\n\nYour CV will not be returned after assessment.\n\n"
            "Essential criteria:\n- A doctorate\n"
        ),
    )
    _write_brief(job)

    candidate = _candidate(workspace, job)

    assert len(candidate.requirements) == 1
    requirement = candidate.requirements[0]
    assert requirement.source_state == "unknown"
    assert requirement.source_text is None
    assert requirement.source_span is None
    assert requirement.unknown_reason == "documents.source_not_found"
    assert stable_document_id(job_id="example-role", normalized_kind="cv") == (
        stable_document_id(job_id="example-role", normalized_kind="cv")
    )


@pytest.mark.parametrize(
    "advert",
    [
        "# Role\n\nRequired documents: CV\n",
        "# Role\n\n## Application materials\n- CV\n",
        "# Role\n\nPlease provide a CV.\n",
        "# Role\n\nApplications should include a CV.\n",
        "# Role\n\nA CV is required.\n",
        "# Role\n\nCV required.\n",
        "# Role\n\nCandidates need a CV.\n",
        "# Role\n\nApplications require a CV.\n",
        "# Role\n\nCV submission is required.\n",
        "# Role\n\nThe application must include a CV.\n",
        "# Role\n\nYour application must be accompanied by a CV.\n",
    ],
)
def test_document_source_span_accepts_explicit_requirement_context(
    tmp_path: Path,
    advert: str,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=advert,
    )
    _write_brief(job)

    requirement = _candidate(workspace, job).requirements[0]

    assert requirement.source_state == "known"
    assert requirement.source_text is not None
    assert requirement.source_span is not None
    assert requirement.source_span.path == "job_advert.md"
    assert requirement.source_span.start_line == requirement.source_span.end_line
    assert requirement.unknown_reason is None


@pytest.mark.parametrize(
    "source_line",
    [
        "Please provide your CV and cover letter.",
        "Applicants must submit a CV and cover letter.",
    ],
)
def test_structured_multi_document_sentences_require_every_member_to_reconcile(
    tmp_path: Path,
    source_line: str,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV", "Cover letter"],
        advert=f"# Role\n\n{source_line}\n",
    )
    _write_brief(job)

    requirements = _candidate(workspace, job).requirements

    assert len(requirements) == 2
    assert all(item.source_state == "known" for item in requirements)


@pytest.mark.parametrize(
    "source_line",
    [
        "Do not submit a CV.",
        "You are not required to provide a CV.",
        "We will never ask you to upload a CV.",
        "Applicants should not include a CV.",
        "If shortlisted, submit a CV.",
        "Applicants may provide a CV.",
        "Upload a CV upon request.",
        "Please submit the form. Your CV will not be returned.",
        "No CV is required.",
        "You don't have to submit a CV.",
        "You needn't submit a CV.",
        "You can submit a CV.",
        "Applicants are encouraged to submit a CV.",
        "CV is required, unless the portal waives it.",
        "CV is required except for internal candidates.",
        "Please submit the form, and your CV will be retained.",
        "Please submit the form — your CV will be retained.",
        "Please submit the form;Your CV will be retained.",
        "Please submit the form.Your CV will be retained.",
        "You might submit a CV.",
        "Perhaps submit a CV.",
        "Possibly submit a CV.",
        "A CV is rarely required.",
        "There is little need to provide a CV.",
        "A CV is required after appointment.",
        "A CV is required for successful candidates.",
        "A CV is required for HR records, not the application.",
        "Please submit the form and your CV will be retained.",
        "Please submit the form: your CV will be retained.",
        "Please submit the form (your CV will be retained).",
        "Please submit the form describing your CV experience.",
        "Please submit the application with your CV number.",
        "Required documents: application form. Your CV will be retained.",
        "Required documents include proof referenced in your CV.",
        "Required documents are described in your CV.",
        "Application materials are discussed in your CV.",
        "Required documents: a summary of your CV experience.",
        "Required documents: CV, where applicable.",
        "Required documents: CV, when applicable.",
        "Required documents: CV as required.",
        "Required documents: CV and/or cover letter.",
        "Required documents: CV or cover letter.",
        "Required documents: CV, optional.",
        "Required documents: CV, not compulsory.",
        "Required documents: CV, not essential.",
        "Required documents: CV, preferred.",
        "Required documents: CV; alternatives accepted.",
        "Required documents: CV, where requested.",
        "Required documents: CV, on request.",
    ],
)
def test_negated_conditional_or_unrelated_clauses_are_not_source_receipts(
    tmp_path: Path,
    source_line: str,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=f"# Role\n\n{source_line}\n",
    )
    _write_brief(job)

    requirement = _candidate(workspace, job).requirements[0]

    assert requirement.source_state == "unknown"
    assert requirement.source_span is None
    assert requirement.unknown_reason == "documents.source_not_found"


@pytest.mark.parametrize(
    "item",
    [
        "a summary of your CV experience",
        "application form mentioning your CV",
        "CV and/or cover letter",
        "CV or cover letter",
        "CV, where applicable",
        "CV, optional",
        "CV, not compulsory",
        "CV, preferred",
        "CV; alternatives accepted",
        "CV, on request",
    ],
)
def test_section_items_require_a_complete_document_member_match(
    tmp_path: Path,
    item: str,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=f"# Role\n\n## Application materials\n- {item}\n",
    )
    _write_brief(job)

    requirement = _candidate(workspace, job).requirements[0]

    assert requirement.source_state == "unknown"
    assert requirement.source_span is None
    assert requirement.unknown_reason == "documents.source_not_found"


@pytest.mark.parametrize(
    "advert",
    [
        "# Role\n\nRequired documents: CV. This is optional.\n",
        "# Role\n\nRequired documents: CV. It is not compulsory.\n",
        "# Role\n\nRequired documents: CV,\ncover letter\n",
        "# Role\n\n## Required documents\n- CV\n  Optional for internal candidates.\n",
        "# Role\n\n## Required documents\n- CV\n  (Only if shortlisted.)\n",
        "# Role\n\n## Required documents\n- CV,\n  cover letter\n",
        "# Role\n\n## Required documents\n- CV\n\n  Optional for internal candidates.\n",
        "# Role\n\nRequired documents: CV\n  Optional for internal candidates.\n",
        "# Role\n\nRequired documents: CV\n\n  Optional for internal candidates.\n",
        "# Role\n\nRequired documents: CV\n  cover letter\n",
    ],
)
def test_multisentence_or_multiline_document_context_is_fail_closed(
    tmp_path: Path,
    advert: str,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=advert,
    )
    _write_brief(job)

    requirement = _candidate(workspace, job).requirements[0]

    assert requirement.source_state == "unknown"
    assert requirement.source_span is None
    assert requirement.unknown_reason == "documents.source_not_found"


@pytest.mark.parametrize(
    "heading",
    [
        "Required documents if shortlisted",
        "Required documents for selected candidates",
        "Application materials upon request",
        "What to submit if invited",
    ],
)
def test_conditional_section_headings_are_not_source_receipts(
    tmp_path: Path,
    heading: str,
) -> None:
    workspace, job, _parsed, _advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=f"# Role\n\n## {heading}\n- CV\n",
    )
    _write_brief(job)

    requirement = _candidate(workspace, job).requirements[0]

    assert requirement.source_state == "unknown"
    assert requirement.source_span is None
    assert requirement.unknown_reason == "documents.source_not_found"


def test_section_heading_is_part_of_source_basis_and_requirement_semantics(
    tmp_path: Path,
) -> None:
    adverts = {
        heading: f"# Role\n\n## {heading}\n- CV\n"
        for heading in (
            "Required documents",
            "Optional documents",
            "Supporting documents",
            "Application materials",
        )
    }
    workspace, job, parsed, original_advert = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=adverts["Required documents"],
    )
    bases = {
        heading: document_requirements_basis_sha256(parsed, advert)
        for heading, advert in adverts.items()
    }
    assert len(set(bases.values())) == len(adverts)
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, original_advert),
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )
    required = _candidate(workspace, job)
    assert required.requirements_state == "confirmed"
    assert required.requirements[0].requirement == "required"
    required_span = required.requirements[0].source_span
    assert required_span is not None

    (job / "job_advert.md").write_text(
        adverts["Optional documents"],
        encoding="utf-8",
    )
    optional = _candidate(workspace, job)

    assert optional.requirements_state == "unconfirmed"
    assert optional.requirements[0].requirement == "optional"
    optional_span = optional.requirements[0].source_span
    assert optional_span is not None
    assert optional_span.text_sha256 == required_span.text_sha256
    assert optional_span.anchor_sha256 != required_span.anchor_sha256
    assert "documents.requirements_basis_changed" in optional.blockers


@pytest.mark.parametrize(
    ("advert", "unknown_reason"),
    [
        (
            "# Role\n\nYour CV will not be returned after assessment.\n\n"
            "Essential criteria:\n- A doctorate\n",
            "documents.source_not_found",
        ),
        (
            "# Role\n\nRequired documents: CV\n\nPlease submit CV.\n\n"
            "Essential criteria:\n- A doctorate\n",
            "documents.source_ambiguous",
        ),
    ],
)
def test_missing_or_ambiguous_sources_cannot_be_confirmed_ready(
    tmp_path: Path,
    advert: str,
    unknown_reason: str,
) -> None:
    workspace, job, parsed, advert_text = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=advert,
    )
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, advert_text),
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )

    plan = _candidate(workspace, job)

    assert plan.requirements_state == "unconfirmed"
    assert plan.requirements[0].source_state == "unknown"
    assert plan.requirements[0].unknown_reason == unknown_reason
    assert plan.tasks[0].action == "needs_confirmation"
    assert "documents.requirements_unconfirmed" in plan.blockers
    assert unknown_reason in plan.blockers


def test_source_receipt_move_change_and_removal_invalidate_confirmation(
    tmp_path: Path,
) -> None:
    advert = (
        "# Role\n\nRequired documents: CV\n\n"
        "Essential criteria:\n- A doctorate\n"
    )
    workspace, job, parsed, _advert_text = _write_job(
        tmp_path,
        required_documents=["CV"],
        advert=advert,
    )
    original_basis = document_requirements_basis_sha256(parsed, advert)
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=original_basis,
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )
    assert _candidate(workspace, job).requirements_state == "confirmed"

    variants = (
        advert.replace("Required documents: CV", "\nRequired documents: CV"),
        advert.replace("Required documents: CV", "Mandatory documents: CV"),
        advert.replace("Required documents: CV\n\n", ""),
    )
    seen_bases = {original_basis}
    for changed_advert in variants:
        (job / "job_advert.md").write_text(changed_advert, encoding="utf-8")
        changed_basis = document_requirements_basis_sha256(parsed, changed_advert)
        assert changed_basis not in seen_bases
        seen_bases.add(changed_basis)
        changed = _candidate(workspace, job)
        assert changed.requirements_state == "unconfirmed"
        assert "documents.requirements_basis_changed" in changed.blockers

    assert changed.requirements[0].source_state == "unknown"
    assert changed.requirements[0].unknown_reason == "documents.source_not_found"


def test_confirmed_empty_basis_is_bound_to_the_exact_current_advert(
    tmp_path: Path,
) -> None:
    workspace, job, parsed, advert = _write_job(
        tmp_path,
        required_documents=[],
    )
    basis = document_requirements_basis_sha256(parsed, advert)
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed_empty",
            basis_sha256=basis,
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )
    assert _candidate(workspace, job).requirements_state == "confirmed_empty"

    changed_advert = advert + "\nA new application instruction.\n"
    (job / "job_advert.md").write_text(changed_advert, encoding="utf-8")

    assert document_requirements_basis_sha256(parsed, changed_advert) != basis
    changed = _candidate(workspace, job)
    assert changed.requirements_state == "unconfirmed"
    assert "documents.requirements_basis_changed" in changed.blockers


def test_candidate_build_rechecks_decision_after_projection(
    tmp_path: Path,
    monkeypatch,
) -> None:
    workspace, job, parsed, advert = _write_job(
        tmp_path,
        required_documents=["CV"],
    )
    _write_brief(
        job,
        confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, advert),
            confirmed_at=NOW,
        ),
        confirm_fields=True,
    )
    fingerprint = brief_input_fingerprint(workspace, job)
    original = brief_stage_module._build_plan

    def build_then_change_decision(**kwargs):
        plan = original(**kwargs)
        decision_path = job / "application_decision.yaml"
        decision_path.write_bytes(decision_path.read_bytes() + b"# concurrent review\n")
        return plan

    monkeypatch.setattr(brief_stage_module, "_build_plan", build_then_change_decision)

    with pytest.raises(BriefStageError, match="changed"):
        build_deterministic_brief_candidate(
            workspace,
            job,
            input_fingerprint=fingerprint,
        )
