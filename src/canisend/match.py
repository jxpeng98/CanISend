from __future__ import annotations

from dataclasses import dataclass, field
import re

from canisend.evidence import EvidenceReference


KIND_KEYWORDS: dict[str, list[str]] = {
    "teaching": ["teaching", "teach", "module", "lecture", "seminar", "tutorial",
                  "classroom", "curriculum", "pedagogy", "course", "student feedback"],
    "supervision": ["supervision", "supervise", "dissertation", "thesis", "phd student",
                    "doctoral student", "postgraduate research", "undergraduate project"],
    "research": ["research", "publication", "paper", "article", "journal", "working paper",
                 "manuscript", "monograph", "book chapter", "edited volume", "conference",
                 "proceeding", "peer-review", "referee", "editorial", "editor"],
    "education": ["phd", "doctor", "doctorate", "degree", "qualification", "graduate",
                  "bachelor", "master", "msc", "ma ", "ba ", "bsc", "mphil"],
    "award": ["award", "prize", "scholarship", "fellowship", "grant", "funding",
              "honour", "honor", "distinction", "medal", "recognition"],
    "service": ["service", "committee", "administrative", "departmental", "faculty",
                "senate", "council", "board", "governance", "outreach", "engagement"],
    "employment": ["employment", "position", "appointment", "postdoc", "post-doctoral",
                   "lecturer", "assistant professor", "associate professor", "fellow",
                   "research fellow", "teaching fellow"],
    "skill": ["skill", "method", "methodology", "analysis", "software", "programming",
              "quantitative", "qualitative", "statistical", "econometric", "data",
              "stata", "r ", "python", "matlab", "latex"],
}

KIND_GROUPS: dict[str, list[str]] = {
    "teaching": ["teaching"],
    "supervision": ["supervision"],
    "research": ["research", "publication"],
    "education": ["education"],
    "award": ["award", "grant"],
    "service": ["service"],
    "employment": ["employment", "job"],
    "skill": ["skill", "skills"],
    "statement": ["statement"],
}


@dataclass
class CriterionMatch:
    criterion: str
    coverage: str
    matched_items: list[EvidenceReference]
    suggestion: str


@dataclass
class EvidenceIndex:
    items: list[EvidenceReference]
    _by_kind: dict[str, list[EvidenceReference]] = field(default_factory=dict)
    _by_word: dict[str, list[EvidenceReference]] = field(default_factory=dict)

    def __post_init__(self) -> None:
        for item in self.items:
            for kind in _target_kinds(item):
                self._by_kind.setdefault(kind, []).append(item)
            for word in _tokenize(item.text.lower()):
                self._by_word.setdefault(word, []).append(item)

    def search(self, criterion_text: str, limit: int = 8) -> list[EvidenceReference]:
        target_kinds = _criterion_kinds(criterion_text.lower())
        candidates: dict[str, EvidenceReference] = {}

        for kind in target_kinds:
            for item in self._by_kind.get(kind, []):
                candidates[item.citation] = item

        query_tokens = _tokenize(criterion_text.lower())
        for token in query_tokens:
            for item in self._by_word.get(token, []):
                candidates[item.citation] = item

        scored = [
            (_match_score(criterion_text.lower(), item), item)
            for item in candidates.values()
        ]
        scored.sort(key=lambda pair: pair[0], reverse=True)
        return [item for _score, item in scored[:limit] if _score > 0]

    def match_criterion(self, criterion_text: str) -> CriterionMatch:
        matches = self.search(criterion_text)
        related_kinds = _criterion_kinds(criterion_text.lower())
        direct_matches = [
            item
            for item in matches
            if _direct_overlap_score(criterion_text.lower(), item) > 0
        ]

        if len(direct_matches) >= 2:
            coverage = "strong"
        elif len(direct_matches) == 1:
            coverage = "partial"
        elif matches or related_kinds:
            coverage = "weak"
        else:
            coverage = "missing"

        suggestion = _coverage_suggestion(coverage, criterion_text, matches, related_kinds)
        return CriterionMatch(
            criterion=criterion_text,
            coverage=coverage,
            matched_items=matches,
            suggestion=suggestion,
        )


def coverage_label(matches: list[EvidenceReference], criterion_text: str) -> str:
    direct_match_count = sum(
        1
        for item in matches
        if _direct_overlap_score(criterion_text.lower(), item) > 0
    )
    if direct_match_count >= 2:
        return "strong"
    if direct_match_count == 1:
        return "partial"
    if matches or _criterion_kinds(criterion_text.lower()):
        return "weak"
    return "missing"


def format_fit_report(
    essential_matches: list[CriterionMatch],
    desirable_matches: list[CriterionMatch],
    evidence: list[EvidenceReference],
) -> str:
    lines = ["# Fit Report", ""]

    lines.append("## Essential Criteria Match")
    lines.append("")
    for m in essential_matches:
        icon = _coverage_icon(m.coverage)
        lines.append(f"- {icon} **{m.coverage.upper()}** — {m.criterion}")
        if m.matched_items:
            lines.append(f"  Evidence: {', '.join(f'`{item.citation}`' for item in m.matched_items[:3])}")
        lines.append(f"  {m.suggestion}")
        lines.append("")

    if desirable_matches:
        lines.append("## Desirable Criteria Match")
        lines.append("")
        for m in desirable_matches:
            icon = _coverage_icon(m.coverage)
            lines.append(f"- {icon} **{m.coverage.upper()}** — {m.criterion}")
            if m.matched_items:
                lines.append(f"  Evidence: {', '.join(f'`{item.citation}`' for item in m.matched_items[:3])}")
            lines.append(f"  {m.suggestion}")
            lines.append("")

    lines.append("## Evidence Index")
    lines.append("")
    if evidence:
        lines.append(f"{len(evidence)} evidence items available across {_kind_summary(evidence)}.")
    else:
        lines.append("No profile evidence found. Run `extract-profile-evidence` first.")

    lines.append("")
    lines.append("## Application Risks")
    lines.append("")
    strong_count = sum(1 for m in essential_matches if m.coverage == "strong")
    missing_count = sum(1 for m in essential_matches if m.coverage == "missing")
    if missing_count:
        lines.append(f"- {missing_count} essential criteria have no matching evidence.")
    if strong_count < len(essential_matches) / 2:
        lines.append("- More than half of essential criteria lack strong evidence coverage.")
    lines.append("- Review each match manually before treating this report as final.")
    lines.append("- Add missing evidence to profile files and re-run `extract-profile-evidence`.")

    return "\n".join(lines).rstrip() + "\n"


def format_cover_letter_draft(parsed_job: dict, matches: list[CriterionMatch]) -> str:
    sections: dict[str, list[str]] = {
        "research": [],
        "teaching": [],
        "departmental": [],
        "service": [],
    }
    for m in matches:
        if m.coverage in ("strong", "partial") and m.matched_items:
            kind_text = _primary_kind(m)
            if kind_text in ("research", "publication"):
                sections["research"].extend(_evidence_bullet(item) for item in m.matched_items[:2])
            elif kind_text in ("teaching", "supervision"):
                sections["teaching"].extend(_evidence_bullet(item) for item in m.matched_items[:2])
            elif kind_text == "service":
                sections["service"].extend(_evidence_bullet(item) for item in m.matched_items[:2])

    lines = [
        "# Cover Letter Draft",
        "",
        f"Dear Selection Committee,",
        "",
        f"I am writing to apply for the position of {parsed_job['title']} at {parsed_job['institution']}.",
        "",
        "## Research Fit",
        "",
    ]
    if sections["research"]:
        for text in sections["research"]:
            lines.append(f"- {text}")
    else:
        lines.append("[Add evidence-based research fit using profile file and section references.]")

    lines.extend(["", "## Teaching Fit", ""])
    if sections["teaching"]:
        for text in sections["teaching"]:
            lines.append(f"- {text}")
    else:
        lines.append("[Add evidence-based teaching fit using profile file and section references.]")

    lines.extend(["", "## Departmental Contribution", ""])
    lines.append("[Add specific departmental fit after reviewing the advert and department context.]")

    lines.extend(["", "## Service and Leadership", ""])
    if sections["service"]:
        for text in sections["service"]:
            lines.append(f"- {text}")
    else:
        lines.append("[Add only supported service or leadership evidence.]")

    lines.extend([
        "",
        "Yours sincerely,",
        "",
        "[Applicant name]",
    ])
    return "\n".join(lines).rstrip() + "\n"


def format_cv_notes(parsed_job: dict, matches: list[CriterionMatch]) -> str:
    teaching_items: list[str] = []
    research_items: list[str] = []
    for m in matches:
        if m.coverage in ("strong", "partial") and m.matched_items:
            kind = _primary_kind(m)
            if kind in ("teaching", "supervision"):
                teaching_items.extend(_evidence_bullet(item) for item in m.matched_items[:1])
            elif kind in ("research", "publication"):
                research_items.extend(_evidence_bullet(item) for item in m.matched_items[:1])

    lines = ["# CV Tailoring Notes", ""]

    strong_teaching = sum(1 for m in matches if _primary_kind(m) in ("teaching", "supervision") and m.coverage == "strong")
    strong_research = sum(1 for m in matches if _primary_kind(m) in ("research", "publication") and m.coverage == "strong")

    if strong_teaching > strong_research:
        lines.append("This appears to be a teaching-heavy role. Consider these actions:")
        lines.append("")
        lines.append("1. Move teaching experience and module leadership to the first page.")
        lines.append("2. Place research summary after teaching evidence.")
    else:
        lines.append("This appears to be a research-forward role. Consider these actions:")
        lines.append("")
        lines.append("1. Lead with research publications and projects on the first page.")
        lines.append("2. Place teaching evidence in a supporting section.")

    lines.append("3. Make essential criteria visible in the CV before submission.")
    lines.append("4. Add profile evidence references before using these notes as final guidance.")
    lines.append("")

    if teaching_items:
        lines.append("### Teaching evidence to foreground")
        for item in teaching_items:
            lines.append(f"- {item}")
        lines.append("")
    if research_items:
        lines.append("### Research evidence to foreground")
        for item in research_items:
            lines.append(f"- {item}")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def format_criteria_checklist(
    essential_matches: list[CriterionMatch],
    desirable_matches: list[CriterionMatch],
) -> str:
    lines = [
        "# Criteria Coverage Checklist",
        "",
        "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |",
        "|---|---|---|---|---|",
    ]
    for m in essential_matches:
        source = f"`{m.matched_items[0].citation}`" if m.matched_items else "Not yet linked"
        risk = _coverage_risk(m.coverage, essential=True)
        lines.append(f"| {m.criterion} | {m.coverage} | {source} | {risk} | {m.suggestion} |")

    for m in desirable_matches:
        source = f"`{m.matched_items[0].citation}`" if m.matched_items else "Not yet linked"
        risk = _coverage_risk(m.coverage, essential=False)
        lines.append(f"| {m.criterion} | {m.coverage} | {source} | {risk} | {m.suggestion} |")

    if len(essential_matches) + len(desirable_matches) == 0:
        lines.append("| No criteria extracted | missing | Not available | High | Review the advert manually. |")

    return "\n".join(lines).rstrip() + "\n"


def _target_kinds(item: EvidenceReference) -> list[str]:
    text_lower = item.text.lower()
    found: list[str] = []
    for kind, keywords in KIND_KEYWORDS.items():
        if any(keyword in text_lower for keyword in keywords):
            found.append(kind)
    if not found:
        found.append("unknown")
    return found


def _criterion_kinds(text: str) -> list[str]:
    found: list[str] = []
    for kind, keywords in KIND_KEYWORDS.items():
        if any(keyword in text for keyword in keywords):
            found.append(kind)
    return found


def _tokenize(text: str) -> list[str]:
    tokens = re.findall(r"[a-z]{3,}", text)
    stopwords = {
        "the", "and", "for", "with", "that", "this", "from", "have", "been",
        "will", "would", "each", "which", "their", "about", "than", "also",
        "into", "other", "more", "some", "such", "only", "over", "very",
        "your", "its", "are", "has", "had", "not", "but", "was", "all",
        "can", "who", "what", "when", "how",
    }
    return [t for t in tokens if t not in stopwords]


def _match_score(query: str, item: EvidenceReference) -> int:
    score = 0
    item_text = item.text.lower()
    query_tokens = _tokenize(query)
    for token in query_tokens:
        if len(token) >= 4 and token in item_text:
            score += 1
    query_kinds = _criterion_kinds(query)
    item_kinds = _target_kinds(item)
    if set(query_kinds) & set(item_kinds):
        score += 3
    return score


def _direct_overlap_score(query: str, item: EvidenceReference) -> int:
    item_text = item.text.lower()
    return sum(
        1
        for token in _tokenize(query)
        if len(token) >= 4 and token in item_text
    )


def _coverage_suggestion(
    coverage: str,
    criterion: str,
    matches: list[EvidenceReference],
    kinds: list[str],
) -> str:
    if coverage == "strong":
        return "Evidence found. Verify claims are proportional before finalizing."
    if coverage == "partial":
        return "Some evidence found. Add more detail or context to strengthen this criterion."
    if coverage == "weak":
        area = kinds[0] if kinds else "relevant"
        return f"No direct evidence found. Add {area} evidence to profile files and re-run extraction."
    return "No evidence available. Add this experience or qualification to profile files."


def _coverage_icon(coverage: str) -> str:
    return {"strong": "✅", "partial": "⚠️", "weak": "🔸", "missing": "❌"}.get(coverage, "❓")


def _coverage_risk(coverage: str, essential: bool) -> str:
    if coverage == "strong":
        return "Low"
    if coverage == "partial":
        return "Medium" if essential else "Low"
    if coverage == "weak":
        return "High" if essential else "Medium"
    return "High" if essential else "Medium"


def _kind_summary(evidence: list[EvidenceReference]) -> str:
    kinds: set[str] = set()
    for item in evidence:
        kinds.update(_target_kinds(item))
    kinds.discard("unknown")
    return ", ".join(sorted(kinds)) if kinds else "no categorized kinds"


def _evidence_bullet(item: EvidenceReference) -> str:
    return f"{item.text} (`{item.citation}`)"


def _primary_kind(match: CriterionMatch) -> str:
    if not match.matched_items:
        return "unknown"
    kinds = _target_kinds(match.matched_items[0])
    for priority in KIND_KEYWORDS:
        if priority in kinds:
            return priority
    return kinds[0] if kinds else "unknown"
