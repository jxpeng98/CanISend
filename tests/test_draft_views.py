from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path

import pytest

from canisend.bundle_models import ProjectionJournalV1
from canisend.bundle_projection import load_artifact_bundle, project_artifact_bundle
from canisend.draft_models import stable_claim_id
from canisend.draft_views import (
    STRUCTURED_DRAFT_PROJECTION_SOURCE,
    STRUCTURED_DRAFT_TYPST_MARKER,
    build_structured_cover_letter_content,
    load_current_structured_draft_views,
)
from canisend.pipeline import run_pipeline
from canisend.legacy_compatibility import (
    LegacyCompatibilityOutcome,
    run_legacy_package_compatibility,
)
from canisend.ready_check import check_application_package
from canisend.stage_runtime import run_deterministic_stage
from canisend.stage_store import read_json_object
from canisend.user_mutations import (
    SetFindingDispositionPatch,
    SetPackageFindingDispositionPatch,
    apply_user_patch,
    initialize_package_review_dispositions,
    initialize_review_dispositions,
)
from canisend.workflow_sequence import SequenceOptions, run_sequence
from tests.test_draft_stage import _candidate, _workspace
from tests.test_review_stage import _complete_sections, _promote_draft
from tests.workflow_fixtures import clone_prebuilt_workspace


STRUCTURED_SENTINEL = "STRUCTURED-DRAFT-PROJECTION-SENTINEL-7319"


def _project_legacy_compatibility(
    workspace: Path,
    job: Path,
) -> LegacyCompatibilityOutcome:
    outcome = run_legacy_package_compatibility(workspace, job)
    assert outcome.active
    return outcome


def _project_guarded_package(workspace: Path, job: Path) -> ProjectionJournalV1:
    if not (job / "review_dispositions.yaml").is_file():
        review_outcome = initialize_review_dispositions(
            workspace,
            job,
            consent_confirmed=True,
        )
        review = read_json_object(job / "review_findings.json")
        for finding in review["findings"]:
            review_outcome = apply_user_patch(
                workspace,
                job,
                SetFindingDispositionPatch(
                    finding_id=finding["finding_id"],
                    disposition="accepted",
                ),
                expected_sha256=review_outcome.snapshot.sha256,
                expected_revision=review_outcome.snapshot.revision,
                consent_confirmed=True,
            )

    run_deterministic_stage(workspace, job, stage="package_review")
    package_review = read_json_object(job / "package_review_findings.json")
    assert package_review["blocker_finding_ids"] == []
    package_outcome = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    for finding in package_review["findings"]:
        package_outcome = apply_user_patch(
            workspace,
            job,
            SetPackageFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=package_outcome.snapshot.sha256,
            expected_revision=package_outcome.snapshot.revision,
            consent_confirmed=True,
        )

    run_deterministic_stage(workspace, job, stage="package")
    bundle = load_artifact_bundle(job / "package_bundle.json")
    assert bundle.mode == "guarded"
    return project_artifact_bundle(job, bundle)


def _build_reviewed_draft(
    tmp_path: Path,
    *,
    text: str = STRUCTURED_SENTINEL,
    unsupported: bool = False,
    include_research_statement: bool = False,
) -> tuple[Path, Path, dict[str, object]]:
    workspace, job = _workspace(
        tmp_path,
        include_cv=False,
        include_research_statement=include_research_statement,
    )
    payload = _complete_sections(_candidate(workspace, job, factual=True))
    claim = payload["sections"][1]["claims"][0]
    claim["text"] = text
    claim["claim_id"] = stable_claim_id(
        job_id=job.name,
        document_id=str(payload["document_id"]),
        kind="factual",
        text=text,
    )
    if unsupported:
        claim["support_strength"] = "unsupported"
        claim["evidence_ref_ids"] = []
        claim["blockers"] = ["claim.unsupported"]
        payload["blockers"] = ["claim.unsupported"]
    _promote_draft(workspace, job, payload)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(payload["document_id"]),
    )
    return workspace, job, payload


def _reviewed_draft(
    tmp_path: Path,
    *,
    text: str = STRUCTURED_SENTINEL,
    unsupported: bool = False,
    include_research_statement: bool = False,
) -> tuple[Path, Path, dict[str, object]]:
    text_key = hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]
    key = (
        f"reviewed-draft-{text_key}"
        f"-unsupported-{int(unsupported)}"
        f"-research-{int(include_research_statement)}"
    )

    def build(root: Path) -> tuple[Path, Path]:
        workspace, job, _payload = _build_reviewed_draft(
            root,
            text=text,
            unsupported=unsupported,
            include_research_statement=include_research_statement,
        )
        return workspace, job

    workspace, job = clone_prebuilt_workspace(
        tmp_path,
        key=key,
        builder=build,
    )
    return workspace, job, read_json_object(job / "cover_letter_draft.json")


def test_current_structured_draft_renders_claims_once_and_builds_traceable_content(
    tmp_path: Path,
) -> None:
    workspace, job, payload = _reviewed_draft(tmp_path)
    parsed = read_json_object(job / "parsed_job.json")

    views = load_current_structured_draft_views(
        workspace,
        job,
        parsed_job=parsed,
    )

    assert views is not None
    claim_texts = [
        claim["text"]
        for section in payload["sections"]
        for claim in section["claims"]
    ]
    assert views.markdown.startswith("# Cover Letter Draft\n\n")
    assert "## " not in views.markdown
    assert all(views.markdown.count(text) == 1 for text in claim_texts)

    content = build_structured_cover_letter_content(parsed, views)
    projection = content["projection"]
    assert projection["source"] == STRUCTURED_DRAFT_PROJECTION_SOURCE
    assert projection["blocker_count"] == 0
    assert projection["draft_blocker_count"] == 0
    assert projection["review_blocker_count"] == 0
    assert projection["document_readiness_state"] == "review_required"
    assert projection["draft_review_state"] == "proposed"
    assert projection["review_state"] == "proposed"
    assert projection["requires_human_review"] is True
    assert projection["markdown_sha256"] == hashlib.sha256(
        views.markdown.encode("utf-8")
    ).hexdigest()
    rendered_claims = [
        claim
        for section in content["structured_sections"]
        for claim in section["claims"]
    ]
    assert [claim["claim_id"] for claim in rendered_claims] == [
        claim["claim_id"]
        for section in payload["sections"]
        for claim in section["claims"]
    ]


def test_cover_projection_uses_stable_identity_when_research_is_also_ready(
    tmp_path: Path,
) -> None:
    workspace, job, payload = _reviewed_draft(
        tmp_path,
        include_research_statement=True,
    )

    views = load_current_structured_draft_views(
        workspace,
        job,
        parsed_job=read_json_object(job / "parsed_job.json"),
    )

    assert views is not None
    assert views.draft.document_id == payload["document_id"]


def test_markdown_projection_neutralizes_agent_controlled_structure(
    tmp_path: Path,
) -> None:
    text = (
        "# Claimed heading\n"
        "[external](https://example.edu) <script>alert(1)</script>\n"
        "Setext heading\n---\n> quote\n| table |\n    indented code"
    )
    workspace, job, _payload = _reviewed_draft(tmp_path, text=text)

    views = load_current_structured_draft_views(
        workspace,
        job,
        parsed_job=read_json_object(job / "parsed_job.json"),
    )

    assert views is not None
    assert "\\# Claimed heading" in views.markdown
    assert "\\[external\\](https://example.edu)" in views.markdown
    assert "&lt;script&gt;" in views.markdown
    assert "<script>" not in views.markdown
    assert "\\---" in views.markdown
    assert "&gt; quote" in views.markdown
    assert "\\| table |" in views.markdown
    assert "&#32;   indented code" in views.markdown


def test_typst_projection_keeps_agent_claim_text_inside_a_string(
    tmp_path: Path,
) -> None:
    text = '#evil("x")\n= heading\n// comment\n*bold*'
    workspace, job, _payload = _reviewed_draft(tmp_path, text=text)

    _project_guarded_package(workspace, job)
    source = (job / "typst" / "cover_letter.typ").read_text(encoding="utf-8")
    assert '#text("#evil(\\"x\\")\\n= heading\\n// comment\\n*bold*")' in source
    assert "\n#evil(" not in source
    assert "\n= heading" not in source
    assert "\n// comment\n" not in source


@pytest.mark.parametrize(
    "failure",
    ["missing_review", "review_blocker", "draft_drift", "review_drift", "parsed_mismatch"],
)
def test_structured_draft_view_falls_back_closed(
    tmp_path: Path,
    failure: str,
) -> None:
    if failure == "review_blocker":
        workspace, job, _payload = _reviewed_draft(tmp_path, unsupported=True)
    else:
        workspace, job = _workspace(tmp_path)
        payload = _complete_sections(_candidate(workspace, job, factual=True))
        _promote_draft(workspace, job, payload)
        if failure != "missing_review":
            run_deterministic_stage(workspace, job, stage="review")

    parsed = read_json_object(job / "parsed_job.json")
    if failure == "draft_drift":
        draft_path = job / "cover_letter_draft.json"
        draft_path.write_bytes(draft_path.read_bytes() + b" ")
    elif failure == "review_drift":
        review_path = job / "review_findings.json"
        review_path.write_bytes(review_path.read_bytes() + b" ")
    elif failure == "parsed_mismatch":
        parsed = deepcopy(parsed)
        parsed["title"] = "A different parser view"

    assert load_current_structured_draft_views(
        workspace,
        job,
        parsed_job=parsed,
    ) is None


def test_pipeline_projects_structured_draft_into_markdown_and_typst(
    tmp_path: Path,
) -> None:
    workspace, job, payload = _reviewed_draft(tmp_path)
    protected_names = (
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
        "cover_letter_draft.json",
        "review_findings.json",
    )
    before = {name: (job / name).read_bytes() for name in protected_names}

    _project_guarded_package(workspace, job)
    assert {name: (job / name).read_bytes() for name in protected_names} == before
    markdown = (job / "03_cover_letter_draft.md").read_text(encoding="utf-8")
    cover_content = json.loads(
        (job / "typst" / "cover_letter_content.json").read_text(encoding="utf-8")
    )
    package_content = json.loads(
        (job / "typst" / "application_package_content.json").read_text(
            encoding="utf-8"
        )
    )
    cover_source = (job / "typst" / "cover_letter.typ").read_text(encoding="utf-8")
    package_source = (job / "typst" / "application_package.typ").read_text(
        encoding="utf-8"
    )
    claim_id = payload["sections"][1]["claims"][0]["claim_id"]

    assert markdown.count(STRUCTURED_SENTINEL) == 1
    assert cover_content["projection"]["source"] == "cover_letter_draft.json"
    assert package_content["cover_letter"] == markdown
    assert package_content["cover_letter_projection"] == cover_content["projection"]
    assert package_content["structured_cover_letter_sections"] == cover_content[
        "structured_sections"
    ]
    assert STRUCTURED_DRAFT_TYPST_MARKER in cover_source
    for source in (cover_source, package_source):
        assert source.count(STRUCTURED_SENTINEL) == 1
        assert f"// CANISEND: claim {claim_id}" in source

    package_check = check_application_package(job, workspace / "profile")
    assert not any(
        issue.path == "typst/cover_letter.typ"
        and "missing stable section marker" in issue.message
        for issue in package_check.issues
    )
    assert not any("not document readiness" in issue.message for issue in package_check.issues)


def test_complete_user_dispositions_make_only_the_cover_letter_reviewed(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    outcome = initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    review = read_json_object(job / "review_findings.json")
    for finding in review["findings"]:
        outcome = apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=outcome.snapshot.sha256,
            expected_revision=outcome.snapshot.revision,
            consent_confirmed=True,
        )

    _project_guarded_package(workspace, job)
    cover_content = json.loads(
        (job / "typst" / "cover_letter_content.json").read_text(encoding="utf-8")
    )
    projection = cover_content["projection"]
    assert projection["draft_review_state"] == "proposed"
    assert projection["review_state"] == "proposed"
    assert projection["document_readiness_state"] == "reviewed"
    assert projection["document_readiness"]["state"] == "reviewed"
    assert projection["requires_human_review"] is False

    package_check = check_application_package(job, workspace / "profile")
    assert not any("not document readiness" in issue.message for issue in package_check.issues)

    with (job / "review_dispositions.yaml").open("ab") as stream:
        stream.write(b" ")
    changed = check_application_package(job, workspace / "profile")
    assert any(
        issue.path == "review_dispositions.yaml"
        and "does not match" in issue.message
        for issue in changed.issues
    )


def test_package_check_binds_structured_projection_to_draft_and_review(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    _project_guarded_package(workspace, job)

    cover_source_path = job / "typst" / "cover_letter.typ"
    cover_source = cover_source_path.read_text(encoding="utf-8")
    cover_source_path.write_text(
        cover_source.replace("// CANISEND: section body\n", "", 1)
        + '\n#text("// CANISEND: section body")\n',
        encoding="utf-8",
    )
    marker_check = check_application_package(job, workspace / "profile")
    assert any(
        issue.path == "typst/cover_letter.typ"
        and "missing stable section marker: // CANISEND: section body"
        in issue.message
        for issue in marker_check.issues
    )

    markdown_path = job / "03_cover_letter_draft.md"
    markdown_path.write_bytes(markdown_path.read_bytes() + b"\nmanual edit\n")
    markdown_check = check_application_package(job, workspace / "profile")
    assert any(
        issue.path == "03_cover_letter_draft.md"
        and "source or view is missing or has changed" in issue.message
        for issue in markdown_check.issues
    )

    draft_path = job / "cover_letter_draft.json"
    draft_path.write_bytes(draft_path.read_bytes() + b" ")

    package_check = check_application_package(job, workspace / "profile")
    assert any(
        issue.path == "cover_letter_draft.json"
        and "source or view is missing or has changed" in issue.message
        for issue in package_check.issues
    )


@pytest.mark.parametrize("drifted_output", ["cover_letter_draft.json", "review_findings.json"])
def test_sequence_fails_closed_without_legacy_fallback_when_structured_output_drifts(
    tmp_path: Path,
    drifted_output: str,
) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)
    drifted_path = job / drifted_output
    drifted_path.write_bytes(drifted_path.read_bytes() + b" ")

    sequence = run_sequence(
        workspace,
        job,
        options=SequenceOptions(legacy_compatibility=True),
    )

    assert sequence.legacy_compatibility is None
    assert any(item.decision == "repair" for item in sequence.plan.items)
    assert not (job / "package_bundle.json").exists()
    assert not (job / "03_cover_letter_draft.md").exists()


def test_pipeline_preserves_edited_typst_when_structured_projection_appears(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path, include_cv=False)
    first = _project_legacy_compatibility(workspace, job)
    primary = job / "typst" / "cover_letter.typ"
    edited = primary.read_text(encoding="utf-8") + "\n// USER EDIT\n"
    primary.write_text(edited, encoding="utf-8")

    payload = _complete_sections(_candidate(workspace, job, factual=True))
    claim = payload["sections"][1]["claims"][0]
    claim["text"] = STRUCTURED_SENTINEL
    claim["claim_id"] = stable_claim_id(
        job_id=job.name,
        document_id=str(payload["document_id"]),
        kind="factual",
        text=STRUCTURED_SENTINEL,
    )
    _promote_draft(workspace, job, payload)
    run_deterministic_stage(workspace, job, stage="review")

    second = _project_guarded_package(workspace, job)

    candidate = job / "typst" / "cover_letter.generated.typ"
    assert first.journal is not None
    assert primary.read_text(encoding="utf-8") == edited
    assert candidate.is_file()
    assert STRUCTURED_SENTINEL in candidate.read_text(encoding="utf-8")
    assert any(
        entry.target_path == "typst/cover_letter.generated.typ"
        and entry.outcome == "candidate_created"
        for entry in second.entries
    )


def test_direct_library_call_preserves_legacy_draft_behavior(tmp_path: Path) -> None:
    workspace, job, _payload = _reviewed_draft(tmp_path)

    run_pipeline(
        job,
        profile_dir=workspace / "profile",
        workspace=None,
    )

    markdown = (job / "03_cover_letter_draft.md").read_text(encoding="utf-8")
    assert STRUCTURED_SENTINEL not in markdown
    assert "## Research Fit" in markdown
