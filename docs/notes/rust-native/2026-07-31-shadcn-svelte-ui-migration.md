# shadcn-svelte UI system migration completion

**Completed:** 2026-07-31

**Scope:** `apps/canisend-desktop`

**Plan:** [CanISend shadcn-svelte system migration plan](../../superpowers/plans/2026-07-31-shadcn-svelte-system-migration-plan.md)

## Outcome

The desktop frontend now uses the checked-in shadcn-svelte registry as its single primitive UI
system. The migration covers the application shell, workspace context, Today, Workspaces,
Opportunities, Applications, Workflow, Delivery, Profile, Agent, Settings, content-library, and
intake-review surfaces without changing Rust command behavior, Tauri DTOs, workflow rules, storage
ownership, prepared-byte boundaries, or consent semantics.

The resulting layer contract is:

- `components/ui`: registry primitives plus reviewed semantic and desktop variants;
- `components/patterns`: mutation-free product compositions such as `LoadingPanel` and the
  development-only state gallery;
- views: feature composition and callback wiring; and
- `App.svelte`: bridge ownership, navigation memory, appearance persistence, and lazy view
  orchestration.

Raw standard controls are confined to the registry layer. Alerts, loading, empty, progress,
selectable-row, disclosure, field, and destructive-confirmation states now use shared primitives or
patterns. Semantic success, warning, information, and destructive tokens cover both themes and
pass the automated contrast matrix.

## Visual and accessibility matrix

The pre-migration fixtures remain documented in the
[UI0 baseline](2026-07-31-shadcn-ui-baseline.md). Final Playwright snapshots are checked in beside
`tests/visual/ui-system.visual.spec.ts`.

| Viewport | Theme | Language | Density | Text/motion | Qualification |
|---:|---|---|---|---|---|
| 1280×820 | Light | English | Comfortable | 100%, normal | snapshot + Axe application shell |
| 1280×820 | Dark | English | Compact | 100%, reduced | snapshot + Axe gallery |
| 960×680 | Dark | Simplified Chinese | Compact | 200%, reduced | Axe application shell |
| 960×680 | Light | English | Comfortable | 200%, normal | snapshot + overflow check |

The browser suite also opens every deferred product view through the primary navigation. The exact
staged macOS application passed named Svelte landmarks, CLI PATH repair, profile initialization,
external-first MCP permissions, bounded runtime evidence, Agent cancellation and session resume,
route/locale restart, bilingual native control names, 200% text, 100% reset, and reduced motion.

The native accessibility harness now reads the WebView tree as a bounded flat collection instead
of recursively walking the deeper component tree. Agent fixture prompts are assigned through the
text area's AX value so the test does not depend on the operator's current IME. The startup probe
uses the owned Tauri/WebKit wrapper path and inspects only direct `AXWebArea` children for the same
named main landmark, avoiding probe-induced full-tree latency.

## Bundle and startup

The production gallery is eliminated by the build and no runtime font, CDN, theme, or browser
service was added. All product views, including Today, load on demand. The non-collapsible desktop
rail uses reviewed shadcn Sidebar desktop variants that omit unused mobile Sheet and collapsed-rail
Tooltip branches.

| Measurement | Before | Final | Change / budget |
|---|---:|---:|---:|
| Complete Vite `dist` bytes | 682,419 | 800,942 | +17.4% across all deferred chunks |
| Initial synchronous bytes | 682,419 | 432,390 | −36.6% |
| Initial synchronous gzip | about 168.2 kB | 108,194 bytes | about −35.7% |
| GUI executable | 60,470,016 bytes | 63,358,640 bytes | +4.8%; 67,108,864 budget |
| App apparent size | 113,577,984 bytes | 116,805,632 bytes | +2.8%; 134,217,728 budget |
| Startup median | 1,535.746 ms | 1,105.707 ms | −28.0% |
| Startup maximum | 1,613.504 ms | 1,214.694 ms | −24.7%; 2,000 ms budget |

The exact five samples and signed executable hashes are recorded in
[`macos-gui-shadcn-migration.json`](../../performance/macos-gui-shadcn-migration.json).

## Verification record

| Gate | Result |
|---|---|
| `pnpm --dir apps/canisend-desktop check` | 0 errors, 0 warnings |
| `pnpm --dir apps/canisend-desktop test` | 10 files, 50 tests passed |
| `pnpm --dir apps/canisend-desktop test:visual` | 8 Playwright/Axe tests passed |
| `pnpm --dir apps/canisend-desktop build` | production build passed |
| UI source guard | raw controls, manual standard states, and raw status palettes rejected |
| `cargo fmt --all -- --check` | passed |
| workspace Clippy with `-D warnings` | passed |
| `cargo test --workspace --locked` | passed; policy-owned ignored tests remained ignored |
| `cargo run -p xtask --locked -- desktop parity` | 37/37 checks passed |
| `cargo run -p xtask --locked -- release check` | passed |
| `release-alpha` app build, stage, signature, and manifest | passed |
| packaged macOS accessibility smoke | passed |
| five-trial startup and size qualification | passed |

Component add/update rules, local variants, source gates, and required verification commands are in
the [shadcn-svelte UI governance guide](../../guides/shadcn-svelte-ui-governance.md).
