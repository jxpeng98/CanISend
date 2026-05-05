import sys

import pytest

from academic_prep.llm import CommandProvider, LLMConfig, OpenAICompatibleProvider, load_llm_config


def test_load_llm_config_defaults_to_openai_compatible_provider():
    config = load_llm_config({})

    assert config.provider == "openai-compatible"
    assert config.timeout_seconds == 300


def test_load_llm_config_reads_command_provider_settings():
    config = load_llm_config(
        {
            "ACADEMIC_PREP_LLM_PROVIDER": "command",
            "ACADEMIC_PREP_LLM_COMMAND": "example-model --json",
            "ACADEMIC_PREP_LLM_TIMEOUT_SECONDS": "120",
        }
    )

    assert config.provider == "command"
    assert config.command == "example-model --json"
    assert config.timeout_seconds == 120


def test_command_provider_sends_prompt_to_configured_command(tmp_path):
    script = tmp_path / "echo_model.py"
    script.write_text(
        "import sys\n"
        "prompt = sys.stdin.read()\n"
        "print(prompt.upper())\n"
    )
    config = LLMConfig(
        provider="command",
        command=f"{sys.executable} {script}",
        timeout_seconds=5,
    )

    response = CommandProvider(config).complete("hello")

    assert response.content == "HELLO"


def test_command_provider_requires_command():
    config = LLMConfig(provider="command", command="", timeout_seconds=5)

    with pytest.raises(ValueError, match="command provider requires"):
        CommandProvider(config).complete("hello")


def test_openai_compatible_provider_posts_chat_completion_request(monkeypatch):
    captured = {}

    class FakeResponse:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, traceback):
            return False

        def read(self):
            return b'{"choices":[{"message":{"content":"parsed json"}}]}'

    def fake_urlopen(request, timeout):
        captured["url"] = request.full_url
        captured["headers"] = dict(request.header_items())
        captured["body"] = request.data.decode("utf-8")
        captured["timeout"] = timeout
        return FakeResponse()

    monkeypatch.setattr("academic_prep.llm.urlopen", fake_urlopen)
    config = LLMConfig(
        provider="openai-compatible",
        openai_api_key="test-key",
        openai_base_url="https://api.example.test/v1",
        openai_model="test-model",
        timeout_seconds=17,
    )

    response = OpenAICompatibleProvider(config).complete("extract job")

    assert response.content == "parsed json"
    assert captured["url"] == "https://api.example.test/v1/chat/completions"
    assert captured["headers"]["Authorization"] == "Bearer test-key"
    assert '"model": "test-model"' in captured["body"]
    assert '"content": "extract job"' in captured["body"]
    assert captured["timeout"] == 17
