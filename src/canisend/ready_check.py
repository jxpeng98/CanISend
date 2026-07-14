from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
import hashlib
import json
from pathlib import Path
import re

import yaml
from pydantic import ValidationError

from canisend.agent_protocol import (
    AgentResponse,
    GateOutcome,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    success_response,
)
from canisend.draft_views import (
    STRUCTURED_DRAFT_PROJECTION_SOURCE,
    STRUCTURED_DRAFT_TYPST_MARKER,
)
from canisend.draft_models import ReviewFindingsV1
from canisend.evidence import load_generated_evidence
from canisend.jobs import job_advert_is_stub
from canisend.materials import MaterialValidationError, validate_markdown_citations
from canisend.parse import ParsedJobValidationError, validate_parsed_job
from canisend.review_readiness import (
    DocumentReadinessV1,
    ReviewDispositionsV1,
    derive_document_readiness,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    load_strict_json,
    load_strict_yaml,
)


REQUIRED_MARKDOWN_FILES = [
    "00_preparation_questions.md",
    "01_job_summary.md",
    "02_fit_report.md",
    "03_cover_letter_draft.md",
    "04_cv_tailoring_notes.md",
    "05_criteria_checklist.md",
    "06_final_application_package.md",
    "07_material_review_checklist.md",
]
REQUIRED_SOURCE_FILES = [
    "job.yaml",
    "job_advert.md",
]
REQUIRED_CONTENT_JSON_FILES = [
    "typst/cover_letter_content.json",
    "typst/application_package_content.json",
]
# Retain the original aggregate name for callers that imported it directly.
REQUIRED_JSON_FILES = ["parsed_job.json", *REQUIRED_CONTENT_JSON_FILES]
REQUIRED_TYPST_MARKERS = {
    "typst/cover_letter.typ": (
        "// CANISEND: section opening",
        "// CANISEND: section research_fit",
        "// CANISEND: section teaching_fit",
        "// CANISEND: section departmental_contribution",
        "// CANISEND: section service_leadership",
        "// CANISEND: section closing",
    ),
    "typst/application_package.typ": (
        "// CANISEND: section job_information",
        "// CANISEND: section fit_report",
        "// CANISEND: section cover_letter",
        "// CANISEND: section cv_tailoring_notes",
        "// CANISEND: section criteria_checklist",
        "// CANISEND: section remaining_actions",
    ),
}
STRUCTURED_COVER_LETTER_TYPST_MARKERS = (
    STRUCTURED_DRAFT_TYPST_MARKER,
    "// CANISEND: section opening",
    "// CANISEND: section body",
    "// CANISEND: section closing",
)
APPLICATION_GATE_REPORT = "application_gate_report.json"
APPLICATION_GATE_REPORT_SCHEMA_VERSION = "1.0.0"
APP_Q1 = "APP-Q1"
APP_Q2 = "APP-Q2"
APP_Q3 = "APP-Q3"
APP_Q4 = "APP-Q4"
PLACEHOLDER_RE = re.compile(r"\[[A-Za-z][^\]\n]{2,}\]")
BLOCKER_RE = re.compile(r"\bBLOCKER\b", flags=re.IGNORECASE)
MODERNPRO_IMPORT_RE = re.compile(
    r'^\s*#import\s+"@preview/modernpro-(?:coverletter|cv):[^"]+"',
    flags=re.MULTILINE,
)
REQUIRED_JOB_METADATA_FIELDS = ("title", "institution", "deadline", "source_url", "status")
OPTIONAL_STANDALONE_TYPST_CANDIDATES = {
    "research_statement.generated.typ",
}


@dataclass(frozen=True)
class PackageCheckIssue:
    path: str
    message: str
    gate: str = APP_Q3


@dataclass(frozen=True)
class PackageCheckResult:
    job_dir: Path
    issues: list[PackageCheckIssue]
    input_hashes: dict[str, str] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        return not self.issues

    @property
    def status(self) -> str:
        return "PASS" if self.ok else "FAIL"

    def output_lines(self) -> list[str]:
        if self.ok:
            return [f"Package check passed: {self.job_dir}"]
        lines = [f"Package check failed: {self.job_dir}"]
        lines.extend(f"- {issue.path}: {issue.message}" for issue in self.issues)
        return lines

    def report_data(self) -> dict[str, object]:
        return {
            "schema_version": APPLICATION_GATE_REPORT_SCHEMA_VERSION,
            "generated_at": _utc_now(),
            "status": self.status,
            "input_hashes": dict(sorted(self.input_hashes.items())),
            "issues": [
                {
                    "gate": issue.gate,
                    "path": issue.path,
                    "message": issue.message,
                }
                for issue in self.issues
            ],
        }

    def write_report(self) -> Path:
        if not self.job_dir.is_dir():
            raise ValueError(
                f"Cannot write application gate report because the job directory does not exist: {self.job_dir}"
            )
        report_path = self.job_dir / APPLICATION_GATE_REPORT
        report_path.write_text(json.dumps(self.report_data(), indent=2) + "\n", encoding="utf-8")
        return report_path


def package_check_agent_response(
    result: PackageCheckResult,
    *,
    workspace: Path,
    report_path: Path | None = None,
    report_write_failed: bool = False,
) -> AgentResponse:
    workspace_root = workspace.expanduser().resolve()
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace_root,
            path=result.job_dir,
            kind="job_directory",
            privacy_tier=2,
            trust_level="generated_candidate",
            media_type="inode/directory",
        )
    ]
    safe_report_path: str | None = None
    if report_path is not None:
        report_reference = artifact_reference_from_path(
            workspace=workspace_root,
            path=report_path,
            kind="application_gate_report",
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
            include_hash=True,
        )
        artifacts.append(report_reference)
        safe_report_path = report_reference.path

    if result.ok:
        workflow = WorkflowSnapshotReference(phase="render", readiness="review_required")
        actions = [
            NextAction(
                id="package.review",
                label="Perform final human review before manual submission",
            )
        ]
        blockers: list[str] = []
    else:
        workflow = WorkflowSnapshotReference(phase="verify", readiness="blocked")
        actions = [
            NextAction(
                id="package.resolve_blockers",
                label="Resolve application gate blockers",
            )
        ]
        blockers = [
            f"{issue.gate}: {_safe_gate_issue_path(issue.path)} requires review"
            for issue in result.issues
        ]

    warnings = ["The application gate report could not be written."] if report_write_failed else []
    return success_response(
        operation="package.check",
        workflow=workflow,
        artifacts=artifacts,
        warnings=warnings,
        blockers=blockers,
        next_actions=actions,
        gate=GateOutcome(
            status=result.status,
            issue_count=len(result.issues),
            report_path=safe_report_path,
        ),
    )


def _safe_gate_issue_path(value: str) -> str:
    if not value or "\\" in value:
        return "job"
    path = Path(value)
    if path.is_absolute() or any(part in {".", ".."} for part in path.parts):
        return "job"
    return "/".join(re.sub(r"[^A-Za-z0-9_.-]", "_", part) for part in path.parts)


def check_application_package(job_dir: Path, profile_dir: Path) -> PackageCheckResult:
    job_dir = job_dir.expanduser().resolve()
    profile_dir = profile_dir.expanduser().resolve()
    issues: list[PackageCheckIssue] = []

    if not job_dir.exists():
        return PackageCheckResult(
            job_dir=job_dir,
            issues=[PackageCheckIssue(str(job_dir), "job directory does not exist", APP_Q1)],
        )

    metadata_path = _require_file(job_dir, "job.yaml", issues, gate=APP_Q1)
    metadata = _check_job_metadata(metadata_path, issues) if metadata_path is not None else None
    advert_path = _require_file(job_dir, "job_advert.md", issues, gate=APP_Q1)
    if advert_path is not None:
        _check_job_advert(advert_path, issues)

    for relative_path in REQUIRED_MARKDOWN_FILES:
        path = _require_file(job_dir, relative_path, issues, gate=APP_Q3)
        if path is not None:
            _check_placeholders(path, relative_path, issues, gate=APP_Q3)
            if relative_path == "07_material_review_checklist.md":
                _check_material_review_blockers(path, relative_path, issues)

    parsed_job_path = _require_file(job_dir, "parsed_job.json", issues, gate=APP_Q3)
    parsed_job = _check_parsed_job(parsed_job_path, issues) if parsed_job_path is not None else None
    if metadata is not None and parsed_job is not None:
        _check_job_metadata_matches_parsed_job(metadata, parsed_job, issues)

    cover_letter_content: dict[str, object] | None = None
    application_package_content: dict[str, object] | None = None
    for relative_path in REQUIRED_CONTENT_JSON_FILES:
        path = _require_file(job_dir, relative_path, issues, gate=APP_Q3)
        if path is not None:
            value = _check_json(path, relative_path, issues, gate=APP_Q3)
            if relative_path == "typst/cover_letter_content.json":
                cover_letter_content = value
            elif relative_path == "typst/application_package_content.json":
                application_package_content = value

    if cover_letter_content is not None:
        _check_structured_draft_projection(
            job_dir,
            cover_letter_content,
            application_package_content,
            issues,
        )

    for relative_path, markers in REQUIRED_TYPST_MARKERS.items():
        path = _require_file(job_dir, relative_path, issues, gate=APP_Q4)
        if path is not None:
            _check_typst_markers(path, relative_path, markers, issues)
    _check_generated_typst_candidates(job_dir, issues)

    profile_input_hashes = _check_profile_evidence_freshness(profile_dir, issues)
    evidence = load_generated_evidence(profile_dir)
    for relative_path in REQUIRED_MARKDOWN_FILES:
        path = job_dir / relative_path
        if not path.exists():
            continue
        try:
            validate_markdown_citations(relative_path, path.read_text(encoding="utf-8"), evidence)
        except MaterialValidationError as exc:
            issues.append(PackageCheckIssue(relative_path, str(exc), APP_Q2))

    input_hashes = _collect_job_input_hashes(job_dir)
    input_hashes.update(profile_input_hashes)
    return PackageCheckResult(job_dir=job_dir, issues=issues, input_hashes=input_hashes)


def _require_file(
    job_dir: Path,
    relative_path: str,
    issues: list[PackageCheckIssue],
    *,
    gate: str,
) -> Path | None:
    path = job_dir / relative_path
    if not path.exists():
        issues.append(PackageCheckIssue(relative_path, f"missing {relative_path}", gate))
        return None
    if not path.is_file():
        issues.append(PackageCheckIssue(relative_path, "expected a file", gate))
        return None
    return path


def _check_job_metadata(path: Path, issues: list[PackageCheckIssue]) -> dict[str, object] | None:
    relative_path = "job.yaml"
    try:
        value = yaml.safe_load(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, yaml.YAMLError) as exc:
        issues.append(PackageCheckIssue(relative_path, f"invalid job metadata YAML: {exc}", APP_Q1))
        return None
    if not isinstance(value, dict):
        issues.append(PackageCheckIssue(relative_path, "job metadata must be a mapping", APP_Q1))
        return None

    for metadata_field in REQUIRED_JOB_METADATA_FIELDS:
        if metadata_field not in value:
            issues.append(
                PackageCheckIssue(
                    relative_path,
                    f"job metadata missing required field: {metadata_field}",
                    APP_Q1,
                )
            )
    if value.get("status") != "packaged":
        issues.append(
            PackageCheckIssue(
                relative_path,
                "job metadata status must be packaged before readiness checking",
                APP_Q1,
            )
        )
    return value


def _check_job_metadata_matches_parsed_job(
    metadata: dict[str, object],
    parsed_job: dict[str, object],
    issues: list[PackageCheckIssue],
) -> None:
    field_pairs = {
        "title": "title",
        "institution": "institution",
        "deadline": "deadline",
        "source_url": "application_url",
    }
    for metadata_field, parsed_field in field_pairs.items():
        metadata_value = _nonempty_text(metadata.get(metadata_field))
        parsed_value = _nonempty_text(parsed_job.get(parsed_field))
        if metadata_value and parsed_value and metadata_value != parsed_value:
            issues.append(
                PackageCheckIssue(
                    "job.yaml",
                    f"job metadata {metadata_field} does not match parsed_job.json {parsed_field}",
                    APP_Q1,
                )
            )


def _nonempty_text(value: object) -> str:
    if value is None or isinstance(value, (dict, list)):
        return ""
    return str(value).strip()


def _check_json(
    path: Path,
    relative_path: str,
    issues: list[PackageCheckIssue],
    *,
    gate: str,
) -> dict[str, object] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        issues.append(PackageCheckIssue(relative_path, f"invalid JSON: {exc.msg}", gate))
        return None
    if not isinstance(value, dict):
        issues.append(PackageCheckIssue(relative_path, "expected a JSON object", gate))
        return None
    return value


def _check_parsed_job(path: Path, issues: list[PackageCheckIssue]) -> dict[str, object] | None:
    relative_path = "parsed_job.json"
    value = _check_json(path, relative_path, issues, gate=APP_Q3)
    if value is None:
        return None
    try:
        validate_parsed_job(value)
    except ParsedJobValidationError as exc:
        issues.append(PackageCheckIssue(relative_path, f"invalid parsed job: {exc}", APP_Q3))
        return None
    return value


def _check_job_advert(path: Path, issues: list[PackageCheckIssue]) -> None:
    text = path.read_text(encoding="utf-8")
    if job_advert_is_stub(text):
        issues.append(
            PackageCheckIssue(
                "job_advert.md",
                "job advert is still a lead or pending-import stub; add the full advert",
                APP_Q1,
            )
        )


def _check_material_review_blockers(
    path: Path,
    relative_path: str,
    issues: list[PackageCheckIssue],
) -> None:
    if BLOCKER_RE.search(path.read_text(encoding="utf-8")):
        issues.append(
            PackageCheckIssue(
                relative_path,
                "material review contains an explicit BLOCKER",
                APP_Q4,
            )
        )


def _check_typst_markers(
    path: Path,
    relative_path: str,
    markers: tuple[str, ...],
    issues: list[PackageCheckIssue],
) -> None:
    text = path.read_text(encoding="utf-8")
    source_lines = {line.strip() for line in text.splitlines()}
    if (
        relative_path == "typst/cover_letter.typ"
        and STRUCTURED_DRAFT_TYPST_MARKER in source_lines
    ):
        markers = STRUCTURED_COVER_LETTER_TYPST_MARKERS
    if MODERNPRO_IMPORT_RE.search(text) is None:
        issues.append(
            PackageCheckIssue(
                relative_path,
                "missing modernpro Typst import",
                APP_Q4,
            )
        )
    if not any(
        stripped and not stripped.startswith("//") and not stripped.startswith("#import")
        for stripped in (line.strip() for line in text.splitlines())
    ):
        issues.append(
            PackageCheckIssue(
                relative_path,
                "Typst source does not contain a non-comment body line",
                APP_Q4,
            )
        )
    for marker in markers:
        if marker not in source_lines:
            issues.append(
                PackageCheckIssue(
                    relative_path,
                    f"missing stable section marker: {marker}",
                    APP_Q4,
                )
            )


def _check_structured_draft_projection(
    job_dir: Path,
    content: dict[str, object],
    application_content: dict[str, object] | None,
    issues: list[PackageCheckIssue],
) -> None:
    projection = content.get("projection")
    if not isinstance(projection, dict):
        return
    if projection.get("source") != STRUCTURED_DRAFT_PROJECTION_SOURCE:
        return

    for relative_path, hash_key in (
        ("cover_letter_draft.json", "draft_sha256"),
        ("review_findings.json", "review_sha256"),
        ("03_cover_letter_draft.md", "markdown_sha256"),
    ):
        expected_hash = projection.get(hash_key)
        source_path = job_dir / relative_path
        if (
            not isinstance(expected_hash, str)
            or len(expected_hash) != 64
            or not source_path.is_file()
            or hashlib.sha256(source_path.read_bytes()).hexdigest() != expected_hash
        ):
            issues.append(
                PackageCheckIssue(
                    relative_path,
                    "structured Draft projection source or view is missing or has changed",
                    APP_Q4,
                )
            )

    markdown_path = job_dir / "03_cover_letter_draft.md"
    try:
        markdown = markdown_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        markdown = None
    if application_content is not None and (
        application_content.get("cover_letter") != markdown
        or application_content.get("cover_letter_projection") != projection
        or application_content.get("structured_cover_letter_sections")
        != content.get("structured_sections")
    ):
        issues.append(
            PackageCheckIssue(
                "typst/application_package_content.json",
                "structured Draft compatibility views have diverged",
                APP_Q4,
            )
        )

    readiness = _validate_projected_document_readiness(job_dir, projection, issues)
    if (
        readiness is None
        or readiness.state != "reviewed"
        or projection.get("document_readiness_state") != "reviewed"
        or projection.get("requires_human_review") is not False
    ):
        issues.append(
            PackageCheckIssue(
                "typst/cover_letter_content.json",
                (
                    "structured Cover Letter lacks current complete user Review "
                    "dispositions; compatibility projection is not document readiness"
                ),
                APP_Q4,
            )
        )


def _validate_projected_document_readiness(
    job_dir: Path,
    projection: dict[str, object],
    issues: list[PackageCheckIssue],
) -> DocumentReadinessV1 | None:
    if projection.get("review_dispositions_source") != "review_dispositions.yaml":
        issues.append(
            PackageCheckIssue(
                "typst/cover_letter_content.json",
                "structured Review disposition source is invalid",
                APP_Q4,
            )
        )
        return None
    try:
        readiness = DocumentReadinessV1.model_validate(
            projection.get("document_readiness")
        )
    except ValidationError:
        issues.append(
            PackageCheckIssue(
                "typst/cover_letter_content.json",
                "structured document-readiness projection is invalid",
                APP_Q4,
            )
        )
        return None

    if projection.get("document_readiness_state") != readiness.state:
        issues.append(
            PackageCheckIssue(
                "typst/cover_letter_content.json",
                "structured document-readiness state has diverged",
                APP_Q4,
            )
        )
        return None
    if readiness.state != "reviewed":
        return readiness

    try:
        draft_path = job_dir / "cover_letter_draft.json"
        review_path = job_dir / "review_findings.json"
        dispositions_path = job_dir / "review_dispositions.yaml"
        draft_hash = hashlib.sha256(draft_path.read_bytes()).hexdigest()
        review_bytes = review_path.read_bytes()
        review_hash = hashlib.sha256(review_bytes).hexdigest()
        dispositions_bytes = dispositions_path.read_bytes()
        dispositions_hash = hashlib.sha256(dispositions_bytes).hexdigest()
        review = ReviewFindingsV1.model_validate(load_strict_json(review_bytes))
        dispositions = ReviewDispositionsV1.model_validate(
            load_strict_yaml(dispositions_bytes)
        )
        derived = derive_document_readiness(
            review,
            draft_sha256=draft_hash,
            review_findings_sha256=review_hash,
            dispositions=dispositions,
            review_dispositions_sha256=dispositions_hash,
        )
    except (
        InvalidUserFileError,
        OSError,
        UnicodeError,
        ValidationError,
        ValueError,
    ):
        issues.append(
            PackageCheckIssue(
                "review_dispositions.yaml",
                "current Review disposition receipts are missing or invalid",
                APP_Q4,
            )
        )
        return None

    if derived != readiness:
        issues.append(
            PackageCheckIssue(
                "review_dispositions.yaml",
                "document readiness does not match current Draft, Review, and dispositions",
                APP_Q4,
            )
        )
        return None
    return readiness


def _check_generated_typst_candidates(job_dir: Path, issues: list[PackageCheckIssue]) -> None:
    typst_dir = job_dir / "typst"
    if not typst_dir.is_dir():
        return
    for candidate_path in sorted(typst_dir.glob("*.generated.typ")):
        if candidate_path.name in OPTIONAL_STANDALONE_TYPST_CANDIDATES:
            continue
        issues.append(
            PackageCheckIssue(
                f"typst/{candidate_path.name}",
                "generated Typst candidate must be reviewed and reconciled with the editable source",
                APP_Q4,
            )
        )


def _check_placeholders(
    path: Path,
    relative_path: str,
    issues: list[PackageCheckIssue],
    *,
    gate: str,
) -> None:
    text = path.read_text(encoding="utf-8")
    for placeholder in _visible_placeholders(text):
        issues.append(PackageCheckIssue(relative_path, f"placeholder {placeholder} must be resolved", gate))


def _visible_placeholders(text: str) -> list[str]:
    placeholders: list[str] = []
    for match in PLACEHOLDER_RE.finditer(text):
        if match.end() < len(text) and text[match.end()] == "(":
            continue
        placeholders.append(match.group(0))
    return placeholders


def _check_profile_evidence_freshness(
    profile_dir: Path,
    issues: list[PackageCheckIssue],
) -> dict[str, str]:
    input_hashes: dict[str, str] = {}
    manifest_path = profile_dir / "profile.yaml"
    if not manifest_path.is_file():
        issues.append(
            PackageCheckIssue(
                "profile/profile.yaml",
                "profile manifest is missing",
                APP_Q2,
            )
        )
        _record_fallback_evidence_hashes(profile_dir, input_hashes)
        return input_hashes

    _record_input_hash(input_hashes, "profile/profile.yaml", manifest_path)
    try:
        manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, yaml.YAMLError) as exc:
        issues.append(
            PackageCheckIssue(
                "profile/profile.yaml",
                f"invalid profile manifest YAML: {exc}",
                APP_Q2,
            )
        )
        _record_fallback_evidence_hashes(profile_dir, input_hashes)
        return input_hashes
    if not isinstance(manifest, dict):
        issues.append(
            PackageCheckIssue(
                "profile/profile.yaml",
                "profile manifest must be a mapping",
                APP_Q2,
            )
        )
        _record_fallback_evidence_hashes(profile_dir, input_hashes)
        return input_hashes

    sources = manifest.get("sources", {})
    generated = manifest.get("generated", {})
    if not isinstance(sources, dict):
        issues.append(
            PackageCheckIssue(
                "profile/profile.yaml",
                "profile manifest sources must be a mapping",
                APP_Q2,
            )
        )
        sources = {}
    if not isinstance(generated, dict):
        issues.append(
            PackageCheckIssue(
                "profile/profile.yaml",
                "profile manifest generated paths must be a mapping",
                APP_Q2,
            )
        )
        generated = {}

    for source_key, raw_source_path in sources.items():
        source_value = _manifest_path_value(raw_source_path)
        if source_value is None:
            issues.append(
                PackageCheckIssue(
                    "profile/profile.yaml",
                    f"profile source path is invalid: {source_key}",
                    APP_Q2,
                )
            )
            continue
        source_path = _resolve_manifest_path(profile_dir, source_value)
        if not source_path.exists():
            continue
        source_label = _safe_input_label("profile", profile_dir, source_path)
        if not source_path.is_file():
            issues.append(PackageCheckIssue(source_label, "profile source must be a file", APP_Q2))
            continue
        _record_input_hash(input_hashes, source_label, source_path)

        evidence_key = f"{source_key}_evidence"
        raw_evidence_path = generated.get(evidence_key, f"generated/{source_key}.evidence.md")
        evidence_value = _manifest_path_value(raw_evidence_path)
        if evidence_value is None:
            issues.append(
                PackageCheckIssue(
                    "profile/profile.yaml",
                    f"generated evidence path is invalid: {evidence_key}",
                    APP_Q2,
                )
            )
            continue
        evidence_path = _resolve_manifest_path(profile_dir, evidence_value)
        evidence_label = _safe_input_label("profile", profile_dir, evidence_path)
        if not evidence_path.is_file():
            issues.append(
                PackageCheckIssue(
                    evidence_label,
                    f"missing generated evidence for profile source: {source_key}",
                    APP_Q2,
                )
            )
            continue
        _record_input_hash(input_hashes, evidence_label, evidence_path)
        if source_path.stat().st_mtime_ns > evidence_path.stat().st_mtime_ns:
            issues.append(
                PackageCheckIssue(
                    evidence_label,
                    f"generated evidence is stale for profile source: {source_key}",
                    APP_Q2,
                )
            )

    for raw_evidence_path in generated.values():
        evidence_value = _manifest_path_value(raw_evidence_path)
        if evidence_value is None:
            continue
        evidence_path = _resolve_manifest_path(profile_dir, evidence_value)
        _record_input_hash(
            input_hashes,
            _safe_input_label("profile", profile_dir, evidence_path),
            evidence_path,
        )
    _record_fallback_evidence_hashes(profile_dir, input_hashes)
    return input_hashes


def _manifest_path_value(value: object) -> str | None:
    if value is None or isinstance(value, (dict, list)):
        return None
    normalized = str(value).strip()
    return normalized or None


def _resolve_manifest_path(root: Path, value: str) -> Path:
    path = Path(value).expanduser()
    return path if path.is_absolute() else root / path


def _record_fallback_evidence_hashes(profile_dir: Path, input_hashes: dict[str, str]) -> None:
    for evidence_path in sorted((profile_dir / "generated").glob("*.evidence.md")):
        _record_input_hash(
            input_hashes,
            _safe_input_label("profile", profile_dir, evidence_path),
            evidence_path,
        )


def _collect_job_input_hashes(job_dir: Path) -> dict[str, str]:
    relative_paths = {
        *REQUIRED_SOURCE_FILES,
        *REQUIRED_MARKDOWN_FILES,
        *REQUIRED_JSON_FILES,
        *REQUIRED_TYPST_MARKERS,
    }
    generation_manifest = job_dir / "typst" / ".canisend-generated.json"
    if generation_manifest.is_file():
        relative_paths.add("typst/.canisend-generated.json")
    for structured_path in (
        "cover_letter_draft.json",
        "review_findings.json",
        "review_dispositions.yaml",
    ):
        if (job_dir / structured_path).is_file():
            relative_paths.add(structured_path)
    typst_dir = job_dir / "typst"
    if typst_dir.is_dir():
        relative_paths.update(
            f"typst/{candidate.name}"
            for candidate in typst_dir.glob("*.generated.typ")
            if candidate.name not in OPTIONAL_STANDALONE_TYPST_CANDIDATES
        )

    input_hashes: dict[str, str] = {}
    for relative_path in sorted(relative_paths):
        _record_input_hash(
            input_hashes,
            f"job/{relative_path}",
            job_dir / relative_path,
        )
    return input_hashes


def _record_input_hash(input_hashes: dict[str, str], label: str, path: Path) -> None:
    if not path.is_file():
        return
    input_hashes[label] = hashlib.sha256(path.read_bytes()).hexdigest()


def _safe_input_label(prefix: str, root: Path, path: Path) -> str:
    try:
        relative = path.resolve().relative_to(root.resolve())
    except ValueError:
        opaque = hashlib.sha256(str(path.resolve()).encode("utf-8")).hexdigest()[:12]
        return f"{prefix}/external-{opaque}"
    safe_parts = [re.sub(r"[^A-Za-z0-9_.-]", "_", part) for part in relative.parts]
    return f"{prefix}/{'/'.join(safe_parts)}"


def _utc_now() -> str:
    return datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
