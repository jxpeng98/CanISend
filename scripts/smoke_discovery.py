#!/usr/bin/env python3
from __future__ import annotations

import argparse
from datetime import UTC, datetime
import json
from pathlib import Path
import subprocess
import sys

import yaml

from canisend.discovery.adapters import discovery_adapter
from canisend.discovery.refresh_models import DiscoverySourcesV1
from canisend.discovery.search_models import DiscoverySearchV1


DISCOVERY_SCHEMAS = (
    "job-lead-v2.schema.json",
    "lead-catalog-v1.schema.json",
    "discovery-sources-v1.schema.json",
    "lead-batch-v1.schema.json",
    "discovery-cache-v1.schema.json",
    "discovery-refresh-report-v1.schema.json",
    "discovery-import-report-v1.schema.json",
    "discovery-search-v1.schema.json",
)
OBSERVED_AT = datetime(2026, 7, 15, 13, 0, tzinfo=UTC)


class SmokeFailure(RuntimeError):
    """An offline discovery smoke failure."""


def _run_json(canisend: str, *args: str) -> dict[str, object]:
    result = subprocess.run(
        [canisend, *args, "--format", "json"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise SmokeFailure(
            f"CanISend command failed with exit code {result.returncode}: "
            f"{' '.join(args[:2])}"
        )
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise SmokeFailure("CanISend command did not return one JSON document.") from exc
    if not isinstance(payload, dict) or payload.get("ok") is not True:
        raise SmokeFailure("CanISend command returned an unsuccessful AgentResponse.")
    return payload


def _require_file(path: Path) -> None:
    if not path.is_file():
        raise SmokeFailure(f"Required installed discovery resource is missing: {path.name}")


def _validate_installed_resources(workspace: Path) -> None:
    for schema_name in DISCOVERY_SCHEMAS:
        _require_file(workspace / "schemas" / schema_name)
    for example_name in (
        "README.md",
        "discovery-sources.example.yaml",
        "normalized-search.example.json",
        "local-leads.example.csv",
        "greenhouse-list.fixture.json",
        "lever-list.fixture.json",
    ):
        _require_file(workspace / "examples" / "discovery" / example_name)
    _require_file(workspace / "docs" / "stage4-migration.md")
    _require_file(workspace / "agent-skills" / "canisend-job-intake" / "SKILL.md")


def _validate_adapter_fixtures(workspace: Path) -> None:
    examples = workspace / "examples" / "discovery"
    sources = DiscoverySourcesV1.model_validate(
        yaml.safe_load(
            (examples / "discovery-sources.example.yaml").read_text(
                encoding="utf-8"
            )
        )
    )
    api_sources = {
        source.kind: source for source in sources.sources if source.kind != "rss_atom"
    }
    expected = {
        "greenhouse": (
            "greenhouse-list.fixture.json",
            "https://boards-api.greenhouse.io/v1/boards/"
            "example_university/jobs?content=true",
        ),
        "lever": (
            "lever-list.fixture.json",
            "https://api.lever.co/v0/postings/example?limit=10000&mode=json",
        ),
    }
    for kind, (fixture_name, request_url) in expected.items():
        source = api_sources[kind]
        adapter = discovery_adapter(source)
        if adapter.request_url(source) != request_url:
            raise SmokeFailure(f"Installed {kind} adapter request contract changed.")
        leads = adapter.parse(
            source,
            (examples / fixture_name).read_bytes(),
            content_type="application/json",
            observed_at=OBSERVED_AT,
        )
        if len(leads) != 1 or leads[0].schema_version != "2.0.0":
            raise SmokeFailure(f"Installed {kind} fixture did not map to one Lead v2.")


def _scan_private_safe_artifacts(workspace: Path) -> None:
    artifacts = sorted((workspace / "job_leads").rglob("*.json"))
    rendered = "\n".join(path.read_text(encoding="utf-8") for path in artifacts).casefold()
    forbidden = (
        "authorization",
        "api_key",
        "access_token",
        "connector_id",
        "session_id",
        "applyurl",
        "applicationform",
        str(workspace.resolve()).casefold(),
        workspace.resolve().as_posix().casefold(),
    )
    if any(value and value in rendered for value in forbidden):
        raise SmokeFailure("Discovery artifacts retained a forbidden private transport field.")


def _resolve_new_workspace(workspace: Path) -> Path:
    resolved = workspace.resolve()
    if resolved.exists():
        raise SmokeFailure("Discovery smoke workspace must not already exist.")
    return resolved


def run_smoke(canisend: str, workspace: Path) -> None:
    workspace = _resolve_new_workspace(workspace)

    init = subprocess.run(
        [canisend, "init-workspace", "--workspace", str(workspace)],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if init.returncode != 0:
        raise SmokeFailure("Could not initialize the offline discovery smoke workspace.")

    _validate_installed_resources(workspace)
    _validate_adapter_fixtures(workspace)

    examples = workspace / "examples" / "discovery"
    csv_response = _run_json(
        canisend,
        "discovery",
        "import",
        "--workspace",
        str(workspace),
        "--input",
        str(examples / "local-leads.example.csv"),
        "--source-name",
        "Packaged Synthetic Search",
    )
    if csv_response.get("extensions", {}).get("canisend.discovery.imported_records") != 2:  # type: ignore[union-attr]
        raise SmokeFailure("Packaged CSV discovery example did not import two leads.")

    search = DiscoverySearchV1.model_validate_json(
        (examples / "normalized-search.example.json").read_text(encoding="utf-8")
    )
    search_response = _run_json(
        canisend,
        "discovery",
        "import-search",
        "--workspace",
        str(workspace),
        "--input",
        str(examples / "normalized-search.example.json"),
    )
    if search_response.get("extensions", {}).get("canisend.discovery.imported_records") != search.result_count:  # type: ignore[union-attr]
        raise SmokeFailure("Packaged host-search example did not import every result.")

    catalog_path = workspace / "job_leads" / "catalog.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    leads = catalog.get("leads", [])
    if not isinstance(leads, list) or len(leads) != 2:
        raise SmokeFailure("CSV and host-search inputs did not deduplicate to two catalog leads.")
    selected = next(
        (lead for lead in leads if lead.get("title") == "Lecturer in Economics"),
        None,
    )
    if not isinstance(selected, dict) or not isinstance(selected.get("lead_id"), str):
        raise SmokeFailure("Catalog did not expose the expected stable lead ID.")

    intake = _run_json(
        canisend,
        "new-job-from-lead",
        "--workspace",
        str(workspace),
        "--leads-file",
        "job_leads/catalog.json",
        "--lead-id",
        selected["lead_id"],
        "--institution",
        "Example University",
        "--deadline",
        "2026-08-31",
    )
    job = intake.get("job")
    if not isinstance(job, dict) or job.get("status") != "lead_imported":
        raise SmokeFailure("Stable lead selection did not create one lead-only job.")
    if "job_advert.md" not in intake.get("missing_fields", []):
        raise SmokeFailure("Lead-only intake did not preserve the full-advert blocker.")

    _scan_private_safe_artifacts(workspace)
    print(
        "Stage 4 discovery smoke passed: packaged resources, adapters, local import, "
        "host search, catalog dedupe, stable selection, and full-advert boundary."
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run the offline Stage 4 discovery smoke.")
    parser.add_argument("--canisend", default="canisend")
    parser.add_argument("--workspace", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        run_smoke(args.canisend, args.workspace)
    except SmokeFailure as exc:
        print(f"Stage 4 discovery smoke failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
