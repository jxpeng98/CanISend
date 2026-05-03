import sys

import pytest

from academic_prep.llm import CommandProvider, LLMConfig, load_llm_config


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
