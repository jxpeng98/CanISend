import json

import pytest

from canisend.llm import LLMResponse
from canisend.parse import ParsedJobValidationError, parse_job_advert_with_provider


class FakeProvider:
    def __init__(self, content: str) -> None:
        self.content = content
        self.prompt = ""

    def complete(self, prompt: str) -> LLMResponse:
        self.prompt = prompt
        return LLMResponse(content=self.content, provider="fake")


def valid_parsed_job() -> dict:
    return {
        "title": "Lecturer in Economics",
        "institution": "University X",
        "department": "Department of Economics",
        "location": "United Kingdom",
        "deadline": "2026-06-15",
        "salary": "Grade 7",
        "contract_type": "Permanent",
        "role_type": "Lecturer",
        "research_fields": ["Economics"],
        "teaching_fields": ["Econometrics"],
        "essential_criteria": [
            {"criterion": "PhD in Economics", "source_text": "PhD in Economics"}
        ],
        "desirable_criteria": [],
        "required_documents": ["CV", "Cover letter"],
        "application_url": "https://example.edu/jobs/123",
        "unknown_fields": [],
        "notes": "",
    }


def test_parse_job_advert_with_provider_uses_prompt_and_validates_json():
    provider = FakeProvider(json.dumps(valid_parsed_job()))
    metadata = {"title": "Lecturer in Economics", "institution": "University X"}

    parsed_job = parse_job_advert_with_provider(
        advert_text="Essential criteria: PhD in Economics",
        metadata=metadata,
        provider=provider,
        prompt_text="Extract JSON for {job_metadata} and {job_advert}",
    )

    assert parsed_job["title"] == "Lecturer in Economics"
    assert parsed_job["essential_criteria"][0]["criterion"] == "PhD in Economics"
    assert "Essential criteria: PhD in Economics" in provider.prompt
    assert "University X" in provider.prompt


def test_parse_job_advert_with_provider_allows_json_braces_in_prompt_template():
    provider = FakeProvider(json.dumps(valid_parsed_job()))

    parse_job_advert_with_provider(
        advert_text="Advert",
        metadata={"title": "Lecturer"},
        provider=provider,
        prompt_text='Return this shape: {"title": "unknown"}\n{job_metadata}\n{job_advert}',
    )

    assert '{"title": "unknown"}' in provider.prompt
    assert "Lecturer" in provider.prompt
    assert "Advert" in provider.prompt


def test_parse_job_advert_with_provider_rejects_missing_required_fields():
    provider = FakeProvider(json.dumps({"title": "Lecturer in Economics"}))

    with pytest.raises(ParsedJobValidationError, match="missing required field: institution"):
        parse_job_advert_with_provider(
            advert_text="Advert",
            metadata={},
            provider=provider,
            prompt_text="Extract JSON for {job_metadata} and {job_advert}",
        )
