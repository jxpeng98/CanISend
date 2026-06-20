from __future__ import annotations

from dataclasses import dataclass
import json
from typing import Mapping
from urllib.request import Request, urlopen

from packaging.version import InvalidVersion, Version


PYPI_JSON_URL = "https://pypi.org/pypi/canisend/json"


@dataclass(frozen=True)
class PyPIVersionInfo:
    stable: str | None
    prerelease: str | None
    source: str = PYPI_JSON_URL


def latest_versions_from_releases(
    releases: Mapping[str, object],
    *,
    source: str = PYPI_JSON_URL,
) -> PyPIVersionInfo:
    stable_versions: list[Version] = []
    prerelease_versions: list[Version] = []

    for release in releases:
        try:
            version = Version(release)
        except InvalidVersion:
            continue
        if version.is_prerelease:
            prerelease_versions.append(version)
        else:
            stable_versions.append(version)

    stable = str(max(stable_versions)) if stable_versions else None
    prerelease = str(max(prerelease_versions)) if prerelease_versions else None
    return PyPIVersionInfo(stable=stable, prerelease=prerelease, source=source)


def fetch_remote_versions(
    *,
    package_name: str = "canisend",
    timeout_seconds: float = 3.0,
) -> PyPIVersionInfo:
    url = f"https://pypi.org/pypi/{package_name}/json"
    request = Request(url, headers={"User-Agent": "canisend"})
    with urlopen(request, timeout=timeout_seconds) as response:
        payload = json.load(response)

    releases = payload.get("releases", {})
    if not isinstance(releases, dict):
        raise ValueError("PyPI response did not include a releases map")
    return latest_versions_from_releases(releases, source=url)


def format_version_report(
    *,
    local_version: str,
    remote: PyPIVersionInfo | None,
    error: str | None = None,
) -> list[str]:
    lines = [
        "CanISend version",
        "---------------",
        f"Local package      {local_version}",
    ]
    if remote is None:
        lines.extend(
            [
                "Remote stable      unavailable",
                "Remote prerelease  unavailable",
                "",
                "Status",
            ]
        )
        if error:
            lines.append(f"  Remote check failed: {error}")
        else:
            lines.append("  Remote check unavailable")
        return lines

    lines.append(f"Remote stable      {remote.stable or 'unavailable'}")
    lines.append(f"Remote prerelease  {remote.prerelease or 'unavailable'}")

    update_lines = _update_lines(local_version, remote)
    lines.extend(["", "Status"])
    if update_lines:
        lines.extend(f"  {line}" for line in update_lines)
    else:
        lines.append("  CanISend is up to date")

    upgrade_lines = _upgrade_lines(update_lines)
    if upgrade_lines:
        lines.extend(["", "Upgrade"])
        lines.extend(f"  {line}" for line in upgrade_lines)
    return lines


def _update_lines(local_version: str, remote: PyPIVersionInfo) -> list[str]:
    try:
        local = Version(local_version)
    except InvalidVersion:
        return [f"update check skipped: invalid local version {local_version}"]

    lines: list[str] = []
    if remote.stable is not None and Version(remote.stable) > local:
        lines.append(f"Stable update available: {remote.stable}")
    if remote.prerelease is not None and Version(remote.prerelease) > local:
        lines.append(f"Prerelease available: {remote.prerelease}")
    return lines


def _upgrade_lines(update_lines: list[str]) -> list[str]:
    lines: list[str] = []
    if any(line.startswith("Stable update available: ") for line in update_lines):
        lines.append("Stable:     uv tool upgrade canisend")
    if any(line.startswith("Prerelease available: ") for line in update_lines):
        lines.append("Prerelease: uv tool upgrade --prerelease allow canisend")
    return lines
