# R10.3 Performance Checkpoint

**Date:** 2026-07-18

**Branch:** `rewrite/rust-native`

**Implementation commit:** `b9d840a`

**CI evidence:** GitHub Actions run `29630280560`

## Outcome

R10.3 converts the roadmap performance targets into release-profile regression gates. The normal workspace suite
still skips the benchmark contract, so developer feedback remains fast; main and tag-release workflows explicitly
activate it after a cached optimized build.

The contract measures real native process launches and storage paths. HTML excludes DNS/network but includes the
same bounded normalization used after safe transport plus an authoritative source commit. PDF includes a generated
50-page text document, local-file validation, parsing/extraction, blob publication, and job revision mutation. The
full synthetic test spans intake, criteria, evidence, matching, decision, four structured documents, review, package,
Typst/PDF render, export, and upstream invalidation.

## Baselines

| Metric | macOS arm64 local | Linux x86_64 CI | Threshold |
| --- | ---: | ---: | ---: |
| `version` startup median | 7 ms | 3 ms | 100 ms |
| capabilities startup median | 7 ms | 3 ms | 150 ms |
| status over 104 jobs | 8 ms | 4 ms | 500 ms |
| 1 MiB HTML intake | 73 ms | 63 ms | 2,000 ms |
| 50-page PDF intake | 29 ms | 15 ms | 5,000 ms |
| embedded Typst render | 5 ms | 6 ms | 1,000 ms |
| full synthetic workflow | 831 ms | 263 ms | 15,000 ms |
| release binary | 48,874,800 bytes | 58,431,816 bytes | 67,108,864 bytes |

CI's benchmark functions themselves completed in 0.45 seconds and 0.27 seconds. Cold compilation and linking are
reported by CI but not counted as product execution. The run's quality job completed in 3 minutes 50 seconds, below
the five-minute normal gate budget; the full cross-platform run completed in 9 minutes 11 seconds, below the
30-minute release-matrix budget.

## Gate behavior

The Linux native bundle now carries `PERFORMANCE.json` beside licenses and notices. Threshold increases require a
same-target before/after baseline, cause analysis, confirmation that security/integrity controls remain enabled, and
an updated roadmap/note. The complete policy is in [benchmark-policy.md](../../performance/benchmark-policy.md).

This closes R10.3. R10.4 continues with human output, installation, privacy, agent, recovery, and troubleshooting
documentation plus staged-binary clean-install smoke.
