# Cross-platform desktop size optimization plan

**Status:** In progress — Phases 0–3 locally qualified on macOS arm64; Phase 4 implementation landed
and awaits native qualification; Phase 5 latest-template `z` plus FatLTO candidate is locally
qualified on macOS arm64  
**Scope:** macOS desktop cut over; Windows x64 and Linux x64 desktop candidates configured  
**Primary constraint:** reduce package size without removing GUI, CLI, MCP, offline rendering, or
release-integrity capabilities

## 1. Outcome

CanISend desktop packages should contain one full native application executable, not one GUI
executable plus a second CLI executable containing the same Rust, SQLite, intake, TLS, Typst, PDF,
schema, and resource stack.

That single executable is a multi-call host with three entry modes:

| Entry | Selection rule | Behaviour |
| --- | --- | --- |
| Desktop GUI | bundle launch, desktop shortcut, or explicit `--gui` | starts Tauri/Svelte |
| CLI | installed name `canisend` / `canisend.exe`, or explicit CLI arguments | runs the shared Clap dispatcher without starting a WebView |
| MCP | explicit `mcp serve` arguments | runs the version-matched stdio MCP server without starting a WebView |

The Settings page continues to offer CLI status, install, update, PATH configuration, and
uninstall. Installation copies the signed and verified multi-call host to a user-managed CLI path.
The copy is created only after explicit user action and is not part of the desktop package payload.
It remains usable if the desktop app is moved or uninstalled.

## 2. Non-negotiable capability boundary

Size work is accepted only when all of these invariants remain true:

1. All 37 operations in `docs/contracts/cli-gui-parity-v1.json` and all 37 implemented entries in
   `docs/contracts/svelte-parity-v1.json` remain implemented.
2. The GUI can perform every committed GUI operation without a terminal CLI already installed.
3. The app can inspect, install, update, configure, and uninstall its managed CLI.
4. The installed CLI can perform ordinary commands, offline Typst/PDF rendering, and `mcp serve`
   without opening a WebView.
5. MCP configuration uses the exact version-matched host selected by CanISend; it does not search
   for an arbitrary executable on `PATH`.
6. SQLite, URL/PDF intake limits, TLS, schemas, embedded resources, deterministic offline fonts,
   backup/recovery, privacy/consent, signature, digest, and provenance controls stay enabled.
7. No runtime, font pack, renderer, or application module becomes a mandatory post-install
   download merely to make the headline package smaller.
8. The standalone CLI release matrix remains supported even while desktop packaging changes.

This explicitly rules out reducing size by deleting operations, replacing the installed CLI with a
fragile app-relative symlink, removing offline rendering, disabling artifact verification, or
shipping a web-only shell.

## 3. Measured starting point

The checked-in Apple Silicon Alpha.5 App baseline is `113,577,984` apparent bytes:

| Component | Bytes | Share |
| --- | ---: | ---: |
| Tauri GUI executable | 60,470,016 | 53.24% |
| Bundled CLI executable | 52,836,304 | 46.52% |
| All other App files | 271,664 | 0.24% |

The two native files are 99.76% of the installed App. The frontend is therefore not the primary
size problem. The exact locally staged and ad-hoc-signed Apple Silicon `release-alpha` App now has
one `63,391,984`-byte host, a `63,645,908`-byte logical payload, a `63,664,128`-byte allocated
payload, and a `26,692,774`-byte ZIP. The allocated App is 43.95% below the Alpha.5 baseline.

Release builds already use one codegen unit, ThinLTO, abort-on-panic, and symbol stripping. The
`release-alpha` preview profile intentionally uses 16 codegen units without LTO and must not be
compared directly with the canonical `release` profile.

The current five-target native release matrix is:

- `aarch64-apple-darwin`;
- `x86_64-apple-darwin`;
- `x86_64-unknown-linux-gnu`;
- `x86_64-unknown-linux-musl`; and
- `x86_64-pc-windows-msvc`.

Windows x64 and Linux GNU x64 are the future desktop targets. Linux musl remains CLI-only until its
window-system boundary is explicitly qualified.

## 4. Size accounting model

Every release record must report these values separately. Combining them into one number hides the
real source of growth.

| Metric | Meaning |
| --- | --- |
| `native_host_bytes` | exact signed/stripped multi-call executable |
| `application_payload_bytes` | host plus CanISend-owned frontend, icons, metadata, licences, and resources |
| `installed_standard_bytes` | installed payload of the normal platform package |
| `download_artifact_bytes` | compressed DMG/ZIP/MSI/NSIS/DEB/RPM artifact |
| `platform_runtime_bytes` | bundled WebView or system-dependency payload, if present |
| `portable_container_bytes` | complete AppImage or other self-contained artifact |
| `optional_cli_copy_bytes` | user-created CLI copy after an explicit in-app install |

The primary product metric is `application_payload_bytes`. Download compression is useful but is
not evidence of lower installed size. The optional CLI copy is reported for transparency but does
not count as duplicated package payload.

There are two distribution classes:

- **Standard small package:** uses an operating-system WebView/runtime or declared system package
  dependency and contains one CanISend native host.
- **Offline/portable package:** includes platform runtime dependencies for machines without them.
  It contains the same one CanISend host but has a separate, explicitly higher budget.

Standard and offline/portable artifacts must never share a size threshold.

## 5. Target architecture

### 5.1 One multi-call host

Move launch selection out of its current macOS-only boundary and make it deterministic on macOS,
Windows, and Linux:

1. A desktop bundle or shortcut launch with the desktop executable name and no arguments starts
   the GUI.
2. `--gui` starts the GUI only when it is the sole explicit mode argument.
3. A copy named `canisend` or `canisend.exe` selects CLI mode even with no arguments, so a terminal
   invocation prints CLI help instead of opening a window.
4. Any explicit CLI or MCP command selects the shared CLI dispatcher.
5. macOS Finder `-psn_...` arguments remain GUI launches.
6. Ambiguous combinations fail through the argument parser rather than silently selecting a mode.

Selection must use a tested combination of executable basename and arguments. It must not depend on
the current working directory, mutable environment variables, or an executable discovered on
`PATH`.

### 5.2 GUI calls remain in-process

Tauri commands continue to call the typed `canisend-app` facade. They do not shell out to the CLI.
The shared executable is an entrypoint and packaging optimization, not a replacement of the GUI's
typed application boundary.

### 5.3 Durable in-app CLI installation

The desktop CLI lifecycle uses the current verified multi-call host as its source:

1. Resolve only the current packaged host recorded by bundle metadata.
2. Reject symlinks and non-regular files at the source and destination boundaries.
3. Verify the packaged host signature where the platform provides one and bind the final source
   digest/version before installation.
4. Copy to a temporary file in the destination directory.
5. Verify the copied digest and `version --json` result.
6. Atomically replace the managed destination and write the install manifest.
7. Preserve unmanaged installations, reject silent downgrade, and retain rollback behaviour.
8. Verify the installed digest again after installation. On macOS, the copied main executable keeps
   the exact signed bytes, but strict standalone `codesign` verification is not a valid assertion
   because the main-executable signature is bound to the App bundle's `Info.plist`.

A full copy is the default because it survives app relocation and uninstall. A tiny shim, symlink,
or app-relative launcher may save optional user disk usage but creates a weaker lifecycle and is not
the release default.

### 5.4 MCP uses the same host

Agent configuration records the verified packaged or managed executable with explicit
`--workspace ... mcp serve` arguments. No second MCP binary or helper runtime is added. MCP launch
tests must demonstrate that no WebView is created.

## 6. Platform distribution policy

| Platform | Standard small package | Offline/portable variant | In-app CLI destination | Policy |
| --- | --- | --- | --- | --- |
| macOS arm64/x64 | signed `.app` in DMG/ZIP, one `Contents/MacOS/canisend-gui` | same app payload; archive format does not add a second host | `~/.local/bin/canisend` | verify the signed App host, then copy its exact bytes after consent and preserve the digest |
| Windows x64 | NSIS/MSI using WebView2 `downloadBootstrapper`, one `canisend-gui.exe` | separate installer with offline WebView2 runtime and separate budget | `%LOCALAPPDATA%\CanISend\bin\canisend.exe` | copy the Authenticode-signed host; configure per-user PATH explicitly and reversibly |
| Linux GNU x64 | `.deb`/`.rpm` declaring WebKitGTK/GTK system dependencies, one host | separate AppImage with its own runtime/container budget | `~/.local/bin/canisend` | copy the verified inner host after consent; do not count AppImage dependencies as app code |
| Linux musl x64 | standalone CLI archive | no GUI claim initially | package-manager/user selected | keep current CLI-only support until a desktop runtime is qualified |

For Windows, the public small installer should use `downloadBootstrapper`: it uses an existing
WebView2 runtime when present and requires a network connection to install one when absent. An
offline installer is published and measured separately for managed or disconnected environments;
the Microsoft Store variant also uses a separate offline configuration. Tauri's Windows installer
documentation reports that the download bootstrapper adds no embedded runtime, the embedded
bootstrapper is about 1.8 MB, and the offline/fixed runtime choices add roughly 127–180 MB. Those
runtime variants therefore cannot use the small-installer budget.

For Linux, `.deb`/`.rpm` is the primary small-package claim because WebKitGTK/GTK are declared system
dependencies. AppImage intentionally bundles dependencies for portability; Tauri documents that
this can turn a small application into a 70+ MB artifact. AppImage remains useful, but is a separate
size class rather than a failure of the CanISend application-payload gate.

References:

- <https://v2.tauri.app/distribute/windows-installer/>
- <https://v2.tauri.app/distribute/debian/>
- <https://v2.tauri.app/distribute/appimage/>
- <https://v2.tauri.app/distribute/microsoft-store/>

## 7. Budgets and release gates

The following are provisional engineering targets until exact signed candidates exist for every
target and package format:

| Gate | Initial target |
| --- | ---: |
| Full unified native host | at most 67,108,864 bytes (existing 64 MiB gate) |
| Standard CanISend application payload | at most 75,497,472 bytes (72 MiB) |
| Number of full CanISend native hosts in a desktop package | exactly 1 |
| Raw built frontend | at most 1,572,864 bytes (1.5 MiB) |
| CLI/GUI parity entries implemented | 37 of 37 |
| Svelte parity entries implemented | 37 of 37 |

Additional rules:

1. Record a clean, signed baseline for every `target + profile + package format`; never compare a
   `.deb` with an AppImage or a standard Windows installer with an offline installer.
2. Freeze exact download and installed-size thresholds from the first qualified Windows and Linux
   candidates. Subsequent releases may not raise them without same-target before/after evidence and
   a documented reason.
3. A standard package fails if it contains a second large PE, ELF, or Mach-O application host even
   when the compressed artifact remains below its byte threshold.
4. Offline Windows and AppImage records expose both total artifact bytes and CanISend application
   payload bytes. Runtime growth cannot hide product growth, and product growth cannot be blamed on
   the runtime.
5. Size gates never authorize removing integrity, safety, privacy, accessibility, or render tests.

The macOS package contract now enforces the measured single-host boundary: at most 64 MiB for the
host and 72 MiB for the App payload. Windows and Linux thresholds remain provisional until their
first native signed/verified candidates are recorded on the target operating systems.

## 8. Delivery phases

### Phase 0 — Freeze contracts and establish comparable baselines

**Implementation status:** Complete for macOS arm64. The versioned recorder and checked-in exact
App/ZIP baseline are release inputs; Windows and Linux records remain target-owned work.

**Work**

- Add a versioned cross-platform desktop-size record containing target, profile, package format,
  host bytes, application payload, installed bytes, download bytes, runtime bytes, hashes, and
  toolchain versions.
- Measure frontend output and native executable sections separately.
- Add a package-content audit that counts large PE/ELF/Mach-O hosts and rejects native duplication.
- Preserve the existing standalone CLI and macOS two-binary measurements as migration baselines.

**Exit criteria**

- Every number names a target, profile, and package format.
- The 37-operation parity manifests are release inputs.
- No budget is inferred from compressed size alone.

### Phase 1 — Generalize the unified entrypoint

**Implementation status:** Portable routing and unit coverage are complete. Native Windows/Linux
GUI qualification remains part of Phase 4.

**Work**

- Refactor `crates/canisend-desktop/src/main.rs` from macOS-only mode selection into a portable,
  pure launch-mode function.
- Enable the shared CLI/Tauri dependencies for macOS, Windows, and Linux GNU in
  `crates/canisend-desktop/Cargo.toml`.
- Test no-argument GUI, renamed installed CLI, explicit CLI, MCP, Finder, case-insensitive Windows
  executable names, non-UTF-8 Unix paths, and ambiguous arguments.
- Assert that CLI and MCP modes never initialize Tauri or a WebView.

**Exit criteria**

- The same built host passes GUI, CLI, and MCP entry smoke tests on each claimed desktop target.
- Standalone CLI artifacts still build and pass their existing five-target matrix.

### Phase 2 — Rebind CLI and MCP lifecycle to the unified host

**Implementation status:** Complete and desktop-smoke-qualified on macOS. Windows/Linux lifecycle
qualification remains part of Phase 4.

**Work**

- Replace the second bundled-CLI path contract with an explicit verified current-host source.
- Keep the existing atomic install manifest, digest comparison, unmanaged-file preservation,
  downgrade refusal, rollback, and uninstall protections.
- Add Windows per-user destination and reversible PATH management without invoking a shell.
- Add Linux GNU/AppImage source-resolution tests and retain macOS signature verification.
- Make Agent/MCP configuration use the same verified source.

**Exit criteria**

- Settings can inspect/install/update/configure/uninstall the CLI on macOS, Windows, and Linux GNU.
- The copied CLI passes `version`, `doctor`, workspace, render, Agent, and MCP smoke tests.
- Moving or uninstalling the desktop package does not corrupt an already installed managed CLI.

### Phase 3 — Cut over macOS packaging first

**Implementation status:** Complete locally: exact App, ZIP, DMG, startup, accessibility, in-App
CLI installation, PATH repair, CLI commands, Agent session restart, and release-contract checks pass.

**Work**

- Update `scripts/stage_macos_gui_app.sh`, verification, startup measurement, archive smoke, DMG
  smoke, bundle metadata, and documentation to record one executable.
- Sign the host and outer app once in the correct nested order, then verify final-byte continuity.
- Run install, update, downgrade refusal, modified-file, rollback, MCP, accessibility, and archive
  qualification against the exact signed candidate.

**Exit criteria**

- The staged App contains no `Contents/Resources/bin/canisend` duplicate.
- Installed App payload is at most 72 MiB and contains exactly one full native host.
- Startup stays within the existing 2,000 ms gate and all 37 parity entries remain implemented.

### Phase 4 — Add Windows and Linux GUI packages with the same invariant

**Implementation status:** Source and packaging implementation complete; first target-owned
qualification pending. Portable Tauri compilation, Windows registry PATH management, Windows
NSIS/MSI plus offline WebView2 configuration, Linux DEB/RPM/AppImage configuration, native GUI/CLI/
MCP smoke tests, one-host extraction, and format-specific size records now live in the scheduled
qualification workflow. No Windows or Linux package is considered qualified until that workflow
passes on its native runner.

**Windows work**

- Add native Tauri Windows compilation, resource metadata, NSIS/MSI configuration, Authenticode
  verification, per-user CLI lifecycle, upgrade/uninstall tests, and both standard and offline
  installer records.
- Keep the offline WebView2 configuration in a separate build config and artifact name so it cannot
  weaken the standard-package budget.

**Linux work**

- Add Linux GNU Tauri compilation, `.deb`/`.rpm` metadata and system dependencies, desktop entry,
  MIME/icon integration, per-user CLI lifecycle, package-manager upgrade/uninstall tests, and an
  independently budgeted AppImage.
- Continue publishing Linux musl as CLI-only until GUI runtime support is intentionally qualified.

**Exit criteria**

- Each standard package contains exactly one CanISend native host and stays within the 72 MiB
  application-payload target.
- Runtime-inclusive variants publish their own truthful measurements.
- Native package-manager lifecycle and all committed operation-parity tests pass on the target OS.

### Phase 5 — Optimize the remaining single host

**Implementation status:** The latest-template four-profile matrix is implemented. The scheduled
Windows/Linux jobs now compare release ThinLTO, `s` ThinLTO, `z` ThinLTO, and `z` FatLTO. macOS
arm64 selects `z` FatLTO with a 31.47% unsigned-host reduction; signed-App, release CLI performance,
offline render, and exact GUI startup gates pass locally.
See [Unified host further size reduction plan](unified-host-further-size-reduction-plan.md) for the
exact section, payload, and performance evidence. The upgraded Typst templates are the new baseline;
see [Post-template-upgrade desktop size optimization plan](post-template-upgrade-size-optimization-plan.md)
for the detailed experiment order, template contract, and remaining promotion gates.

Do this only after duplication is removed, in descending expected return:

1. Compare canonical `release` ThinLTO with an isolated size profile using `opt-level = "s"` and
   `opt-level = "z"`; keep a change only if startup, intake, workflow, and render gates pass.
2. Inspect Mach-O, PE, and ELF sections and `cargo tree -e features` per target before changing
   dependencies.
3. Audit Typst embedded-font coverage and licensing. A smaller deterministic font set is acceptable
   only with multilingual, CJK, complex-layout, offline-render, and reproducibility fixtures.
4. Remove demonstrably unused Rust/Tauri features and duplicate generated resources.
5. Keep frontend tree-shaking, icon imports, source-map exclusion, and asset compression as a small
   regression budget; do not spend the main effort there while it remains near 1 MiB.

Do not use UPX for release binaries, split core functions into downloaded plugins, replace the full
installed CLI with an app-relative shim, or dynamically fetch rendering resources. These approaches
complicate signing, malware reputation, startup, offline operation, or rollback for less reliable
savings.

### Phase 6 — Freeze release thresholds

- Generate exact signed candidate records on macOS arm64/x64, Windows x64, and Linux GNU x64.
- Set per-target and per-format thresholds from those records with explicit headroom.
- Make release assembly depend on package-content, size, parity, CLI lifecycle, MCP, accessibility,
  upgrade/rollback, signature, and provenance evidence.
- Publish a size table that distinguishes download, installed application payload, platform runtime,
  and optional CLI copy.

## 9. Verification matrix

| Area | Required evidence |
| --- | --- |
| Launch routing | GUI/no args, CLI basename/no args, explicit CLI, MCP, Finder, invalid combinations |
| Product capability | both 37-entry parity manifests remain fully implemented |
| GUI independence | ordinary GUI operations work with no terminal CLI installed |
| CLI lifecycle | status, fresh install, update, unmanaged preservation, downgrade refusal, modified-file refusal, uninstall, rollback |
| CLI function | version, doctor, workspace, intake, render, Agent, MCP |
| Integrity | source and copied digest; macOS packaged-host signature; Windows Authenticode; Linux package/hash evidence |
| Packaging | exactly one large native host; no development files or `node_modules`; format-specific dependency audit |
| Desktop behaviour | startup, keyboard, screen reader, IME, file dialogs, window state, update/uninstall |
| Performance | existing startup, intake, render, workflow, and native-binary gates |
| Release lifecycle | exact candidate-to-public bytes, package-manager install/upgrade/uninstall, rollback, provenance |

## 10. Rollout and rollback

The migration is per platform. Keep the existing two-binary macOS packaging contract available only
as a temporary release fallback until Phase 3 is qualified. Do not delete its verification path in
the same change that first introduces the unified package.

If a platform fails CLI lifecycle, parity, signing, WebView, package-manager, accessibility, or
upgrade qualification, keep that platform on its last qualified layout or CLI-only release. Do not
restore functionality by weakening a verification control. Windows and Linux desktop publication
does not block continued standalone CLI publication on their existing targets.

Rollback of the packaging change must not uninstall or overwrite an independently installed managed
CLI. The CLI install manifest and digest continue to determine ownership.

## 11. Definition of done

The optimization is complete when:

1. macOS, Windows x64, and Linux GNU x64 standard desktop packages each contain exactly one full
   CanISend native host;
2. the app can still perform all 37 committed operation groups and install/manage its terminal CLI;
3. GUI, installed CLI, and MCP entrypoints are served by the same version-matched code without
   initializing unnecessary UI runtimes;
4. standard application payloads satisfy their signed per-target budgets;
5. Windows offline and Linux AppImage artifacts have separate, truthful runtime-inclusive budgets;
6. standalone CLI artifacts remain available for all five existing targets, including Linux musl;
   and
7. release CI blocks native duplication, unexplained size growth, feature-parity regression, and
   lifecycle or integrity failure.
