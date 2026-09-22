# ADR-RN-0024: Add Linux GNU registry-testing builds

**Status:** Accepted

**Date:** 2026-09-22

**Decision owner:** CanISend maintainer

## Context

The registry-testing Beta was installable only on Apple Silicon macOS through npm and PyPI.
GitHub now provides native hosted runners for Linux x86_64 and arm64, so both architectures can
build and execute their own candidate instead of relying on an unverified cross-compile.

## Decision

- Build `x86_64-unknown-linux-gnu` on `ubuntu-24.04` and
  `aarch64-unknown-linux-gnu` on `ubuntu-24.04-arm` for registry-testing Betas.
- Publish one npm package containing the macOS arm64 and both Linux GNU executables, and publish
  one native PyPI wheel per platform for the same version.
- Run archive or wheel installation smoke on each native builder, then install and verify the
  exact public registry package again on each Linux architecture.
- Keep publication in the existing `release.yml` Trusted Publisher workflow. These jobs are
  registry testing evidence, not full native-release or real-Host qualification.
- Keep `release/targets.json` and the five-target GitHub Release contract unchanged. Linux arm64
  GitHub archives, musl arm64, desktop packages, signing, and Stable support remain separate work.

## Consequences

- npm and PyPI testing releases can be installed on glibc-based Linux x86_64 and arm64 systems.
- The npm package remains self-contained and uses the existing runtime platform selector; no
  install script, binary download, platform package, or new dependency is introduced.
- Linux wheel compatibility is bounded by the wheel tag produced and verified by the native
  Ubuntu 24.04 build. Wider manylinux compatibility requires a separate audited build change.

## Rejected alternatives

- Cross-compile Linux arm64 on x86_64: rejected because native execution evidence is available.
- Add Linux arm64 to the full GitHub Release matrix now: rejected because this request concerns
  the registry Beta and must not silently broaden the qualified 1.0 archive contract.
