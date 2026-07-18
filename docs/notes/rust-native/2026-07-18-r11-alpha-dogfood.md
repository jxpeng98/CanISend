# R11.1 native Alpha qualification and public-job dogfood

**Date:** 2026-07-18

**Roadmap item:** R11.1

## Candidate identity

The qualified `v0.7.0-alpha.1` candidate is source commit
`4cec4ec48cc2e96f3798dde0b438d3aaa617a2f8`. The tag must continue to resolve to that commit even if later
documentation commits advance `rewrite/rust-native`.

Ordinary CI run `29632388521` passed dependency policy, Rust quality, recovery, rendering, performance, and staged
documentation jobs. Native release dry-run `29632388573` then passed the release identity and complete source gates,
target tests, release builds, archive creation, exact extracted-archive smokes, GNU performance and full-workflow
budgets, final assembly, and release-evidence upload for:

- `aarch64-apple-darwin`;
- `x86_64-apple-darwin`;
- `x86_64-unknown-linux-gnu`;
- `x86_64-unknown-linux-musl`; and
- `x86_64-pc-windows-msvc`.

The complete evidence artifact was downloaded outside the producing jobs. `xtask release verify` accepted the tag
and all 11 checksum-listed files, and `shasum -a 256 -c SHA256SUMS` matched every archive, SBOM, manifest, notice,
release note, limitation, and issue-intake file. GitHub attestation verification accepted all 12 files, including
`SHA256SUMS`, while enforcing repository `jxpeng98/CanISend`, signer workflow `.github/workflows/release.yml`, and the
exact candidate source digest.

## Failures retained as release evidence

The successful dry-run followed three useful fail-closed corrections rather than bypasses:

1. Run `29631443437` rejected internal path dependencies without exact release versions. All 15 internal dependency
   declarations now pin `=0.7.0-alpha.1`, and `xtask release check` validates normal and target-specific dependency
   tables.
2. Run `29631782291` passed four target matrices but exposed Windows sharing violations when two SQLite-backed unit
   tests removed temporary workspaces before releasing the owning `Workspace`. The tests now release the database
   owner before cleanup.
3. Run `29632221770` correctly rejected an overbroad cleanup attempt through Clippy's `drop_non_drop`. Only the
   resource-owning `Workspace` is explicitly released; non-owning service wrappers are not artificially dropped.

The final runs therefore preserve strict dependency, Windows cleanup, and warnings-as-errors policies.

## Public source intake

The dogfood workspace was temporary and is not committed. It contains no real applicant profile. Both sources were
public University of Cambridge job resources fetched through the same bounded URL intake used by end users:

| Source | Media type | Original SHA-256 | Normalized-text SHA-256 |
|---|---|---|---|
| `https://www.cam.ac.uk/jobs/university-assistant-professor-in-polar-studies-lc49794` | `text/html; charset=utf-8` | `c7a1465baa1cc460f53be07a62bfaa765bd914675017decc5d25e6fe8db52eda` | `6464277ad625dbb79797abc17954b9cdf03544b38ce8869e7df1bacb7fe776a2` |
| `https://www.jobs.cam.ac.uk/job/55635/file/LC49794+-+Further_information_hr7_FINAL+-+v2.pdf` | `application/pdf` | `06a9bec05b4c25ad314ada3560a9bf38660c50084df2341d9de6cbb26450d9cb` | `73498c6218e06f6b837e27f94f1936f57639b9ca369689e31d6aafa2aa9555ef` |

Both imports preserved original and normalized immutable artifacts, recorded no redirects, and remained body-free in
routine `job show` output. The final `workspace check` reported SQLite integrity `ok`, 11 referenced blobs checked,
and no issues, stale artifacts, unreferenced blobs, or required projection repairs.

## Application workflow outcome

The HTML job continued through Intake, Parse, Criteria, Evidence, Match, and Plan using a clearly labelled synthetic
Economics profile. It was deliberately not modified to resemble a qualified Polar Studies applicant. Match produced
three partial matches and three gaps, covering discipline relevance, publication relevance, teaching breadth,
research leadership, grant funding, and doctoral/postdoctoral supervision. It also produced explicit prohibited
claims for every unsupported area.

The user-decision stage recorded `hold`, four planned application documents, and six blocking essential criteria.
Draft then reported `workflow.plan_blocked`; Review, Package, and Render remained transitively blocked. This is the
expected successful dogfood result: CanISend retained provenance and completed useful analysis while refusing to
fabricate evidence or generate misleading application material.

## Transition

The five target builds, exact packaged-binary smokes, independently verified release evidence, and real-job dogfood
satisfied the pre-publication R11.1 qualification gates. Publication and public-asset verification are recorded in
the [Alpha release closeout note](2026-07-18-r11-alpha-release.md). R11.1 is complete and R11.2 Beta hardening is
active.
