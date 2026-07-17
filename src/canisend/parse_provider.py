from __future__ import annotations

from pathlib import Path
from typing import Mapping

from canisend.llm import LLMProvider
from canisend.parse import ParsedJobValidationError, parse_job_advert_with_provider
from canisend.resource_files import read_resource_text
from canisend.stages.parse_stage import (
    parse_input_fingerprint,
    validate_parse_candidate,
)
from canisend.workspace import load_workspace_config


class ParseProviderError(ValueError):
    """A body-free configured-provider Parse failure."""


class ParseProviderExecutionError(ParseProviderError):
    """The configured provider could not complete the Parse request."""


class ParseProviderResponseError(ParseProviderError):
    """The configured provider returned no acceptable Parse candidate."""


class ParseProviderInputChangedError(ParseProviderError):
    """The declared Parse inputs changed while the provider was running."""


def build_configured_provider_parse_candidate(
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    input_documents: Mapping[str, object],
    provider: LLMProvider,
) -> dict[str, object]:
    """Generate and independently validate one provider-backed Parsed Job."""

    schema_path = load_workspace_config(workspace).path("schema_dir") / "parsed_job.schema.json"
    if parse_input_fingerprint(job_dir, schema_path=schema_path) != input_fingerprint:
        raise ParseProviderInputChangedError("Parse inputs changed before provider execution.")
    metadata = input_documents.get("job.yaml")
    advert = input_documents.get("job_advert.md")
    if not isinstance(metadata, dict) or not isinstance(advert, str):
        raise ParseProviderResponseError("Parse provider inputs have an invalid shape.")
    try:
        prompt_dir = load_workspace_config(workspace).path("prompt_dir")
        prompt = read_resource_text(
            "prompts/job_parser.md",
            local_path=prompt_dir / "job_parser.md",
        )
        candidate = parse_job_advert_with_provider(
            advert_text=advert,
            metadata=metadata,
            provider=provider,
            prompt_text=prompt,
        )
    except (OSError, UnicodeError) as exc:
        raise ParseProviderExecutionError("The Parse provider prompt is unavailable.") from exc
    except ParsedJobValidationError as exc:
        raise ParseProviderResponseError("The Parse provider response is invalid.") from exc
    except Exception as exc:
        raise ParseProviderExecutionError("The configured provider failed during Parse.") from exc
    if parse_input_fingerprint(job_dir, schema_path=schema_path) != input_fingerprint:
        raise ParseProviderInputChangedError("Parse inputs changed during provider execution.")
    try:
        return validate_parse_candidate(
            candidate,
            advert_text=advert,
            schema_path=schema_path,
        )
    except ValueError as exc:
        raise ParseProviderResponseError("The Parse provider candidate is invalid.") from exc
