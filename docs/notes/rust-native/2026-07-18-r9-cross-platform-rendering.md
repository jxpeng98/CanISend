# R9.4 Cross-Platform Rendering Checkpoint

**Date:** 2026-07-18

**Branch:** `rewrite/rust-native`

**Implementation commit:** `c8d562b`

**CI evidence:** GitHub Actions run `29628602007`

## Outcome

R9 is complete. The production release binary renders the same complex fixture and full revision-bound application
package on the three initial native target families without Python, Node, Java, a Typst executable, runtime package
downloads, network access, system-font scanning, or user-installed fonts.

The matrix exercises embedded fonts, Unicode from multiple scripts, mathematical notation, URLs, nested lists,
explicit page breaks, and a missing user-font environment. It also builds an optimized probe, enforces the 64 MiB
binary budget, stages the binary with notices, and uploads the resulting native bundle as CI evidence.

## Measurements

| Target | Release binary | Render probe | PDF | Warnings |
| --- | ---: | ---: | ---: | ---: |
| macOS arm64 (`aarch64-apple-darwin`) | 48,792,144 bytes | 13 ms | 2 pages / 26,678 bytes | 0 |
| Windows x86_64 (`x86_64-pc-windows-msvc`) | 51,936,256 bytes | 11 ms | 2 pages / 26,678 bytes | 0 |
| Linux x86_64 (`x86_64-unknown-linux-gnu`) | 58,421,672 bytes | 5 ms | 2 pages / 26,678 bytes | 0 |

Every target remained below the 67,108,864-byte release threshold. The identical PDF size is supporting evidence
for the deterministic embedded asset path; semantic and structural PDF tests remain the authority rather than byte
identity across toolchain versions.

## Distribution obligations

`scripts/stage_native_bundle.sh` stages the executable together with:

- the CanISend product license;
- `THIRD_PARTY_NOTICES.md`;
- embedded font license texts;
- the embedded Typst asset notice.

CI refuses to upload the native render evidence if staging or any earlier acceptance step fails. R11 release archives
must use the same staging boundary so the notices cannot be accidentally omitted from published binaries.

## Gate result

GitHub Actions run `29628602007` completed successfully:

- `rust-quality`: success;
- `render-macos-arm64`: success;
- `render-windows-x86_64`: success;
- `render-linux-x86_64`: success.

This closes R9.4 and the complete offline rendering phase. The next roadmap authority is R10 hardening and
performance, beginning with the security review.

