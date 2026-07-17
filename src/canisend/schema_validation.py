"""Content-addressed JSON Schema compilation for repeated stage validation."""

from __future__ import annotations

from functools import lru_cache
import json

from jsonschema import Draft202012Validator
from jsonschema.exceptions import SchemaError


class SchemaCompilationError(ValueError):
    """Raised when configured schema text is not valid Draft 2020-12 JSON Schema."""


@lru_cache(maxsize=128)
def compiled_schema_validator(schema_text: str) -> Draft202012Validator:
    """Return a checked validator keyed by the schema's exact text content."""

    try:
        schema = json.loads(schema_text)
        Draft202012Validator.check_schema(schema)
    except (json.JSONDecodeError, SchemaError) as exc:
        raise SchemaCompilationError("The configured JSON Schema is invalid.") from exc
    return Draft202012Validator(schema)
