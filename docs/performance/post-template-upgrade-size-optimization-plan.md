# Post-template-upgrade desktop size optimization plan

**Status:** In progress — supporting M2 of the
[CanISend 1.0 delivery roadmap](../superpowers/plans/2026-07-25-1.0-release-roadmap.md); Phases A
and B are implemented, macOS arm64 is locally qualified, and clean/native target evidence is
pending
**Targets:** macOS arm64/x64, Windows x64, Linux GNU x64; Linux musl remains CLI-only  
**Scope:** Reduce the unified desktop host and standard packages without removing GUI, CLI, MCP,
offline Typst/PDF rendering, intake, integrity, privacy, backup, accessibility, or in-App CLI
installation

**M2 boundary:** Only the completed profile work and clean native-candidate qualification are on
the 1.0 path. Derived-font, TLS-provider, linker/dependency experiments, and public Windows/Linux
GUI publication are P2 or Deferred unless a confirmed release blocker explicitly promotes them.

## 1. Decision and baseline

The latest upstream Typst templates are authoritative. Size work must adapt to those templates; it
must not simplify, pin back, or visually rewrite them to obtain a smaller binary.

The repository currently embeds two template resources:

- `templates/application-document.typ`, which explicitly selects Libertinus Serif and uses regular
  and semibold text;
- `templates/cover-letter.typ`, which inherits the surrounding renderer's font context.

The renderer is pinned to `typst-as-lib 0.16.0`, `typst-assets 0.15.1`, and `typst-pdf 0.15.1` and
uses the complete offline font pack from `typst_assets::fonts()`. The exact template bytes,
dependency lockfile, frontend output, target, toolchain, and signing state must therefore be bound
to every new size record. Measurements taken before the template upgrade are historical context,
not a promotion baseline.

The most relevant existing macOS arm64 measurements are:

| Candidate | Signed host | Logical App payload | Allocated App payload |
| --- | ---: | ---: | ---: |
| Unified default `release` | 60,095,216 B | 60,349,098 B | 60,366,848 B |
| Unified `release`, `opt-level=z` | 53,895,376 B | 54,149,258 B | 54,169,600 B |

These results show that profile optimization is material, but both candidates must be rebuilt from
the upgraded-template commit before a release decision.

### Executed latest-template matrix

The first post-upgrade macOS arm64 matrix was completed on 2026-08-01. Its evidence records a dirty
working tree because it intentionally measures the current uncommitted implementation; it is
nonpublishing engineering evidence and must be repeated from a clean commit on every native owner
before production promotion.

| Candidate | LTO | Unsigned unified host | Saving vs release |
| --- | --- | ---: | ---: |
| `release` | thin | 60,446,288 B | — |
| `size-s-thin` | thin | 59,181,040 B | 1,265,248 B (2.09%) |
| `size-z-thin` | thin | 54,231,248 B | 6,215,040 B (10.28%) |
| `size-z-fat` | fat | 41,422,368 B | 19,023,920 B (31.47%) |

The staged and ad-hoc-signed FatLTO App contains one `41,181,680`-byte host, a `41,435,562`-byte
logical application payload, and the same `939,675`-byte frontend. Five exact-App launches reached
the native window and Svelte main landmark in `1,387.574 ms` median and `1,561.711 ms` maximum.
The FatLTO release CLI performance candidate also passed with an `8 ms` version startup, `9 ms`
capabilities startup, `11 ms` 100-job status, `146 ms` 1 MiB HTML intake, `37 ms` 50-page PDF
intake, and `11 ms` embedded Typst render.

Evidence:

- [profile summary](profile-matrix/aarch64-apple-darwin-latest-template/summary.json)
- [signed App size](profile-matrix/aarch64-apple-darwin-latest-template/macos-app-size-z-fat.json)
- [CLI performance](profile-matrix/aarch64-apple-darwin-latest-template/performance-size-z-fat.json)
- [GUI startup](profile-matrix/aarch64-apple-darwin-latest-template/macos-gui-startup-size-z-fat.json)

`size-z-fat` is now the leading candidate, not the production profile. Windows x64, Linux GNU x64,
macOS x64, clean-source reproduction, installer lifecycle, and the release-owned native matrix are
still required.

## 2. Success criteria

A candidate is promotable only when all of the following are true:

1. It saves at least `max(1 MiB, 2% of the signed native-host baseline)` on the target where it is
   enabled. Smaller changes remain cleanup only and do not justify additional release complexity.
2. The exact upgraded templates pass the template-render contract with no new warnings, missing
   glyphs, page-count changes, clipped content, or unapproved layout changes.
3. GUI, renamed CLI, MCP, in-App CLI lifecycle, offline rendering, URL/PDF intake, update checks,
   and all 37 GUI/CLI parity operations still pass.
4. Existing startup and workload budgets remain satisfied. A target may not trade a smaller package
   for a release-gate regression.
5. The standard package contains exactly one full CanISend host. Windows offline installers and
   Linux AppImage artifacts continue to use separate runtime-inclusive budgets.
6. No renderer, font, schema, application module, or CLI capability becomes a mandatory runtime
   download.
7. Every candidate can be reverted by removing one isolated profile, feature, or asset-selection
   change without reverting the template upgrade.

The first qualified native result on Windows and Linux freezes a target-and-format baseline with
headroom equal to `max(2 MiB, 3% of the measured application payload)`. Subsequent growth beyond
that threshold requires a same-target explanation and explicit release approval.

## 3. Measurement contract

Extend the desktop size evidence before optimizing dependencies. Each record must contain:

| Field group | Required values |
| --- | --- |
| Source identity | source commit, dirty-state flag, `Cargo.lock` SHA-256, pnpm lock SHA-256 |
| Template identity | SHA-256 and resource ID for every embedded `.typ`, template-contract version |
| Toolchain | Rust/Cargo/LLVM, Node/pnpm, Tauri CLI, OS image, linker, signing tool versions |
| Build | target, profile, opt-level, LTO, codegen units, panic strategy, strip state, target directory |
| Dependency candidates | TLS provider, font-pack ID, Typst capability profile, Cargo feature fingerprint |
| Native output | unsigned and signed host bytes, relevant PE/ELF/Mach-O sections, host SHA-256 |
| Product output | frontend bytes, application payload bytes, installed standard bytes |
| Distribution output | format, artifact bytes, package class, platform-runtime bytes, one-host audit |
| Render contract | fixture count, warnings, pages, normalized-text digest, layout result, PDF digest |
| Performance | GUI startup and the existing CLI, intake, workflow, and Typst-render measurements |

The PDF byte digest is evidence, not the sole compatibility assertion. If upstream metadata makes
PDF bytes differ across operating systems, promotion uses the same normalized text, page count,
warning set, visual/layout contract, and per-target reproducibility instead of claiming false
cross-platform byte identity.

Build every comparison from the same source and lockfiles, in separate target directories, with a
single prebuilt frontend. Sign and package only after the unsigned matrix identifies viable
candidates; final decisions use signed/package bytes.

## 4. Phase A — Rebaseline the upgraded templates

**Purpose:** make the latest templates a testable product contract before any font or renderer
change.

**Implementation status:** Complete locally. The contract, embedded-resource audit, all-document-
kind and all-section-kind rendering, warning policy, and native workflow hooks are implemented.
Windows, Linux, and macOS x64 execution remains target-owned evidence.

### Implementation

1. Add `release/typst-template-contract.json` with:
   - contract version and current template resource IDs;
   - exact template SHA-256 values;
   - pinned Typst crate versions;
   - explicitly requested font families, styles, and weights;
   - committed script/symbol coverage;
   - allowed image formats and external-file/package policy;
   - expected fixture pages and warning policy;
   - font licence/notice inventory.
2. Add an `xtask desktop template-audit` command that reads embedded resources rather than a copied
   test directory. It fails when a `.typ` file changes without a contract update.
3. Expand renderer fixtures to cover:
   - application document and cover-letter templates;
   - every `DocumentKind` and every structured section kind;
   - regular and semibold text, long words, URLs, tables/lists, explicit page breaks, empty/maximum
     fields, and maximum supported pages;
   - accented Latin, Greek, Cyrillic, committed mathematical symbols, punctuation, and fallback;
   - any additional script or media capability introduced by the upgraded templates.
4. Record for every fixture: compilation warnings, extracted normalized text, page count, page
   dimensions, PDF size, render duration, and stable visual anchors. Add bounded page-image
   comparisons for the template samples so clipped or displaced content fails even when extracted
   text matches.
5. Run the full-pack renderer on macOS arm64/x64, Windows x64, and Linux GNU x64. Store one baseline
   record per target; do not infer native results from a macOS cross-build.

### Exit gate

- All upgraded-template fixtures render offline with the current full font pack.
- No fixture depends on a system font, filesystem read, Typst package resolution, or network access.
- The template contract, resource hashes, render evidence, and licences are complete.
- Default-release and `opt-level=z` package records have been regenerated from this baseline.

Until this phase passes, the old three-font subset idea is only a hypothesis and must not be
implemented in production.

## 5. Phase B — Select the release optimization profile

**Expected return:** high; the previous macOS arm64 `z` candidate saved about 6.2 MB from the signed
host.

**Implementation status:** Four-candidate tooling and native workflow ownership are complete.
macOS arm64 selected `size-z-fat` as a material nonpublishing candidate with a 31.47% unsigned-host
saving and passing signed-App, CLI/MCP, offline render, performance, and GUI startup evidence.
Production promotion is intentionally pending the clean Windows/Linux/macOS native matrix.

### Candidate matrix

Use Cargo-supported profiles or profile environment overrides and a unique target directory for
each row:

| Candidate | opt-level | LTO | Other release settings |
| --- | --- | --- | --- |
| B0 reference | `3` | `thin` | current `codegen-units=1`, abort, strip symbols |
| B1 size-s | `s` | `thin` | same as B0 |
| B2 size-z | `z` | `thin` | same as B0 |
| B3 winner-fat | winning opt-level | `fat` | same as B0 |

Do not vary codegen units, panic strategy, dependency features, templates, or frontend output in the
same experiment. Cargo documents that `s` and `z` are not guaranteed to be smaller, so all four
rows remain measurements rather than assumptions.

### Verification

For every desktop target:

- build the unified host and record unsigned sections;
- run `version`, `capabilities`, `doctor`, renamed-CLI, MCP stdio, intake, workflow, and embedded
  render contracts;
- package the native standard format and audit exactly one full host;
- run GUI startup and install/upgrade/uninstall on the target OS;
- sign the final candidate and record signed host/application/package bytes;
- run the existing performance contract, including the 100-job status, 1 MiB HTML, 50-page PDF,
  and Typst-render workloads.

### Promotion rule

Choose the smallest candidate that passes every gate on that target. Initially scope the selected
profile to desktop packaging jobs. Keep standalone CLI releases on the canonical release profile
until their five-target matrix independently passes with the candidate. If one desktop target
regresses, retain its current profile rather than weakening the gate for all platforms.

## 6. Phase C — Derive an offline font pack from the latest templates

**Expected return:** potentially high, but unknown until the upgraded-template contract is complete.

This phase does not modify template design. It changes only which licensed font bytes the renderer
embeds.

### Discovery

1. Parse the template contract and statically inventory explicit font family, weight, style, and
   math requests. Treat inherited/default font use as a required dependency, not as “unused”.
2. Instrument a development-only font resolver to record the face chosen for every template
   fixture and Unicode range. Do not ship telemetry or document contents.
3. Produce `full-pack` and `derived-pack-v1` manifests containing file name, face index, family,
   style, weight, supported ranges, raw bytes, SHA-256, licence, and source version.
4. Verify whether the acceptance probe's math and symbol coverage is part of the product contract.
   Preserve it when it is committed; do not add or remove CJK or other script claims merely as part
   of size work.

### Candidate implementation

- Keep `full-pack` as the reference and rollback path.
- Vendor only immutable, licence-compatible font files selected by the manifest, or generate a
  product-owned asset crate whose public function returns the deterministic font byte list.
- Load faces in a fixed order and keep system-font scanning disabled.
- Include font licences and notices in every desktop format.
- Fail closed in tests when a contracted template requests a family/weight/range absent from the
  derived pack.

### Acceptance

- Zero new compile warnings for product templates.
- Exact normalized text and page count against the upgraded-template full-pack baseline.
- No missing glyph boxes, clipped lines, changed page geometry, or failed visual anchors.
- Deterministic results across repeated builds on each target.
- Offline rendering still works with networking and system font discovery unavailable.
- Final signed-host saving meets the materiality threshold on all enabled targets.

If the derived pack cannot meet the visual contract, keep the full upstream font pack. A font
download, system-font fallback, or older template is not an acceptable substitute.

## 7. Phase D — A/B test the Rustls crypto provider

**Current fact:** `reqwest 0.13.4` with the `rustls` feature selects `aws-lc-rs 1.17.3` in the
unified desktop graph.

### Candidate

1. Add an isolated `tls-ring-candidate` feature path using Reqwest's provider-neutral Rustls mode
   and an explicit Rustls `ring` provider.
2. Install the selected provider exactly once before any `reqwest::blocking::Client` is built. Put
   this initialization in the shared I/O boundary so GUI, CLI, and MCP cannot diverge.
3. Keep the current AWS-LC path available as the reference and rollback.
4. Capture provider name/version in the size evidence and diagnostic capabilities output without
   exposing user network data.

### Defensive regression matrix

- TLS 1.2 and TLS 1.3 against bounded owned fixtures;
- platform certificate verification on macOS, Windows, and Linux;
- URL destination policy, redirect limits, request timeouts, response-byte limits, proxy behavior,
  and malformed-response regression tests;
- update-feed, URL intake, and any other HTTPS call sites;
- GUI, CLI, and MCP startup to catch provider-initialization order failures;
- licence and platform build compatibility.

### Decision

Promote `ring` only when it meets the material size threshold in signed PE, ELF, and both Mach-O
targets and has no security/capability regression. Do not select it solely from a macOS result. If
the product later needs an AWS-LC-specific FIPS or post-quantum capability, that requirement takes
priority over size.

## 8. Phase E — Audit Typst image/SVG features

The current structured projection is text-oriented and the restricted world blocks external file
and package access, but the upstream Typst graph still carries GIF, JPEG, PNG, WebP, SVG, `resvg`,
`usvg`, and `fontdb` paths. Their presence in `cargo tree` does not prove their final linked-byte
cost.

### Work

1. Add the promised media capability to the template contract: none, embedded raster formats,
   embedded SVG, or a specific list. The upgraded templates decide this contract.
2. Generate native linker maps for the chosen Phase B profile and attribute actual retained bytes
   to image decoders, SVG rendering, and font database code.
3. Add one bounded offline fixture per promised media format before changing features.
4. Test a feature-reduced upstream candidate only if the public Typst crate boundaries permit it.
5. Consider a maintained patch/fork only when all of these are true:
   - the removable final signed-host cost is at least 3 MiB on every desktop target;
   - the feature contract has fixtures and a clear migration path;
   - upstream updates can be rebased promptly;
   - licences, advisories, and release provenance remain traceable.

If upstream coupling requires a fork for less than that saving, retain the image/SVG stack. The
maintenance and template-upgrade risk is larger than the package benefit.

## 9. Phase F — Linker-led dependency deduplication

Run this only after Phases B–D. `cargo tree --duplicates` includes build scripts and proc macros that
do not necessarily contribute to the final host, so dependency-count reduction is not the goal.

### Work queue

1. Generate `cargo tree -e normal`, feature inversion, duplicate-version, and target-specific
   reports from the locked graph.
2. Generate Mach-O, PE, and ELF linker maps and rank retained runtime symbols/data by crate family.
3. Classify each duplicate as build-only, proc-macro-only, runtime but dead-stripped, or runtime and
   materially retained.
4. Investigate one family per change. Current candidates include `html5ever`, `png`, `schemars`,
   `quick-xml`, `toml`, and `thiserror`; `syn` and other macro-only duplicates are low priority.
5. Prefer normal dependency upgrades/feature alignment. Do not add `[patch]` forks only to make
   `cargo tree` look cleaner.

Accept an individual unification only when it saves at least 256 KiB in the final signed host,
passes the owning crate tests, and does not expand API or format compatibility risk. The overall
release candidate still must meet the 1 MiB/2% materiality gate.

## 10. Phase G — Packaging and regression budgets

Packaging policy remains independent of native-code optimization:

- macOS DMG/ZIP contains one signed App host;
- Windows standard NSIS/MSI uses WebView2 `downloadBootstrapper`; the offline WebView2 installer is
  a separate class and size record;
- Linux DEB/RPM is the standard class with declared WebKitGTK/GTK dependencies; AppImage remains a
  separately budgeted portable container;
- the in-App CLI action copies the exact verified unified host and remains reversible;
- frontend output remains below 1.5 MiB and excludes source maps, `node_modules`, tests, and
  duplicate assets.

Report, but do not combine, native host bytes, application payload, standard installed bytes,
download artifact bytes, platform runtime bytes, portable container bytes, and the optional
user-created CLI copy.

## 11. Delivery sequence

Implement as small, independently reversible changes:

| Change | Deliverable | May change production output? |
| --- | --- | --- |
| 1 | Template contract, hashes, fixtures, audit command, fresh full-pack baselines | No |
| 2 | Four-row native profile matrix and evidence schema | No |
| 3 | Promote per-target desktop profile after native qualification | Yes |
| 4 | Font inventory and derived-pack candidate behind an explicit feature | No |
| 5 | Promote derived font pack after cross-platform visual/render qualification | Yes |
| 6 | TLS provider A/B candidate and defensive regression fixtures | No |
| 7 | Promote provider only after all-target evidence | Yes |
| 8 | Typst media/link-map audit; optional feature experiment | No by default |
| 9 | One measured runtime dependency cleanup at a time | Possibly |
| 10 | Freeze signed per-target/per-format budgets in release contracts | Release policy |

Do not combine profile, font, TLS, and dependency changes in one measurement or promotion change.
After individual qualification, build a final cumulative candidate and rerun the complete native
package matrix because byte savings and performance are not necessarily additive.

## 12. Required native matrix

| Gate | macOS arm64 | macOS x64 | Windows x64 | Linux GNU x64 |
| --- | :---: | :---: | :---: | :---: |
| Template contract/full-pack baseline | required | required | required | required |
| Profile B0–B3 matrix | required | required | required | required |
| GUI/no-argument launch | required | required | required | required |
| Renamed CLI and MCP without WebView | required | required | required | required |
| In-App CLI install/update/uninstall | required | required | required | required |
| Offline render fixtures | required | required | required | required |
| URL/TLS defensive regression | required | required | required | required |
| Standard package one-host extraction | required | required | required | required |
| Signature/package integrity | required | required | required | required |
| Install/upgrade/uninstall | required | required | required | required |
| Accessibility and startup | required | required | required | required |

Linux musl continues through the standalone CLI matrix. It is not allowed to block or imply Linux
GUI support until its window/runtime boundary is separately defined.

## 13. Stop conditions and rejected shortcuts

Stop an experiment and retain the reference when it:

- changes the upgraded templates, their appearance, or their supported content merely to reduce
  size;
- introduces warnings, missing glyphs, runtime downloads, system-font dependence, or nondeterminism;
- weakens TLS verification, URL destination policy, parser bounds, signing, provenance, privacy,
  accessibility, backup, or rollback;
- requires a long-lived upstream fork below its explicit return threshold;
- saves only compressed package bytes while installed application payload does not improve;
- makes the CLI dependent on the App remaining at its original path.

Do not use UPX or a similar executable packer. Do not compare standard installers with offline
WebView2 or AppImage totals. Do not remove user-visible operations to pass a size budget.

## 14. Completion definition

This plan is complete when:

1. upgraded-template baselines and hashes are checked in for every desktop target;
2. one profile is qualified per target and production desktop builds use it explicitly;
3. the font-pack decision is evidence-backed, including a documented decision to retain the full
   pack if the visual contract prevents safe reduction;
4. the TLS provider decision is recorded from native signed artifacts;
5. Typst media and dependency duplicates have linker-based keep/remove decisions;
6. standard and runtime-inclusive packages have separate frozen thresholds;
7. the cumulative candidate passes exact package lifecycle, parity, performance, accessibility,
   rendering, integrity, and rollback gates on all claimed desktop targets.

## References

- Cargo profiles and custom profiles: <https://doc.rust-lang.org/cargo/reference/profiles.html>
- Cargo profile environment overrides: <https://doc.rust-lang.org/cargo/reference/environment-variables.html>
- Reqwest TLS provider selection: <https://docs.rs/reqwest/latest/reqwest/tls/>
- Rustls crypto providers: <https://docs.rs/rustls/latest/rustls/crypto/index.html>
- Typst embedded assets source: <https://docs.rs/typst-assets/latest/src/typst_assets/lib.rs.html>
- Tauri Windows installer/runtime modes: <https://v2.tauri.app/distribute/windows-installer/>
- Tauri AppImage packaging: <https://v2.tauri.app/distribute/appimage/>
