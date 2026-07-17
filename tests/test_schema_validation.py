from __future__ import annotations

import json

import pytest

import canisend.schema_validation as schema_validation
from canisend.schema_validation import (
    SchemaCompilationError,
    compiled_schema_validator,
)


def test_compiled_validator_is_reused_for_exact_schema_content() -> None:
    compiled_schema_validator.cache_clear()
    schema_text = json.dumps(
        {"type": "object", "required": ["value"]},
        sort_keys=True,
    )

    first = compiled_schema_validator(schema_text)
    second = compiled_schema_validator(schema_text)

    assert second is first
    assert compiled_schema_validator.cache_info().hits == 1


def test_schema_content_change_compiles_a_new_validator_and_changes_results() -> None:
    compiled_schema_validator.cache_clear()
    requires_a = json.dumps({"type": "object", "required": ["a"]})
    requires_b = json.dumps({"type": "object", "required": ["b"]})

    first = compiled_schema_validator(requires_a)
    second = compiled_schema_validator(requires_b)

    assert second is not first
    assert not list(first.iter_errors({"a": 1}))
    assert list(second.iter_errors({"a": 1}))


def test_invalid_schema_failures_are_not_cached(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    compiled_schema_validator.cache_clear()
    loads = 0
    original = schema_validation.json.loads

    def counted(value: str) -> object:
        nonlocal loads
        loads += 1
        return original(value)

    monkeypatch.setattr(schema_validation.json, "loads", counted)

    for _ in range(2):
        with pytest.raises(SchemaCompilationError):
            compiled_schema_validator('{"type": 7}')

    assert loads == 2
    assert compiled_schema_validator.cache_info().currsize == 0
