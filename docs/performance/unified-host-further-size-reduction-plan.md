# Unified host further size reduction plan

**Status:** Latest-template profile matrix implemented and locally qualified on macOS arm64;
`opt-level=z` plus FatLTO leads, native clean Windows/Linux/macOS qualification pending

**Decision boundary:** keep every GUI, CLI, MCP, offline-rendering, intake, integrity, privacy,
backup, and release function. A smaller artifact is not accepted when it depends on a mandatory
runtime or font download.

The upgraded upstream Typst templates are now the authoritative baseline. The detailed execution,
measurement, rollback, and cross-platform promotion sequence is defined in
[Post-template-upgrade desktop size optimization plan](post-template-upgrade-size-optimization-plan.md).
Measurements below predate the required post-upgrade rebaseline and remain comparison evidence,
not a direct production-promotion record.

## Measured result

All rows below use the same Svelte output (`939,675` bytes) and one GUI/CLI/MCP host. Signed rows
were staged with `scripts/stage_macos_gui_app.sh` and passed bundle signature, final-byte, version,
one-host, and size verification.

| Apple Silicon candidate | Signed host | Logical App payload | Allocated App payload |
| --- | ---: | ---: | ---: |
| Historical Alpha.5, two hosts | — | — | 113,577,984 B |
| Unified `release-alpha` | 63,391,984 B | 63,645,908 B | 63,664,128 B |
| Unified `release`, profile default | 60,095,216 B | 60,349,098 B | 60,366,848 B |
| Unified `release`, `opt-level=z` | 53,895,376 B | 54,149,258 B | 54,169,600 B |
| Latest templates, `opt-level=z` plus FatLTO | 41,181,680 B | 41,435,562 B | 41,455,616 B |

The size candidate saves `6,199,840` bytes (10.32%) against the signed default `release` host and
`9,496,608` bytes (14.98%) against the current signed `release-alpha` host. Its complete allocated
App is 52.31% below the historical two-host Alpha.5 App.

The subsequent latest-template matrix found a stronger nonpublishing candidate: `opt-level=z` plus
FatLTO reduces the unsigned host from `60,446,288` to `41,422,368` bytes (31.47%). After ad-hoc
signing, its one-host App is `41,455,616` allocated bytes. Exact evidence is in the
[post-template-upgrade plan](post-template-upgrade-size-optimization-plan.md). This does not change
the production profile until clean native Windows, Linux, and macOS qualification passes.

The unsigned Mach-O comparison explains the reduction:

| Section | Default `release` | `opt-level=z` | Change |
| --- | ---: | ---: | ---: |
| `__text` | 31,300,368 B | 16,473,392 B | -14,826,976 B |
| `__const` | 24,472,536 B | 24,304,600 B | -167,936 B |
| `__LINKEDIT` | 1,425,408 B | 9,830,400 B | +8,404,992 B |
| Whole unsigned host | 60,446,320 B | 54,210,400 B | -6,235,920 B |

The larger link-edit segment means future work must continue to compare final files, not only the
text section.

## Capability and performance evidence

The `opt-level=z` candidate passed the full release CLI performance contract on macOS arm64:

| Gate | Candidate | Limit |
| --- | ---: | ---: |
| Version startup median | 8 ms | 100 ms |
| Capabilities startup median | 8 ms | 150 ms |
| Status over 100 jobs | 11 ms | 500 ms |
| 1 MiB HTML intake | 145 ms | 2,000 ms |
| 50-page PDF intake | 47 ms | 5,000 ms |
| Embedded Typst render | 11 ms | 1,000 ms |
| Standalone CLI binary | 44,800,800 B | 67,108,864 B |

The staged size candidate also passed `version`, `doctor`, embedded-font verification, the
cross-platform Unicode/math/two-page PDF probe, final-byte verification, and ad-hoc bundle signing.
Native-window startup and exact installer lifecycle still belong to the native platform jobs.

## Implemented candidate changes

1. The scheduled Windows and Linux desktop qualification jobs build release candidates with
   `CARGO_PROFILE_RELEASE_OPT_LEVEL=z` and record that optimization in the size evidence.
2. The renderer loads the same Typst embedded fonts directly and no longer enables the unused
   `typst-kit/scan-fonts` feature. This removed `typst-kit` from the dependency graph while keeping
   system-font scanning disabled. It saved only 208 bytes in the final macOS host because the SVG
   renderer still needs `fontdb`; treat it as dependency cleanup, not the main size result.
3. Package evidence distinguishes standard, offline, and portable artifacts and records whether a
   platform runtime is included. Windows offline NSIS and Linux AppImage never share a threshold
   with the standard installers.

The production release profile is not changed yet. Promotion requires a successful native
Windows/Linux qualification run plus the exact macOS GUI startup gate, so the current standalone
CLI matrix is not silently moved to a different optimization policy.

## Next experiments, in order

### 1. Promote the winning `opt-level=z` LTO candidate after native qualification

The latest Apple Silicon matrix selects FatLTO and saves about 19.0 MB from the unsigned default
release host; Windows and Linux must publish their own evidence. Accept only if GUI creation,
renamed CLI, MCP, intake, rendering,
package extraction, install/upgrade/uninstall, signing, and performance checks pass. If one target
regresses, keep per-target evidence and retain the default release optimization on that target.

### 2. Replace the general Typst font pack with a product-owned deterministic subset

The previous three-face suggestion is no longer a valid starting assumption after the template
upgrade. First bind the exact latest template hashes, font/style requests, inherited defaults,
script/symbol coverage, render warnings, normalized text, page geometry, and visual samples in the
template contract. Then derive and measure a candidate font manifest from actual template and
fixture resolution while keeping the full pack as the reference and rollback path.

This remains potentially the largest data reduction, but it is not a blind deletion. Promotion
requires every document kind and committed script/media capability to pass offline on macOS,
Windows, and Linux with no new warning, missing glyph, clipped content, page-count change, or
unapproved visual difference. Copy font licences and attribution with any vendored subset. If the
latest templates need the full upstream pack, retain it.

### 3. Measure the TLS crypto provider

`reqwest` currently selects Rustls with `aws-lc-rs`. Build an isolated `ring` provider candidate
and compare final Mach-O, PE, and ELF sizes. Keep it only if HTTPS destination policy, redirects,
platform certificate verification, TLS 1.2/1.3 fixtures, malformed-response limits, and native
builds all pass. This is an experiment, not an assumption that a provider is universally smaller.

### 4. Audit Typst image and SVG capabilities against the product contract

The Typst PDF stack currently retains GIF, JPEG, PNG, WebP, SVG, system-font, and memory-mapped font
features through upstream defaults. Do not remove them merely because current templates are
text-heavy. First state which embedded-image formats application documents promise, add one bounded
fixture per promised format, then test a dependency feature patch in isolation. Upstream feature
coupling may make this lower return than the font subset.

### 5. Keep frontend and packaging hygiene as regression work

The frontend is below 1 MiB, so further Svelte tree-shaking cannot materially replace native work.
Continue excluding source maps, `node_modules`, test fixtures, and duplicate icons. Keep Windows
`downloadBootstrapper` as the standard small installer and measure the WebView2 offline installer
separately. Keep DEB/RPM as the Linux standard class and AppImage as a separately budgeted portable
class.

## Rejected shortcuts

- Do not use UPX or another executable packer; it complicates signing, reputation, diagnostics, and
  startup.
- Do not remove the in-App CLI installation or replace the durable copied CLI with an app-relative
  symlink.
- Do not download the renderer, fonts, schemas, or application modules after installation to make
  the package look smaller.
- Do not remove TLS, signature, provenance, URL/PDF bounds, privacy, backup, or accessibility gates.
- Do not compare MSI/DEB installed payloads with offline WebView2 or AppImage container sizes.

## Promotion gate

Promote a size candidate only when its exact bytes have target-owned evidence for one-host package
contents, GUI/CLI/MCP routing, full operation parity, CLI lifecycle, offline rendering, intake,
performance, accessibility, install/upgrade/uninstall, artifact verification, and rollback. Freeze
new per-target and per-format thresholds only after the first successful Windows and Linux runs.
