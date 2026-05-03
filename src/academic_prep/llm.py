from __future__ import annotations

from dataclasses import dataclass
from os import environ
import shlex
import subprocess
from typing import Mapping


@dataclass(frozen=True)
class LLMConfig:
    provider: str = "openai-compatible"
    openai_api_key: str = ""
    openai_base_url: str = ""
    openai_model: str = ""
    command: str = ""
    timeout_seconds: int = 300


@dataclass(frozen=True)
class LLMResponse:
    content: str
    provider: str


class LLMProvider:
    def complete(self, prompt: str) -> LLMResponse:
        raise NotImplementedError


class CommandProvider(LLMProvider):
    def __init__(self, config: LLMConfig) -> None:
        self.config = config

    def complete(self, prompt: str) -> LLMResponse:
        if not self.config.command.strip():
            raise ValueError("command provider requires ACADEMIC_PREP_LLM_COMMAND")

        result = subprocess.run(
            shlex.split(self.config.command),
            input=prompt,
            text=True,
            capture_output=True,
            timeout=self.config.timeout_seconds,
            check=False,
        )
        if result.returncode != 0:
            stderr = result.stderr.strip()
            raise RuntimeError(f"LLM command failed with exit code {result.returncode}: {stderr}")

        return LLMResponse(content=result.stdout.strip(), provider="command")


class OpenAICompatibleProvider(LLMProvider):
    def __init__(self, config: LLMConfig) -> None:
        self.config = config

    def complete(self, prompt: str) -> LLMResponse:
        if not self.config.openai_api_key:
            raise ValueError("openai-compatible provider requires OPENAI_API_KEY")
        if not self.config.openai_model:
            raise ValueError("openai-compatible provider requires OPENAI_MODEL")
        raise NotImplementedError("OpenAI-compatible API calls will be implemented in the parser milestone.")


def load_llm_config(env: Mapping[str, str] | None = None) -> LLMConfig:
    source = environ if env is None else env
    timeout_value = source.get("ACADEMIC_PREP_LLM_TIMEOUT_SECONDS", "300")
    return LLMConfig(
        provider=source.get("ACADEMIC_PREP_LLM_PROVIDER", "openai-compatible"),
        openai_api_key=source.get("OPENAI_API_KEY", ""),
        openai_base_url=source.get("OPENAI_BASE_URL", ""),
        openai_model=source.get("OPENAI_MODEL", ""),
        command=source.get("ACADEMIC_PREP_LLM_COMMAND", ""),
        timeout_seconds=int(timeout_value),
    )


def provider_from_config(config: LLMConfig) -> LLMProvider:
    if config.provider == "command":
        return CommandProvider(config)
    if config.provider == "openai-compatible":
        return OpenAICompatibleProvider(config)
    raise ValueError(f"Unsupported LLM provider: {config.provider}")
