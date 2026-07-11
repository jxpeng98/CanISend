from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import re
from typing import Any

import yaml

from canisend.llm import LLMProvider
from canisend.resource_files import read_resource_text


@dataclass(frozen=True)
class EvidenceItem:
    source_file: str
    section: str
    kind: str
    text: str


@dataclass(frozen=True)
class EvidenceReference:
    source_file: str
    section: str
    text: str
    item_id: str = ""

    @property
    def citation(self) -> str:
        section_citation = f"{self.source_file}#{self.section}"
        if not self.item_id:
            return section_citation
        return f"{section_citation}/{self.item_id}"

    @property
    def section_citation(self) -> str:
        return f"{self.source_file}#{self.section}"


class EvidenceAugmentationError(ValueError):
    pass


EVIDENCE_BLOCK_FUNCTIONS = {
    "award",
    "conference",
    "dated-entry",
    "education",
    "employment",
    "entry",
    "event",
    "experience",
    "grant",
    "job",
    "presentation",
    "project",
    "publication",
    "reference",
    "references",
    "research",
    "service",
    "skill",
    "skills",
    "supervision",
    "talk",
    "teaching",
}


def extract_typst_evidence(path: Path) -> list[EvidenceItem]:
    return _extract_typst_evidence_from_lines(
        path=path,
        lines=path.read_text(encoding="utf-8").splitlines(),
        initial_section="Unsectioned",
        infer_section_titles=False,
    )


def _extract_typst_evidence_from_lines(
    *,
    path: Path,
    lines: list[str],
    initial_section: str,
    infer_section_titles: bool,
) -> list[EvidenceItem]:
    section = initial_section
    evidence: list[EvidenceItem] = []
    statement_lines: list[str] = []
    content_started = section != "Unsectioned"

    def flush_statement() -> None:
        nonlocal statement_lines
        text = _clean_statement_lines(statement_lines)
        if text:
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind="statement",
                    text=text,
                )
            )
        statement_lines = []

    index = 0
    while index < len(lines):
        raw_line = lines[index]
        line = raw_line.strip()
        if not line:
            index += 1
            continue

        if infer_section_titles:
            title_match = re.match(r"title:\s*(.+?)(?:,)?$", line)
            if title_match:
                title = _clean_typst_inline(title_match.group(1))
                if title:
                    flush_statement()
                    section = title
                    content_started = True
                index += 1
                continue

        section_match = re.match(r'#section\("([^"]+)"\)', line)
        if section_match:
            flush_statement()
            section = section_match.group(1)
            content_started = True
            index += 1
            continue

        heading_match = re.match(r"=+\s+(.+)", line)
        if heading_match:
            flush_statement()
            section = heading_match.group(1).strip()
            content_started = True
            index += 1
            continue

        block_name = _typst_call_name(line)
        if block_name in EVIDENCE_BLOCK_FUNCTIONS:
            flush_statement()
            block_text, index = _collect_typst_call(lines, index)
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind=block_name,
                    text=_clean_typst_inline(_strip_typst_call(block_name, block_text)),
                )
            )
            continue

        if line.startswith("#"):
            if "(" in line and _paren_delta(line) > 0:
                block_text, index = _collect_typst_call(lines, index)
                evidence.extend(
                    _extract_typst_evidence_from_lines(
                        path=path,
                        lines=_inner_typst_lines(block_text),
                        initial_section=section,
                        infer_section_titles=True,
                    )
                )
            else:
                index += 1
            continue

        if line.startswith("+ @"):
            flush_statement()
            evidence.append(
                EvidenceItem(
                    source_file=str(path),
                    section=section,
                    kind="publication",
                    text=line[2:].strip(),
                )
            )
            index += 1
            continue

        if content_started and _is_statement_line(line):
            statement_lines.append(line)

        index += 1

    flush_statement()

    return evidence


def extract_profile_evidence(
    profile_dir: Path,
    *,
    llm_provider: LLMProvider | None = None,
    prompt_dir: Path = Path("prompts"),
) -> list[Path]:
    manifest_path = profile_dir / "profile.yaml"
    manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
    sources = manifest.get("sources", {})
    generated = manifest.get("generated", {})
    written: list[Path] = []

    for source_key, source_value in sources.items():
        if not str(source_value).endswith(".typ"):
            continue
        source_path = profile_dir / source_value
        output_path = _output_path_for_source(profile_dir, source_key, generated)
        source_bytes = source_path.read_bytes()
        try:
            source_text = source_bytes.decode("utf-8")
        except UnicodeError as exc:
            raise EvidenceAugmentationError("Profile sources must be valid UTF-8 text") from exc
        evidence = _extract_typst_evidence_from_lines(
            path=source_path,
            lines=source_text.splitlines(),
            initial_section="Unsectioned",
            infer_section_titles=False,
        )
        if llm_provider is not None:
            evidence = augment_typst_evidence_with_provider(
                source_key=str(source_key),
                source_path=source_path,
                source_text=source_text,
                local_evidence=evidence,
                provider=llm_provider,
                prompt_text=read_resource_text(
                    "prompts/profile_evidence_augmenter.md",
                    local_path=prompt_dir / "profile_evidence_augmenter.md",
                ),
            )
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            _evidence_markdown(
                source_key,
                evidence,
                source_sha256=sha256(source_bytes).hexdigest(),
            ),
            encoding="utf-8",
        )
        written.append(output_path)

    return written


def augment_typst_evidence_with_provider(
    *,
    source_key: str,
    source_path: Path,
    source_text: str,
    local_evidence: list[EvidenceItem],
    provider: LLMProvider,
    prompt_text: str,
) -> list[EvidenceItem]:
    prompt = _render_augmentation_prompt(
        prompt_text,
        source_key=source_key,
        source_path=source_path,
        source_text=source_text,
        local_evidence=local_evidence,
    )
    response = provider.complete(prompt)
    parsed = _loads_evidence_augmentation_json(response.content)
    raw_items = parsed.get("items")
    if not isinstance(raw_items, list):
        raise EvidenceAugmentationError("LLM evidence augmenter returned JSON without an items list")

    augmented = list(local_evidence)
    seen = {_normalize_evidence_text(item.text) for item in augmented}
    normalized_source = _normalize_source_support(source_text)

    for index, raw_item in enumerate(raw_items):
        if not isinstance(raw_item, dict):
            raise EvidenceAugmentationError(f"LLM evidence augmenter items[{index}] must be an object")

        text = _string_field(raw_item, "text")
        source_quote = _string_field(raw_item, "source_text")
        if not text or not source_quote:
            continue
        normalized_quote = _normalize_source_support(source_quote)
        if not normalized_quote or normalized_quote not in normalized_source:
            continue

        cleaned_text = _clean_typst_inline(text)
        normalized_text = _normalize_evidence_text(cleaned_text)
        if not cleaned_text or normalized_text in seen:
            continue

        section = _clean_typst_inline(_string_field(raw_item, "section")) or "Unsectioned"
        kind = _safe_evidence_kind(_string_field(raw_item, "kind")) or "llm-augmented"
        augmented.append(
            EvidenceItem(
                source_file=str(source_path),
                section=section,
                kind=kind,
                text=cleaned_text,
            )
        )
        seen.add(normalized_text)

    return augmented


def load_generated_evidence(profile_dir: Path) -> list[EvidenceReference]:
    references: list[EvidenceReference] = []
    for path in _generated_evidence_paths(profile_dir):
        section = "Unsectioned"
        relative = path.relative_to(profile_dir.parent) if profile_dir.parent in path.parents else path
        for raw_line in path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if line.startswith("## "):
                section = line[3:].strip()
            elif line.startswith("- "):
                item_id, text = _parse_evidence_markdown_item(line)
                references.append(
                    EvidenceReference(
                        source_file=str(relative),
                        section=section,
                        item_id=item_id,
                        text=text,
                    )
                )
    return references


def _generated_evidence_paths(profile_dir: Path) -> list[Path]:
    manifest_paths = _manifest_generated_evidence_paths(profile_dir)
    if manifest_paths is not None:
        return manifest_paths

    generated_dir = profile_dir / "generated"
    if not generated_dir.exists():
        return []
    return sorted(generated_dir.glob("*.evidence.md"))


def _manifest_generated_evidence_paths(profile_dir: Path) -> list[Path] | None:
    manifest_path = profile_dir / "profile.yaml"
    if not manifest_path.exists():
        return None

    try:
        manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
    except yaml.YAMLError:
        return None
    if not isinstance(manifest, dict):
        return None

    sources = manifest.get("sources")
    generated = manifest.get("generated")
    if sources is None and generated is None:
        return None
    if sources is not None and not isinstance(sources, dict):
        return None
    if generated is not None and not isinstance(generated, dict):
        return None

    source_map = sources or {}
    generated_map = generated or {}
    paths: list[Path] = []

    for source_key, source_value in source_map.items():
        if not str(source_value).endswith(".typ"):
            continue
        paths.append(_output_path_for_source(profile_dir, str(source_key), generated_map))

    paths.extend(profile_dir / Path(str(path)) for path in generated_map.values())

    unique_paths = sorted({path for path in paths if path.exists()})
    return unique_paths


def _render_augmentation_prompt(
    prompt_text: str,
    *,
    source_key: str,
    source_path: Path,
    source_text: str,
    local_evidence: list[EvidenceItem],
) -> str:
    rendered = prompt_text.replace("{source_key}", source_key)
    rendered = rendered.replace("{source_path}", source_path.name)
    rendered = rendered.replace("{source_text}", source_text)
    rendered = rendered.replace("{local_evidence}", _evidence_items_json(local_evidence))
    return rendered


def _evidence_items_json(evidence: list[EvidenceItem]) -> str:
    data = [
        {
            "section": item.section,
            "kind": item.kind,
            "text": item.text,
        }
        for item in evidence
    ]
    return json.dumps(data, indent=2, default=str)


def _loads_evidence_augmentation_json(content: str) -> dict[str, Any]:
    stripped = content.strip()
    if not stripped:
        raise EvidenceAugmentationError("LLM evidence augmenter returned an empty response")
    if stripped.startswith("```"):
        stripped = _strip_json_fence(stripped)
    else:
        fenced = _single_json_fence(stripped)
        if fenced is not None:
            stripped = fenced
    try:
        parsed = json.loads(stripped)
    except json.JSONDecodeError as exc:
        raise EvidenceAugmentationError(
            f"LLM evidence augmenter returned invalid JSON: {exc.msg}. Return exactly one JSON object."
        ) from exc
    if not isinstance(parsed, dict):
        raise EvidenceAugmentationError("LLM evidence augmenter returned JSON that is not an object")
    return parsed


def _strip_json_fence(content: str) -> str:
    lines = content.splitlines()
    if lines and lines[0].startswith("```"):
        lines = lines[1:]
    if lines and lines[-1].startswith("```"):
        lines = lines[:-1]
    return "\n".join(lines).strip()


def _single_json_fence(content: str) -> str | None:
    matches = re.findall(r"```(?:json)?\s*\n(.*?)\n```", content, flags=re.DOTALL | re.IGNORECASE)
    if len(matches) != 1:
        return None
    return matches[0].strip()


def _string_field(item: dict[str, Any], field: str) -> str:
    value = item.get(field, "")
    if value is None:
        return ""
    if isinstance(value, (dict, list)):
        return ""
    return str(value).strip()


def _safe_evidence_kind(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_-]+", "-", value.strip().lower()).strip("-")
    return cleaned[:50]


def _normalize_evidence_text(text: str) -> str:
    return re.sub(r"\s+", " ", _clean_typst_inline(text)).strip().casefold()


def _normalize_source_support(text: str) -> str:
    cleaned = _clean_typst_inline(text)
    cleaned = re.sub(r"[^\w\s]", " ", cleaned)
    return re.sub(r"\s+", " ", cleaned).strip().casefold()


def _typst_call_name(line: str) -> str:
    match = re.match(r"#([A-Za-z][\w-]*)\s*\(", line)
    return match.group(1) if match else ""


def _collect_typst_call(lines: list[str], start_index: int) -> tuple[str, int]:
    collected: list[str] = []
    balance = 0

    for index in range(start_index, len(lines)):
        line = lines[index].strip()
        collected.append(line)
        balance += _paren_delta(line)
        if balance <= 0:
            return "\n".join(collected), index + 1

    return "\n".join(collected), len(lines)


def _inner_typst_lines(block_text: str) -> list[str]:
    lines = block_text.splitlines()
    if len(lines) <= 2:
        return []
    return lines[1:-1]


def _paren_delta(line: str) -> int:
    return line.count("(") - line.count(")")


def _strip_typst_call(name: str, call_text: str) -> str:
    text = call_text.strip()
    text = re.sub(rf"^#{re.escape(name)}\s*\(", "", text, count=1)
    if text.endswith(")"):
        text = text[:-1]
    return text


def _is_statement_line(line: str) -> bool:
    if line.startswith(("//", "%", "|")):
        return False
    if re.match(r"^[A-Za-z0-9_-]+\s*:", line):
        return False
    return bool(re.search(r"[A-Za-z]", line))


def _clean_statement_lines(lines: list[str]) -> str:
    cleaned: list[str] = []
    for line in lines:
        if line.startswith(("- ", "+ ")):
            line = line[2:].strip()
        cleaned.append(line)
    return _clean_typst_inline(" ".join(cleaned))


def _clean_typst_inline(value: str) -> str:
    value = value.replace("[", "").replace("]", "")
    value = value.replace('"', "")
    value = value.replace("\n", " ")
    value = re.sub(r",\s*", ", ", value)
    value = re.sub(r"\s+", " ", value)
    return value.strip(" ,")


def _output_path_for_source(profile_dir: Path, source_key: str, generated: dict[str, str]) -> Path:
    manifest_key = f"{source_key}_evidence"
    output = generated.get(manifest_key, f"generated/{source_key}.evidence.md")
    return profile_dir / output


def _evidence_markdown(
    source_key: str,
    evidence: list[EvidenceItem],
    *,
    source_sha256: str,
) -> str:
    lines = [
        f"# Evidence: {source_key}",
        f"<!-- canisend-source-sha256: {source_sha256} -->",
        "",
    ]
    current_section = ""

    item_prefix = _evidence_item_prefix(source_key)
    for index, item in enumerate(evidence, start=1):
        if item.section != current_section:
            current_section = item.section
            lines.extend([f"## {current_section}", ""])
        item_id = f"{item_prefix}-{index:03d}"
        lines.append(f"- [{item_id}] `{item.kind}`: {item.text}")

    if not evidence:
        lines.append("_No evidence extracted._")

    return "\n".join(lines).rstrip() + "\n"


def _evidence_item_prefix(source_key: str) -> str:
    prefix = re.sub(r"[^A-Za-z0-9]+", "-", source_key).strip("-").lower()
    return prefix or "item"


def _parse_evidence_markdown_item(line: str) -> tuple[str, str]:
    item_match = re.match(r"- \[([^\]]+)\]\s+(.*)", line)
    if item_match:
        return item_match.group(1).strip(), item_match.group(2).strip()
    return "", line[2:].strip()
