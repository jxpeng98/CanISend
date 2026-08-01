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

Both native executables intentionally include much of the same statically linked Rust application
stack: the application facade, SQLite, URL/PDF intake, TLS, schemas/resources, and the embedded
Typst/PDF renderer and fonts. The App currently includes a second full CLI executable because
Settings can install a version-matched terminal CLI and Agent integration starts the same binary as
an MCP server. Moving from egui to Svelte changes the WebView UI layer but does not remove that
duplicated native stack.

## Current release boundary

The App remains within the frozen 128 MiB apparent-size budget. Do not remove
`Contents/Resources/bin/canisend` as an isolated packaging change: CLI installation, MCP
configuration, version matching, archive verification, signing, and upgrade/rollback qualification
all depend on that path today.

Release builds already use symbol stripping, abort-on-panic, one codegen unit, and ThinLTO. DMG/ZIP
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

The first Apple Silicon `release-alpha` source measurement after steps 1–2 is `63,696,000` bytes for
the unified executable. Adding the previous bundle's `271,664` bytes of non-executable content
would produce an indicative `63,967,664`-byte App, 43.68% below the checked-in Alpha.5 baseline.
This is not yet a staged, signed, or qualified package measurement: the current release scripts
still include the second CLI and the 128 MiB budget remains authoritative until steps 3–5 pass.

## Secondary investigation

After the single-executable cutover, use Mach-O section and dependency measurements to decide
whether a size-optimized release profile or a smaller deterministic embedded-font set provides a
meaningful gain. Font or renderer changes must preserve offline rendering, reproducibility,
licensing, and the existing PDF safety limits. Dynamic Rust libraries and runtime downloads are not
preferred because they weaken portability and release integrity for a relatively small additional
gain.
