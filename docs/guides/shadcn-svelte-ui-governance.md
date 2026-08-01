# shadcn-svelte UI governance

CanISend's desktop frontend uses the checked-in shadcn-svelte registry under
`apps/canisend-desktop/src/lib/components/ui` as its single primitive UI layer. The files are
application source, not an opaque dependency: registry changes must be pinned, reviewed, tested,
and kept independent from Rust or Tauri transport changes.

## Ownership boundaries

- `src/lib/components/ui` owns registry primitives and small product-wide variants. It must not
  import feature state, feature DTOs, or the Tauri bridge.
- `src/lib/components/patterns` owns reusable presentation-only compositions such as
  `LoadingPanel`. Patterns accept labels, values, and callbacks but do not mutate a workspace.
- `src/lib/views`, feature components, and `App.svelte` own feature composition and callback
  wiring. They use primitives and patterns instead of recreating controls, alerts, loading
  panels, empty states, progress tracks, or confirmation behavior.
- `src/app.css` owns semantic theme variables and global appearance preferences. Feature source
  must not encode product status with raw palette utilities.

Primary page structure uses the presentation-only `Page` composition under
`src/lib/components/patterns/page`. Every routed view starts with `Page.Root` and `Page.Header`;
recurring page-level grids, stacks, and bordered panels use `Page.Grid`, `Page.Stack`, and
`Page.Panel`. Feature-specific form rows, data layouts, and one-off semantic elements remain local
Tailwind composition. The boundary is the reusable page contract, not a wrapper around every
leaf element.

## Visual direction

The canonical visual reference is the shadcn-svelte Nova style with the default 10 px radius, as
recorded in `components.json`. CanISend adapts that compact neutral component language to a
local-first desktop product rather than copying marketing-page content or introducing a runtime
dependency on the website.

- Light and dark themes use neutral black, white, and gray OKLCH surfaces. Product identity does
  not depend on a saturated brand color.
- The Today view uses a left-aligned 24–32 px responsive title, balanced copy, a compact badge,
  monochrome actions, and dense dashboard cards.
- Business views share the same 24–32 px title scale and compact application typography for forms
  and data.
- Controls and content surfaces use the default-feeling shared radius scale: 10 px for ordinary
  controls and 12 px for elevated cards. Cards use restrained one-pixel borders and a subtle
  shadow. Inputs and textareas use quiet filled surfaces that become bordered on focus.
- The 56 px application header and 224 px neutral sidebar preserve clear desktop navigation
  without consuming unnecessary workspace.
- Success, warning, information, and destructive colors remain semantic exceptions and are never
  used as decorative brand color.

Continue to use system fonts: matching the reference must not add a runtime font request, CDN, or
copied website asset. Standard desktop controls are 36 px in comfortable density and 32 px in
compact density, while remaining above the WCAG 24 px minimum target size.

Density is a layout contract, not a button-only preference. Feature stacks, grid gaps, nested
panel padding, Card spacing, Tabs, the application header, Sidebar rows, workspace context, and
main-content padding use the shared density variables in `app.css`. Comfortable mode keeps 16 px
section/Card spacing, a 56 px header, and 36 px controls; Compact uses 10–12 px spacing, a 48 px
header, and 32 px controls. New feature surfaces should use `--density-section-gap` and
`--density-panel-padding` instead of adding fixed `gap-4`, `space-y-4`, or `p-4` utilities.

The shared `Page.Header` owns the title hierarchy, eyebrow badge, description width, responsive
action placement, and text wrapping. Views provide translated content and an optional actions
snippet; they must not reproduce the `page-header` class tree. This keeps typography, density,
overflow handling, and future shell changes consistent across all routes.

## Adding a registry component

The workspace pins `shadcn-svelte` exactly in `package.json`. Use that workspace binary and name
only the required components:

```console
pnpm --dir apps/canisend-desktop exec shadcn-svelte@1.4.2 add COMPONENT
```

Do not run `shadcn-svelte init`: initialization can replace the customized `app.css` token
contract. Do not use `@latest`, perform a registry-wide update, or combine generated registry
churn with a business-view migration.

After generation:

1. Review every changed file, including `package.json` and `pnpm-lock.yaml`.
2. Restore unrelated dependency movement; generated commands must not silently upgrade Lucide,
   Svelte, Bits UI, or Tailwind.
3. Confirm the component has no feature imports, bridge calls, runtime network access, remote
   font, or CDN asset.
4. Apply the shared Nova desktop target, focus, invalid, dark-mode, density, and reduced-motion
   contracts where relevant.
5. Add the primitive to the development gallery and add behavior coverage for keyboard or value
   propagation when the primitive is interactive.
6. Run the focused and browser gates below before migrating feature markup.

## Local variants and semantic choices

Local changes must express a product-wide contract and stay small enough to reapply during a
future registry update. The current reviewed extensions are:

- `Button`: `desktop` and `icon-desktop` sizes driven by the shared 36/32 px density token;
- `Input`, `NativeSelect`, tabs, accordion triggers, and dialog close controls: desktop target and
  focus treatment;
- `Tabs`: the default variant uses `primary` / `primary-foreground` for a high-contrast selected
  state, while the line variant preserves its underline treatment;
- `Badge` and `Alert`: `success`, `warning`, and `info` semantic variants;
- `Alert`: persistent `role="alert"` announcement behavior;
- `Progress`: reduced-motion behavior;
- `Item`: list-item semantics when used under `Item.Group`; and
- `Sidebar`: application-owned persistence, with no component-level cookie storage. The
  `DesktopProvider`, `DesktopRoot`, and `DesktopMenuButton` variants are the reviewed path for
  CanISend's non-collapsible 960 px-minimum desktop rail; they intentionally omit the upstream
  mobile `Sheet` and collapsed-rail `Tooltip` branches so those unused interaction systems do not
  enter the startup bundle.

Use `NativeSelect` for ordinary finite choices. Use a custom popup only when search, grouping, or
rich option content is a real requirement. Use `Switch` for reversible appearance preferences and
`Checkbox` for consent or multi-selection. Destructive actions use `AlertDialog`; informational
or preview flows use `Dialog`.

Color is never the only status signal. Status text or an accessible icon accompanies every
semantic color. New colors require light and dark token pairs in `app.css`, exposure through
`@theme inline`, and automated contrast coverage.

Radius follows the same token discipline as color. `--radius` is 10 px, elevated registry cards
may use the 12 px `--radius-xl` token, and feature source delegates surface shape to primitives or
uses `rounded-lg` or smaller. Large and arbitrary feature radii are rejected by the UI source
guard. Badge pills and switch tracks are semantic shape exceptions; they must not be used as a
precedent for cards, inputs, navigation, or dialogs.

## Updating registry source

Treat an update as a dedicated maintenance change:

1. Start from a clean frontend diff and record the old and proposed exact CLI versions.
2. Update one component family at a time.
3. Diff generated output against the checked-in source and reapply only documented local
   variants.
4. Verify that public component props and Bits UI keyboard/focus behavior remain compatible.
5. Regenerate visual snapshots only after reviewing the rendered difference in every state.
6. Keep dependency updates and feature migrations in separate changes unless the dependency is a
   demonstrated prerequisite.

Never accept an update by replacing the whole `components/ui` directory. If an upstream change
conflicts with an undocumented local edit, stop and document the intended contract before
choosing either version.

## Required gates

Run the fast frontend loop for every UI change:

```console
pnpm --dir apps/canisend-desktop check
pnpm --dir apps/canisend-desktop test
pnpm --dir apps/canisend-desktop build
```

Run the browser matrix after token, primitive, shell, or cross-view changes:

```console
pnpm --dir apps/canisend-desktop test:visual
```

The browser suite covers light/dark, comfortable/compact, reduced motion, the 960 px minimum
window, 200% text, English/Chinese application shells, visual snapshots, and automated WCAG
rules. Update snapshots only for an intentional, reviewed visual change:

```console
pnpm --dir apps/canisend-desktop test:visual:update
```

The Vitest source gates reject raw standard controls, manually composed alert/loading live
regions, raw palette status colors, and large or arbitrary feature radii outside the registry and
pattern layers. Exceptions must preserve native semantics, include a source comment explaining
why a shadcn-svelte primitive is not appropriate, and be added as the smallest exact allowlist
entry instead of weakening the gate.

Broad UI or configuration changes also require the repository parity and source gates:

```console
cargo run -p xtask --locked -- desktop parity
cargo run -p xtask --locked -- release check
```

Final native qualification uses the exact staged macOS application with
`scripts/smoke_macos_gui_accessibility.sh` and `scripts/measure_macos_gui_startup.sh`. Browser tests
do not replace native WebView, file-dialog, IME, signing, or packaged-startup ownership.

For local visual review before qualification, run
`pnpm --dir apps/canisend-desktop macos:preview -- --open`. It builds an ad-hoc-signed temporary
App with an isolated HOME and synthetic long-label fixture, while retaining the browser suite as
the automated source of visual, reflow, and accessibility assertions.

## Production constraints

The gallery is development-only and is loaded only for `?ui-system=1` in a Vite development
build. The production UI must not contain the gallery, download fonts or themes, load a CDN asset,
call a browser service, or gain new direct access to workspace internals. Rust command semantics,
Tauri DTOs, consent boundaries, and local-first storage ownership are outside registry updates.
