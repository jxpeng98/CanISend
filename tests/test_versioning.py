from canisend.versioning import (
    PyPIVersionInfo,
    format_version_report,
    latest_versions_from_releases,
)


def test_latest_versions_from_releases_splits_stable_and_prerelease():
    info = latest_versions_from_releases(
        {
            "0.1.0": [],
            "0.2.0b7": [],
            "0.2.0": [],
            "0.3.0rc1": [],
            "not-a-version": [],
        }
    )

    assert info.stable == "0.2.0"
    assert info.prerelease == "0.3.0rc1"


def test_format_version_report_recommends_newer_remote_versions():
    lines = format_version_report(
        local_version="0.2.0b7",
        remote=PyPIVersionInfo(stable="0.2.0", prerelease="0.3.0rc1"),
    )

    assert lines == [
        "CanISend version",
        "---------------",
        "Local package      0.2.0b7",
        "Remote stable      0.2.0",
        "Remote prerelease  0.3.0rc1",
        "",
        "Status",
        "  Stable update available: 0.2.0",
        "  Prerelease available: 0.3.0rc1",
        "",
        "Upgrade",
        "  Stable:     uv tool upgrade canisend",
        "  Prerelease: uv tool upgrade --prerelease allow canisend",
    ]


def test_format_version_report_handles_remote_check_failures():
    lines = format_version_report(local_version="0.2.0", remote=None, error="timed out")

    assert lines == [
        "CanISend version",
        "---------------",
        "Local package      0.2.0",
        "Remote stable      unavailable",
        "Remote prerelease  unavailable",
        "",
        "Status",
        "  Remote check failed: timed out",
    ]
