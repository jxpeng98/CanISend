from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path

import pytest
import yaml
from typer.testing import CliRunner

from canisend.cli import app
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
from canisend.draft_models import stable_claim_id
from canisend.llm import LLMProvider, LLMResponse
from canisend.stage_agent import (
    stage_apply_agent_response,
    stage_cancel_agent_response,
    stage_prepare_agent_response,
    stage_status_agent_response,
    stage_submit_agent_response,
)
from canisend.stage_runtime import (
    PreparedStage,
    StageRuntimeError,
    apply_stage_result,
    cancel_stage_task,
    inspect_stage_status,
    prepare_stage,
    run_configured_provider_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stage_store import read_json_object, sha256_file
from canisend.stages.brief_stage import (
    document_requirements_basis_sha256,
    stable_document_id,
)
from canisend.stages.draft_stage import (
    CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY,
    DRAFT_GENERATOR_STRATEGY,
    DRAFT_GENERATOR_VERSION,
    DraftStageValidationError,
    draft_input_fingerprint,
    draft_input_projection,
    draft_precondition_reasons,
    validate_draft_candidate,
)


NOW = "2026-07-13T10:00:00Z"
PRIVATE_MOTIVATION = "PRIVATE-DRAFT-MOTIVATION-SENTINEL-8924"
RAW_PROVIDER_PREAMBLE = "RAW-PROVIDER-PREAMBLE-MUST-NOT-PERSIST-4491"
RAW_PROVIDER_FAILURE = "RAW-PROVIDER-FAILURE-MUST-NOT-PERSIST-7721"


class RecordingProvider(LLMProvider):
    def __init__(self, content: str) -> None:
        self.content = content
        self.prompts: list[str] = []

    def complete(self, prompt: str) -> LLMResponse:
        self.prompts.append(prompt)
        return LLMResponse(content=self.content, provider="test-provider")


def _workspace(
    tmp_path: Path,
    *,
    include_cover_letter: bool = True,
    include_research_statement: bool = False,
    omit_cover_letter: bool = False,
    exclusions: tuple[str, ...] = (),
) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    profile = workspace / "profile"
    generated = profile / "generated"
    job.mkdir(parents=True)
    generated.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (profile / "profile.yaml").write_text(
        "generated:\n  cv_evidence: generated/cv.evidence.md\n",
        encoding="utf-8",
    )
    (generated / "cv.evidence.md").write_text(
        "# Evidence: CV\n\n"
        "## Education\n\n"
        "- [education-001] `education`: Completed a PhD in Economics.\n\n"
        "## Teaching\n\n"
        "- [teaching-001] `teaching`: Designed and taught applied econometrics modules.\n",
        encoding="utf-8",
    )
    (job / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": job.name,
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/job",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct",
                "created_at": NOW,
                "updated_at": NOW,
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    document_labels = ["CV"]
    if include_cover_letter:
        document_labels.append("Cover letter")
    if include_research_statement:
        document_labels.append("Research statement")
    documents = ", ".join(document_labels)
    (job / "job_advert.md").write_text(
        f"""# Lecturer in Economics

Required documents: {documents}

Essential criteria:
- PhD in Economics
- Experience teaching applied econometrics
""",
        encoding="utf-8",
    )

    for stage in ("evidence", "parse", "confirm", "match"):
        run_deterministic_stage(workspace, job, stage=stage)  # type: ignore[arg-type]

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

    parsed = read_json_object(job / "parsed_job.json")
    advert = (job / "job_advert.md").read_text(encoding="utf-8")
    document_choices: tuple[DocumentChoiceV1, ...] = ()
    if include_cover_letter and omit_cover_letter:
        document_choices = (
            DocumentChoiceV1(
                document_id=stable_document_id(
                    job_id=job.name,
                    normalized_kind="cover_letter",
                ),
                action="omit",
                confirmation_state="confirmed",
            ),
        )
    brief = ApplicationBriefV1(
        job_id=job.name,
        revision=1,
        updated_at=NOW,
        decision_sha256=sha256_file(job / "application_decision.yaml"),
        language=LanguagePreferenceV1(value="uk", confirmation_state="confirmed"),
        writing_style=ConfirmedTextV1(
            value="direct and evidence-led",
            confirmation_state="confirmed",
        ),
        motivation=ConfirmedTextV1(
            value=PRIVATE_MOTIVATION,
            confirmation_state="confirmed",
        ),
        emphasis=ConfirmedIdSelectionV1(confirmation_state="confirmed"),
        exclusions=ConfirmedStringListV1(
            items=exclusions,
            confirmation_state="confirmed",
        ),
        document_requirements_confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, advert),
            confirmed_at=NOW,
        ),
        document_choices=document_choices,
    )
    _write_yaml(job / "application_brief.yaml", brief)
    run_deterministic_stage(workspace, job, stage="brief")
    return workspace, job


def _write_yaml(path: Path, model: object) -> None:
    path.write_text(
        yaml.safe_dump(model.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )


def _candidate(workspace: Path, job: Path, *, factual: bool = False) -> dict[str, object]:
    fingerprint = draft_input_fingerprint(workspace, job)
    projection = draft_input_projection(workspace, job)
    document_id = str(projection["cover_letter_document_id"])
    if factual:
        criteria = read_json_object(job / "criteria.json")
        evidence = read_json_object(job / "evidence_catalog.json")
        criterion_id = criteria["criteria"][0]["criterion_id"]
        evidence_id = evidence["items"][0]["evidence_id"]
        text = "I completed a PhD in Economics."
        claim = {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind="factual",
                text=text,
            ),
            "text": text,
            "kind": "factual",
            "support_strength": "strong",
            "criterion_ids": [criterion_id],
            "evidence_ref_ids": [evidence_id],
            "brief_field_refs": [],
            "job_field_refs": [],
            "blockers": [],
            "review_state": "proposed",
        }
    else:
        text = "The opportunity to contribute to the department motivates my application."
        claim = {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind="motivation",
                text=text,
            ),
            "text": text,
            "kind": "motivation",
            "support_strength": "not_applicable",
            "criterion_ids": [],
            "evidence_ref_ids": [],
            "brief_field_refs": ["motivation"],
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
            "parsed_job_sha256": projection["parsed_job_sha256"],
            "criteria_sha256": projection["criteria_sha256"],
            "evidence_catalog_sha256": projection["evidence_catalog_sha256"],
            "criterion_matches_sha256": projection["criterion_matches_sha256"],
            "application_decision_sha256": projection[
                "application_decision_sha256"
            ],
            "application_brief_sha256": projection["application_brief_sha256"],
            "required_document_plan_sha256": projection[
                "required_document_plan_sha256"
            ],
        },
        "generation_mode": "host_agent",
        "generator_strategy": DRAFT_GENERATOR_STRATEGY,
        "generator_version": DRAFT_GENERATOR_VERSION,
        "review_state": "proposed",
        "sections": [{"section_id": "body", "heading": None, "claims": [claim]}],
        "blockers": [],
    }


def _provider_proposal(job: Path) -> dict[str, object]:
    criteria = read_json_object(job / "criteria.json")
    evidence = read_json_object(job / "evidence_catalog.json")
    return {
        "sections": [
            {
                "section_id": "body",
                "claims": [
                    {
                        "text": "I completed a PhD in Economics.",
                        "kind": "factual",
                        "support_strength": "strong",
                        "criterion_ids": [criteria["criteria"][0]["criterion_id"]],
                        "evidence_ref_ids": [evidence["items"][0]["evidence_id"]],
                        "brief_field_refs": [],
                        "job_field_refs": [],
                        "blockers": [],
                    },
                    {
                        "text": "The opportunity to contribute to the department motivates my application.",
                        "kind": "motivation",
                        "support_strength": "not_applicable",
                        "criterion_ids": [],
                        "evidence_ref_ids": [],
                        "brief_field_refs": ["motivation"],
                        "job_field_refs": [],
                        "blockers": [],
                    },
                ],
            }
        ]
    }


def _provider_content(job: Path, *, preamble: bool = False) -> str:
    encoded = json.dumps(_provider_proposal(job), ensure_ascii=False)
    if not preamble:
        return encoded
    return f"{RAW_PROVIDER_PREAMBLE}\n```json\n{encoded}\n```\n"


def _workspace_bytes(workspace: Path) -> dict[str, bytes]:
    return {
        path.relative_to(workspace).as_posix(): path.read_bytes()
        for path in sorted(workspace.rglob("*"))
        if path.is_file() and not path.is_symlink()
    }


def _downgrade_prepared_document_run_to_v1(
    prepared: PreparedStage,
    job: Path,
) -> tuple[bytes, bytes]:
    task = read_json_object(prepared.task_spec_path)
    task["schema_version"] = "1.0.0"
    task.pop("document_id")
    prepared.task_spec_path.write_text(
        json.dumps(task, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    preparation_path = prepared.task_spec_path.parent / "preparation.json"
    preparation = read_json_object(preparation_path)
    preparation["schema_version"] = "1.0.0"
    preparation.pop("document_id")
    preparation["task_spec_sha256"] = sha256_file(prepared.task_spec_path)
    preparation_path.write_text(
        json.dumps(preparation, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    state_path = job / "workflow" / "state.json"
    state = read_json_object(state_path)
    state["schema_version"] = "1.0.0"
    for record in state["stages"]:
        record.pop("document_id", None)
    state_path.write_text(
        json.dumps(state, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return prepared.task_spec_path.read_bytes(), preparation_path.read_bytes()


def _all_workspace_text(workspace: Path) -> str:
    return "\n".join(
        payload.decode("utf-8", errors="ignore")
        for payload in _workspace_bytes(workspace).values()
    )


def test_draft_projection_binds_all_seven_current_inputs(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)

    assert draft_precondition_reasons(workspace, job) == ()
    projection = draft_input_projection(workspace, job)

    assert projection["parsed_job_sha256"] == sha256_file(job / "parsed_job.json")
    assert projection["criteria_sha256"] == sha256_file(job / "criteria.json")
    assert projection["evidence_catalog_sha256"] == sha256_file(
        job / "evidence_catalog.json"
    )
    assert projection["criterion_matches_sha256"] == sha256_file(
        job / "criterion_matches.json"
    )
    assert projection["application_decision_sha256"] == sha256_file(
        job / "application_decision.yaml"
    )
    assert projection["application_brief_sha256"] == sha256_file(
        job / "application_brief.yaml"
    )
    assert projection["required_document_plan_sha256"] == sha256_file(
        job / "required_document_plan.json"
    )
    assert PRIVATE_MOTIVATION not in json.dumps(projection)


@pytest.mark.parametrize("factual", [False, True], ids=["motivation", "factual"])
def test_validator_accepts_structured_current_claims(tmp_path: Path, factual: bool) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=factual)

    validated = validate_draft_candidate(
        payload,
        workspace=workspace,
        job_dir=job,
        input_fingerprint=str(payload["input_fingerprint"]),
    )

    assert validated.job_id == job.name
    assert validated.review_state == "proposed"
    assert validated.sections[0].claims[0].kind == (
        "factual" if factual else "motivation"
    )


def test_validator_rejects_stale_or_tampered_basis(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    tampered = deepcopy(payload)
    tampered["basis"]["evidence_catalog_sha256"] = "0" * 64

    with pytest.raises(DraftStageValidationError, match="basis"):
        validate_draft_candidate(
            tampered,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )

    with pytest.raises(DraftStageValidationError, match="stale"):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint="0" * 64,
        )


@pytest.mark.parametrize("reference", ["criterion", "evidence"])
def test_validator_rejects_fabricated_semantic_references(
    tmp_path: Path,
    reference: str,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=True)
    claim = payload["sections"][0]["claims"][0]
    if reference == "criterion":
        claim["criterion_ids"] = ["criterion_" + "0" * 32]
    else:
        claim["evidence_ref_ids"] = ["evidence_" + "0" * 32]

    with pytest.raises(DraftStageValidationError, match="current"):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )


def test_validator_rejects_unknown_role_context_field(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    text = "The deadline is clearly stated."
    claim = payload["sections"][0]["claims"][0]
    claim.update(
        {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=str(payload["document_id"]),
                kind="role_context",
                text=text,
            ),
            "text": text,
            "kind": "role_context",
            "support_strength": "not_applicable",
            "brief_field_refs": [],
            "criterion_ids": [],
            "job_field_refs": ["deadline"],
        }
    )
    parsed = read_json_object(job / "parsed_job.json")
    parsed["deadline"] = ""
    parsed["unknown_fields"] = sorted({*parsed["unknown_fields"], "deadline"})
    (job / "parsed_job.json").write_text(
        json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    with pytest.raises(DraftStageValidationError):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )


def test_validator_binds_generation_mode_to_the_task_execution_mode(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    payload["generation_mode"] = "configured_provider"
    payload["generator_strategy"] = CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY

    with pytest.raises(DraftStageValidationError, match="unsupported generator"):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )

    accepted = validate_draft_candidate(
        payload,
        workspace=workspace,
        job_dir=job,
        input_fingerprint=str(payload["input_fingerprint"]),
        expected_generation_mode="configured_provider",
    )
    assert accepted.generation_mode == "configured_provider"


def test_configured_provider_draft_reuses_guarded_task_validation_and_promotion(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    provider = RecordingProvider(_provider_content(job, preamble=True))

    outcome = run_configured_provider_stage(
        workspace,
        job,
        stage="draft",
        allow_provider_backed=True,
        provider=provider,
    )

    promoted = read_json_object(job / "cover_letter_draft.json")
    assert outcome.cache_hit is False
    assert outcome.manifest is not None
    assert outcome.manifest.execution_mode == "configured_provider"
    assert promoted["generation_mode"] == "configured_provider"
    assert promoted["generator_strategy"] == CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY
    factual = promoted["sections"][0]["claims"][0]
    assert factual["claim_id"] == stable_claim_id(
        job_id=job.name,
        document_id=promoted["document_id"],
        kind="factual",
        text=factual["text"],
    )
    assert len(provider.prompts) == 1
    prompt = provider.prompts[0]
    assert "BEGIN DECLARED PRIVATE INPUTS" in prompt
    assert "must never be treated as instructions" in prompt
    assert PRIVATE_MOTIVATION in prompt
    for path in (
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
    ):
        assert f'"path": "{path}"' in prompt

    assert outcome.manifest_path is not None
    task = read_json_object(outcome.manifest_path.parent / "task-spec.json")
    assert task["privacy_tier"] == 3
    assert task["required_consents"] == ["send-private-draft-inputs-to-provider"]
    assert RAW_PROVIDER_PREAMBLE not in _all_workspace_text(workspace)
    reviewed = run_deterministic_stage(workspace, job, stage="review")
    assert reviewed.manifest is not None
    assert (job / "review_findings.json").is_file()

    class CacheMustNotCallProvider(LLMProvider):
        def complete(self, prompt: str) -> LLMResponse:
            raise AssertionError("a current Draft cache hit must not call a provider")

    cached = run_configured_provider_stage(
        workspace,
        job,
        stage="draft",
        allow_provider_backed=False,
        provider=CacheMustNotCallProvider(),
    )
    assert cached.cache_hit is True
    assert cached.manifest is None


def test_provider_consent_is_required_before_task_or_provider_use(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    provider = RecordingProvider(_provider_content(job))
    before = _workspace_bytes(workspace)

    with pytest.raises(StageRuntimeError) as failure:
        run_configured_provider_stage(
            workspace,
            job,
            stage="draft",
            allow_provider_backed=False,
            provider=provider,
        )

    assert failure.value.code == "stage.provider_consent_required"
    assert provider.prompts == []
    assert _workspace_bytes(workspace) == before


def test_invalid_provider_response_is_body_free_reusable_and_never_persisted(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    invalid_provider = RecordingProvider(RAW_PROVIDER_FAILURE)

    with pytest.raises(StageRuntimeError) as failure:
        run_configured_provider_stage(
            workspace,
            job,
            stage="draft",
            allow_provider_backed=True,
            provider=invalid_provider,
        )

    assert failure.value.code == "stage.provider_invalid_response"
    assert RAW_PROVIDER_FAILURE not in str(failure.value)
    assert RAW_PROVIDER_FAILURE not in _all_workspace_text(workspace)
    assert not (job / "cover_letter_draft.json").exists()
    inspection = inspect_stage_status(workspace, job, stage="draft")
    assert inspection.stage.status == "running"
    assert inspection.pending_task_path is not None
    task = read_json_object(inspection.pending_task_path)
    assert task["execution_mode"] == "configured_provider"
    assert not (job / task["candidate_output"]).exists()
    response = stage_status_agent_response(workspace, job, inspection)
    assert [item.id for item in response.required_consents] == [
        "send-private-draft-inputs-to-provider"
    ]
    assert response.required_consents[0].privacy_tier == 3
    assert response.next_actions[0].id == "stage.run_draft_provider"
    assert RAW_PROVIDER_FAILURE not in response.model_dump_json()

    retry = run_configured_provider_stage(
        workspace,
        job,
        stage="draft",
        allow_provider_backed=True,
        provider=RecordingProvider(_provider_content(job)),
    )
    assert retry.manifest is not None
    assert retry.manifest.attempt == 1


def test_provider_failure_and_mid_call_input_drift_fail_closed_without_leaking(
    tmp_path: Path,
) -> None:
    failed_workspace, failed_job = _workspace(tmp_path / "failed")

    class FailingProvider(LLMProvider):
        def complete(self, prompt: str) -> LLMResponse:
            raise RuntimeError("PRIVATE-PROVIDER-STDERR-SECRET-5519")

    with pytest.raises(StageRuntimeError) as failed:
        run_configured_provider_stage(
            failed_workspace,
            failed_job,
            stage="draft",
            allow_provider_backed=True,
            provider=FailingProvider(),
        )
    assert failed.value.code == "stage.provider_failed"
    assert "PRIVATE-PROVIDER-STDERR" not in str(failed.value)
    assert "PRIVATE-PROVIDER-STDERR" not in _all_workspace_text(failed_workspace)
    assert not (failed_job / "cover_letter_draft.json").exists()

    drift_workspace, drift_job = _workspace(tmp_path / "drift")

    class DriftingProvider(LLMProvider):
        def complete(self, prompt: str) -> LLMResponse:
            brief = drift_job / "application_brief.yaml"
            brief.write_bytes(brief.read_bytes() + b"# changed during provider call\n")
            return LLMResponse(
                content=_provider_content(drift_job),
                provider="test-provider",
            )

    with pytest.raises(StageRuntimeError) as drifted:
        run_configured_provider_stage(
            drift_workspace,
            drift_job,
            stage="draft",
            allow_provider_backed=True,
            provider=DriftingProvider(),
        )
    assert drifted.value.code == "stage.stale_input"
    assert not (drift_job / "cover_letter_draft.json").exists()
    inspection = inspect_stage_status(drift_workspace, drift_job, stage="draft")
    assert inspection.pending_task_path is not None
    task = read_json_object(inspection.pending_task_path)
    assert not (drift_job / task["candidate_output"]).exists()


def test_provider_retry_resumes_submitted_candidate_without_second_model_call(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _workspace(tmp_path)
    provider = RecordingProvider(_provider_content(job))

    def interrupt_after_submission(*args: object, **kwargs: object) -> object:
        raise StageRuntimeError("stage.interrupted", "Synthetic interruption.")

    monkeypatch.setattr(
        "canisend.stage_runtime.apply_stage_result",
        interrupt_after_submission,
    )
    with pytest.raises(StageRuntimeError, match="Synthetic interruption"):
        run_configured_provider_stage(
            workspace,
            job,
            stage="draft",
            allow_provider_backed=True,
            provider=provider,
        )
    assert len(provider.prompts) == 1
    inspection = inspect_stage_status(workspace, job, stage="draft")
    assert inspection.pending_task_path is not None
    task = read_json_object(inspection.pending_task_path)
    assert (job / task["candidate_output"]).is_file()
    assert (job / task["result_output"]).is_file()

    monkeypatch.setattr(
        "canisend.stage_runtime.apply_stage_result",
        apply_stage_result,
    )

    class NoSecondCallProvider(LLMProvider):
        def complete(self, prompt: str) -> LLMResponse:
            raise AssertionError("a submitted candidate must resume without another provider call")

    resumed = run_configured_provider_stage(
        workspace,
        job,
        stage="draft",
        allow_provider_backed=True,
        provider=NoSecondCallProvider(),
    )
    assert resumed.manifest is not None
    assert resumed.manifest.attempt == 1
    assert (job / "cover_letter_draft.json").is_file()


def test_configured_provider_cli_is_explicit_and_body_free(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace, job = _workspace(tmp_path)
    provider = RecordingProvider(_provider_content(job))
    document_id = str(
        draft_input_projection(workspace, job)["cover_letter_document_id"]
    )
    constructed: list[object] = []

    def configured_provider(config: object) -> LLMProvider:
        constructed.append(config)
        return provider

    monkeypatch.setattr(
        "canisend.stage_runtime.llm.provider_from_config",
        configured_provider,
    )
    base = [
        "stage",
        "run",
        "--workspace",
        str(workspace),
        "--job",
        "jobs/example-role",
        "--stage",
        "draft",
        "--document-id",
        document_id,
        "--mode",
        "configured-provider",
        "--format",
        "json",
    ]

    denied = CliRunner().invoke(app, base)
    assert denied.exit_code == 1
    denied_payload = json.loads(denied.stdout)
    assert denied_payload["error"]["code"] == "stage.provider_consent_required"
    assert provider.prompts == []
    assert constructed == []

    accepted = CliRunner().invoke(app, [*base, "--allow-provider-backed"])
    assert accepted.exit_code == 0, accepted.output
    payload = json.loads(accepted.stdout)
    assert payload["operation"] == "workflow.stage_run"
    assert payload["extensions"]["canisend.execution_mode"] == "configured_provider"
    assert payload["extensions"]["canisend.stage_status"] == "succeeded"
    assert payload["extensions"]["canisend.document_id"] == document_id
    assert PRIVATE_MOTIVATION not in accepted.stdout
    assert len(provider.prompts) == 1
    assert len(constructed) == 1


def test_precondition_blocks_omitted_or_unplanned_cover_letter(tmp_path: Path) -> None:
    omitted_workspace, omitted_job = _workspace(
        tmp_path / "omitted",
        omit_cover_letter=True,
    )
    missing_workspace, missing_job = _workspace(
        tmp_path / "missing",
        include_cover_letter=False,
    )

    assert draft_precondition_reasons(omitted_workspace, omitted_job) == (
        "input_not_ready:document_plan_blocked",
    )
    assert draft_precondition_reasons(missing_workspace, missing_job) == (
        "input_not_ready:cover_letter_not_planned",
    )


def test_document_scoped_runtime_rejects_invalid_mismatched_or_cross_stage_ids(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    before = _workspace_bytes(workspace)
    other_document_id = "document_fedcba9876543210fedcba9876543210"

    with pytest.raises(StageRuntimeError) as invalid:
        inspect_stage_status(
            workspace,
            job,
            stage="draft",
            document_id="not-a-document-id",
        )
    assert invalid.value.code == "stage.document_id_invalid"

    with pytest.raises(StageRuntimeError) as mismatched:
        prepare_stage(
            workspace,
            job,
            stage="draft",
            execution_mode="host_agent",
            document_id=other_document_id,
        )
    assert mismatched.value.code == "stage.document_not_found"

    with pytest.raises(StageRuntimeError) as cross_stage:
        inspect_stage_status(
            workspace,
            job,
            stage="parse",
            document_id=other_document_id,
        )
    assert cross_stage.value.code == "stage.document_scope_invalid"
    assert _workspace_bytes(workspace) == before
    assert PRIVATE_MOTIVATION not in " ".join(
        str(item.value) for item in (invalid, mismatched, cross_stage)
    )


def test_legacy_v1_pending_draft_is_reused_and_promoted_without_rewrite(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    candidate = _candidate(workspace, job)
    document_id = str(candidate["document_id"])
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )
    task_bytes, preparation_bytes = _downgrade_prepared_document_run_to_v1(
        prepared,
        job,
    )

    status = inspect_stage_status(workspace, job, stage="draft")
    reused = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
        document_id=document_id,
    )
    assert status.stage.document_id == document_id
    assert reused.reused is True
    assert reused.document_id == document_id
    assert reused.task_spec.schema_version == "1.0.0"
    assert reused.task_spec.document_id is None
    assert stage_prepare_agent_response(
        workspace,
        job,
        reused,
    ).extensions["canisend.document_id"] == document_id

    submitted = submit_stage_candidate(
        workspace,
        job,
        task_spec_path=reused.task_spec_path,
        candidate_bytes=(json.dumps(candidate) + "\n").encode("utf-8"),
    )
    assert submitted.document_id == document_id
    assert submitted.result.document_id is None
    assert stage_submit_agent_response(
        workspace,
        submitted,
    ).extensions["canisend.document_id"] == document_id

    applied = apply_stage_result(
        workspace,
        job,
        task_spec_path=reused.task_spec_path,
        task_result_path=submitted.result_path,
    )
    manifest = read_json_object(applied.manifest_path)
    assert applied.document_id == document_id
    assert applied.manifest.document_id is None
    assert manifest["schema_version"] == "1.0.0"
    assert "document_id" not in manifest
    assert stage_apply_agent_response(
        workspace,
        applied,
    ).extensions["canisend.document_id"] == document_id
    assert prepared.task_spec_path.read_bytes() == task_bytes
    assert (prepared.task_spec_path.parent / "preparation.json").read_bytes() == (
        preparation_bytes
    )
    assert next(
        item for item in applied.state.stages if item.stage == "draft"
    ).document_id == document_id


def test_legacy_v1_pending_draft_can_be_cancelled_by_current_document_id(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    document_id = str(_candidate(workspace, job)["document_id"])
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )
    task_bytes, preparation_bytes = _downgrade_prepared_document_run_to_v1(
        prepared,
        job,
    )

    cancelled = cancel_stage_task(
        workspace,
        job,
        stage="draft",
        document_id=document_id,
    )
    manifest = read_json_object(cancelled.manifest_path)
    assert cancelled.document_id == document_id
    assert cancelled.manifest.document_id is None
    assert manifest["schema_version"] == "1.0.0"
    assert "document_id" not in manifest
    assert stage_cancel_agent_response(
        workspace,
        cancelled,
    ).extensions["canisend.document_id"] == document_id
    assert prepared.task_spec_path.read_bytes() == task_bytes
    assert (prepared.task_spec_path.parent / "preparation.json").read_bytes() == (
        preparation_bytes
    )
    assert next(
        item for item in cancelled.state.stages if item.stage == "draft"
    ).document_id == document_id


def test_draft_runs_through_guarded_host_agent_promotion(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=True)

    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )
    prepare_response = stage_prepare_agent_response(workspace, job, prepared)

    assert [item.path for item in prepared.task_spec.inputs] == [
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
    ]
    assert prepared.task_spec.authoritative_target == "cover_letter_draft.json"
    assert prepared.task_spec.document_id == payload["document_id"]
    assert prepared.task_spec.output_schema == "canisend.cover-letter-draft/v1"
    assert prepared.task_spec.execution_mode == "host_agent"
    assert prepared.task_spec.required_consents == ("read-private-draft-inputs",)
    assert prepared.task_spec.write_authority == "core_service"
    assert PRIVATE_MOTIVATION not in prepared.task_spec.model_dump_json()
    assert [item.id for item in prepare_response.required_consents] == [
        "read-private-draft-inputs"
    ]
    assert prepare_response.workflow is not None
    assert prepare_response.workflow.readiness == "action_required"
    assert prepare_response.extensions["canisend.document_id"] == payload["document_id"]

    submitted = submit_stage_candidate(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(payload, ensure_ascii=False) + "\n").encode(),
    )
    submit_response = stage_submit_agent_response(workspace, submitted)
    assert not (job / "cover_letter_draft.json").exists()
    assert submit_response.workflow is not None
    assert submit_response.workflow.readiness == "review_required"
    assert submitted.result.document_id == payload["document_id"]
    assert submit_response.extensions["canisend.document_id"] == payload["document_id"]

    applied = apply_stage_result(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        task_result_path=submitted.result_path,
    )
    apply_response = stage_apply_agent_response(workspace, applied)
    promoted = read_json_object(job / "cover_letter_draft.json")

    assert promoted["review_state"] == "proposed"
    assert applied.manifest.document_id == payload["document_id"]
    assert apply_response.extensions["canisend.document_id"] == payload["document_id"]
    assert next(
        item for item in applied.state.stages if item.stage == "draft"
    ).document_id == payload["document_id"]
    assert promoted["sections"][0]["claims"][0]["kind"] == "factual"
    assert apply_response.workflow is not None
    assert apply_response.workflow.readiness == "review_required"
    assert apply_response.extensions["canisend.draft_claim_count"] == 1
    assert apply_response.extensions["canisend.draft_factual_claim_count"] == 1
    assert apply_response.extensions["canisend.draft_blocker_count"] == 0
    assert [item.id for item in apply_response.next_actions] == [
        "stage.run_review"
    ]

    status = inspect_stage_status(workspace, job, stage="draft")
    assert status.stage.status == "succeeded"
    assert not status.reasons


def test_invalid_draft_submission_cannot_modify_authoritative_or_owned_inputs(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    payload["ready"] = True
    legacy = job / "03_cover_letter_draft.md"
    legacy.write_text("USER-EDITED-LEGACY-DRAFT\n", encoding="utf-8")
    protected = {
        path: path.read_bytes()
        for path in (
            job / "application_decision.yaml",
            job / "application_brief.yaml",
            job / "required_document_plan.json",
            job / "03_cover_letter_draft.md",
            workspace / "profile" / "profile.yaml",
        )
    }
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )

    with pytest.raises(StageRuntimeError) as failure:
        submit_stage_candidate(
            workspace,
            job,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(payload) + "\n").encode(),
        )

    assert failure.value.code == "stage.invalid_candidate"
    assert not (job / "cover_letter_draft.json").exists()
    assert not prepared.candidate_path.exists()
    assert all(path.read_bytes() == before for path, before in protected.items())


def test_prepared_draft_becomes_stale_before_submission_without_promotion(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )
    brief = job / "application_brief.yaml"
    brief.write_bytes(brief.read_bytes() + b"# edited after task preparation\n")

    with pytest.raises(StageRuntimeError) as failure:
        submit_stage_candidate(
            workspace,
            job,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(payload) + "\n").encode(),
        )

    assert failure.value.code in {"stage.dependency_not_current", "stage.stale_input"}
    assert not (job / "cover_letter_draft.json").exists()
    assert not prepared.candidate_path.exists()


def test_draft_refuses_deterministic_execution_mode(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)

    with pytest.raises(StageRuntimeError) as failure:
        prepare_stage(
            workspace,
            job,
            stage="draft",
            execution_mode="deterministic",
        )

    assert failure.value.code == "stage.unsupported_mode"
