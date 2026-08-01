# macOS App size strategy

## Measured composition

The checked-in Apple Silicon Alpha.5 baseline is `113,577,984` apparent bytes. It is not primarily
frontend content:

| Bundle component | Bytes | Share |
| --- | ---: | ---: |
| Tauri GUI executable | 60,470,016 | 53.24% |
| Bundled CLI executable | 52,836,304 | 46.52% |
| All other App files | 271,664 | 0.24% |

The two executables therefore account for 99.76% of the installed App. The Stage 4O compiled Svelte
frontend is about 684 KiB and `node_modules` is not packaged.

The historical native executables intentionally included much of the same statically linked Rust application
stack: the application facade, SQLite, URL/PDF intake, TLS, schemas/resources, and the embedded
Typst/PDF renderer and fonts. The current package removes that duplication by using one
GUI/CLI/MCP host; Settings can still install a durable version-matched terminal copy and Agent
integration starts the same host as an MCP server.

## Current single-host boundary

The staged `release-alpha` App now contains one `63,391,984`-byte signed host and a
`63,664,128`-byte allocated application payload. The versioned logical payload record is
`63,645,908` bytes, including `939,675` frontend bytes; the exact ZIP is `26,692,774` bytes.
The package rejects the old
`Contents/Resources/bin/canisend` path and enforces provisional 64 MiB host and 72 MiB payload
budgets.

Canonical release builds use symbol stripping, abort-on-panic, one codegen unit, and ThinLTO; the
measured `release-alpha` profile intentionally uses 16 codegen units without LTO. DMG/ZIP
compression can reduce download size, but it does not reduce installed App size and is not a
substitute for removing native duplication.

## Primary reduction: one multi-call executable

The material reduction path is to package one native executable that can provide both entry modes:

1. **Complete in source:** extract the Clap parser, command dispatcher, JSON renderer, and MCP
   dispatch from the current CLI binary into a reusable `canisend-cli` library API.
2. **Complete in source:** link that dispatcher into the Tauri executable. A normal Finder launch
   with no CLI command opens the GUI; an explicit CLI argument dispatches the CLI without starting
   a WebView.
3. Change terminal installation and MCP configuration to copy/use that version-matched executable.
4. Remove the second bundled executable only after CLI, GUI, MCP, upgrade/rollback, archive,
   signing, and accessibility contracts pass against the unified file.
5. Update the macOS bundle and release contracts to bind one executable digest and lower the frozen
   apparent-size budget from measured clean-tag evidence.

Because most dependencies are already linked into the GUI, the target is one roughly 60–70 MiB App
instead of two roughly 53–60 MiB executables. This is an engineering target, not a release claim;
the new budget must be set from an exact signed candidate.

The exact locally staged Apple Silicon `release-alpha` measurement is `63,391,984` bytes for the
signed unified host and `63,664,128` allocated bytes for the App, 43.95% below the historical
Alpha.5 baseline. Five LaunchServices starts reached the stable main-content landmark in a
`1,415.566` ms median and `1,650.709` ms maximum. Native candidate archive, accessibility, and
upgrade/rollback qualification remain release gates rather than assumptions from this local
measurement.

The [cross-platform desktop size optimization plan](cross-platform-desktop-size-optimization-plan.md)
extends this single-host design to Windows and Linux, preserves in-app CLI installation, separates
standard packages from runtime-inclusive offline/portable artifacts, and defines the rollout gates.
The [further size reduction plan](unified-host-further-size-reduction-plan.md) records a locally
qualified `opt-level=z` candidate with a `53,895,376`-byte signed host and `54,169,600`-byte
allocated App. It remains a candidate until native Windows/Linux packaging and exact macOS GUI
startup evidence pass; the production release profile has not been silently changed.

## Secondary investigation

After the single-executable cutover, Mach-O measurement shows that `opt-level=z` is the first
material candidate, while removing the unused Typst system-font scan path saves only 208 bytes.
The next high-potential experiment is a smaller deterministic embedded-font set. Font or renderer
changes must preserve offline rendering, reproducibility, licensing, and the existing PDF safety
limits. Dynamic Rust libraries and runtime downloads are not preferred because they weaken
portability and release integrity for a relatively small additional gain.
