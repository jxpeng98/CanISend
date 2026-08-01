# CanISend shadcn-svelte system migration plan

**Status:** Implemented

**Plan date:** 2026-07-31

**Completed:** 2026-07-31 — [qualification evidence](../../notes/rust-native/2026-07-31-shadcn-svelte-ui-migration.md)

**Scope:** `apps/canisend-desktop`

**Depends on:**

- [Tauri + Svelte UI migration roadmap](2026-07-27-tauri-svelte-ui-migration-roadmap.md)
- [ADR-RN-0015](../../architecture/rust-native/decisions/0015-replace-egui-with-tauri-svelte.md)

## 1. Outcome

Complete the existing partial shadcn-svelte adoption so that CanISend has one coherent visual and
interaction system across the entire desktop frontend.

This is a standardization and completion project, not a framework bootstrap. The repository
already uses Svelte 5, Tailwind CSS 4, shadcn-svelte, Bits UI, Lucide, semantic CSS variables, dark
mode, compact density, and reduced-motion support. Do not rerun `shadcn-svelte init`: the project is
already initialized and that command could replace the deliberately customized global stylesheet.

The target state is:

- registry-sourced shadcn-svelte components own interactive controls and standard UI states;
- CanISend product compositions own repeated domain-neutral layouts such as page headers, status
  panels, selectable rows, empty states, and confirmation boundaries;
- view files use Tailwind utilities only for page composition and exceptional feature layout;
- all visual meaning comes from semantic design tokens rather than raw colors or one-off classes;
- Rust commands, Tauri DTOs, local-first behavior, privacy boundaries, and workflow semantics remain
  unchanged; and
- accessibility and release qualification are at least as strong as before the migration.

## 2. Baseline inventory

The 2026-07-31 source snapshot contains:

| Area | Current state |
|---|---:|
| Application and feature Svelte source | 10,566 lines |
| Global stylesheet | 165 lines |
| View components | 8 |
| Shared product components | 3 |
| Installed shadcn-svelte component families | 12 |
| shadcn `Button` usages | 136 |
| shadcn `Card.Root` usages | 58 |
| shadcn `Dialog.Root` usages | 10 |
| shadcn `Tabs.Root` usages | 6 |
| Raw `<button>` usages outside the registry layer | 13 |
| Raw `<select>` usages outside the registry layer | 16 |
| Raw `<details>/<summary>` groups | 3 |
| Hand-composed alert states | 17 |
| Hand-composed status/loading states | 8 |
| Dashed-border empty-state patterns | 20 |
| Repeated `rounded-xl border` compositions | 129 |

These counts are migration heuristics, not product requirements. They identify repeated patterns
that should be reviewed; they do not imply that every layout `div` must become a component.

The project already has the correct foundation:

- `components.json` targets the local `$lib/components/ui` registry layer and the Nova style;
- `app.css` uses OKLCH theme variables, Tailwind v4 `@theme inline`, dark mode, density, focus, and
  reduced-motion rules;
- no feature component contains a local `<style>` block; and
- the fast CI already runs `svelte-check`, Vitest, and the production Vite build.

The remaining debt is therefore semantic and compositional: views still recreate controls,
feedback, lists, and state surfaces with long Tailwind class strings.

## 3. Migration boundaries

### In scope

- Theme-token completion and documented component variants.
- Adding missing shadcn-svelte component source through the pinned workspace CLI.
- Replacing manually styled interactive controls and repeated presentation patterns.
- Decomposing presentation-only sections of very large Svelte files when needed for safe migration.
- Component interaction, accessibility, and visual-regression coverage.
- Documentation and source checks that keep the frontend on the unified component system.

### Out of scope

- Changes to Rust domain behavior, storage, Tauri command DTOs, or Agent protocol semantics.
- Changes to the information architecture established by the existing UI migration roadmap.
- A router rewrite, state-management rewrite, or redesign of workflow rules.
- Runtime fonts, CDN assets, remote themes, or any new runtime network dependency.
- Replacing Tailwind. shadcn-svelte components are themselves styled with Tailwind; page layout
  utilities remain part of the target architecture.
- Blind registry updates. shadcn-svelte components are copied into the repository and may contain
  reviewed CanISend-specific variants, so updates must be isolated and diffed.

## 4. Target frontend architecture

```text
apps/canisend-desktop/src/
├── app.css                         semantic tokens and global preferences only
├── App.svelte                      orchestration and top-level view selection
└── lib/
    ├── components/
    │   ├── ui/                     registry-sourced shadcn-svelte primitives
    │   ├── patterns/               reusable CanISend presentation compositions
    │   └── feature components      feature-specific presentation sections
    ├── views/                      screen composition and event wiring
    ├── bridge.ts                   unchanged typed Tauri boundary
    └── *.svelte.ts / *.ts          unchanged feature state and pure logic
```

Layer rules:

1. `$lib/components/ui` contains registry components plus small, documented local variants. It must
   not import feature types or call the Tauri bridge.
2. `$lib/components/patterns` composes UI primitives into reusable product patterns. It may accept
   translated strings and typed presentation data, but it must not perform mutations.
3. Views own feature state display and callback wiring. They should not recreate standard control,
   alert, empty, loading, or selectable-item styling.
4. `App.svelte` remains the orchestration boundary. Presentation-only shell sections may be
   extracted, but bridge calls and navigation state remain centralized until a separate
   architecture decision authorizes a change.

Initial product compositions should be limited to patterns that occur at least three times:

- `PageHeader`
- `MetricCard`
- `StatusAlert`
- `LoadingPanel`
- `EmptyState`
- `SelectableItem`
- `KeyValueList`
- `ConsentField`
- `ConfirmationSummary`
- `DestructiveActionDialog`

Names may be adjusted during implementation, but each abstraction must remove real duplication and
preserve semantic HTML. Do not build a parallel generic component library on top of shadcn-svelte.

## 5. Component migration map

| Current pattern | Target | Guidance |
|---|---|---|
| Raw `<select>` | `NativeSelect` by default | Preserve native desktop, keyboard, and screen-reader behavior. |
| Complex/custom choice popup | `Select` or `Combobox` | Use only when search, grouping, or custom content justifies it. |
| Raw `<button>` | `Button`, `Item` child, or `SidebarMenuButton` | Keep semantic button behavior and existing callbacks. |
| Manual labels/help/errors | `Field` with `Input`, `Textarea`, `Checkbox`, or select | Associate descriptions and errors with the control. |
| Preference checkbox used as toggle | `Switch` | Keep actual consent checkboxes as `Checkbox`. |
| Destructive generic dialog | `AlertDialog` | Preserve explicit confirmation and irreversible-action copy. |
| Informational generic dialog | `Dialog` | Retain focus return and Escape behavior. |
| `<details>/<summary>` disclosure | `Accordion` or `Collapsible` | Select one based on single versus grouped disclosure behavior. |
| Custom progress track | `Progress` | Preserve value text and accessible value attributes. |
| Hand-composed error/success/warning | `Alert` plus semantic variants | Do not rely on color alone. |
| Loader icon plus status wrapper | `Spinner` inside `LoadingPanel` | Keep polite live-region announcements. |
| Dashed empty panel | `Empty` | Standardize icon, title, description, and action placement. |
| Bordered content/list row | `Item`/`Item.Group` | Use `Card` only when content is a true section or panel. |
| Fixed hand-built application rail | `Sidebar` compositions | Preserve visible labels and desktop window behavior. |
| Section button groups | `Tabs`, `ToggleGroup`, or buttons | Choose by semantics; navigation is not automatically a tablist. |
| Global recoverable notices | `Alert` | Keep persistent action and retry controls visible. |

`Sonner` is not required for the first migration. Important results must remain persistent and
available to assistive technology; transient toasts may be considered later for supplementary
feedback only.

## 6. Design-token contract

Complete `app.css` before migrating views so that every later component consumes the same contract.

### Color

- Retain the existing background, foreground, card, popover, primary, secondary, muted, accent,
  destructive, border, input, ring, and sidebar variables.
- Add complete semantic pairs for success, warning, and information states, including foreground
  tokens, in both light and dark themes.
- Expose semantic state colors through `@theme inline` so views can use `text-success`,
  `bg-warning/10`, and equivalent token utilities instead of `[var(--success)]` or `amber-*`.
- Add documented `Badge` and `Alert` variants for product states. Text or icons must accompany
  color in every state indicator.

### Typography

- Define roles for page title, section title, body, supporting text, metadata, micro-label, and
  monospaced artifact data.
- Replace repeated arbitrary 9–11 px labels with a documented metadata or micro-label role.
- Continue using system fonts only; no font download or packaged custom font is required.

### Spacing, radius, and elevation

- Retain the 4/8 px spacing rhythm and 12 px default radius from the accepted design direction.
- Encode comfortable and compact density in tokens or component variants rather than repeated
  per-view padding strings.
- Use restrained borders and the shadcn Nova ring model. Add elevation only to overlays or where
  hierarchy cannot be communicated with spacing and surface color.

### Interaction

- Encode the 44 px critical target requirement in shared component sizes instead of repeating
  `min-h-11` across views.
- Preserve visible focus, `aria-invalid` styling, disabled states, and reduced-motion behavior.
- Avoid dynamic Tailwind class construction that the compiler cannot statically discover.

## 7. Ordered migration stages

### UI0 — Baseline and migration guardrails

Deliverables:

- Record the inventory in this plan and add a machine-readable allowlist only where a source guard
  needs an intentional exception.
- Capture representative screenshots for light/dark, comfortable/compact, English/Chinese, and
  100%/200% text before visual changes.
- Add a development-only component/state gallery that renders primitives and product compositions
  without invoking Tauri mutations.
- Replace the current accessibility source test assertions that require raw `<button>`, `<select>`,
  and `<details>` with semantic behavior assertions that remain valid after component migration.

Exit criteria:

- The baseline state matrix is reproducible at 960×680 and 1280×820.
- No business behavior or production styling has changed.
- Fast frontend checks pass.

### UI1 — Tokens and registry primitives

Deliverables:

- Complete semantic color, typography, density, control-size, radius, and motion tokens.
- Add, in small reviewed batches, the required registry components: `native-select`, `field`,
  `alert-dialog`, `alert`, `progress`, `accordion`, `collapsible`, `item`, `empty`, `spinner`,
  `sidebar`, `tooltip`, `switch`, and `toggle-group`.
- Extend local `Button`, `Badge`, and `Alert` variants only where the product contract needs a
  documented desktop size or semantic state.
- Add primitive render and keyboard interaction tests for locally changed variants.

Implementation rule:

Use the pinned workspace binary, for example:

```console
pnpm --dir apps/canisend-desktop exec shadcn-svelte add native-select field alert progress
```

Do not use `@latest` in implementation or CI, and do not combine a registry-wide update with a
feature migration. The CLI `update` operation can overwrite local component changes, so any update
must be a separate, clean commit with an explicit diff review.

Exit criteria:

- The component gallery demonstrates all variants in light/dark and both densities.
- Theme tokens meet WCAG AA for supported text roles.
- No feature view has been behaviorally changed.

### UI2 — Application shell and global context

Primary files:

- `src/App.svelte`
- `src/lib/components/WorkspaceContextBar.svelte`
- `src/app.css`

Deliverables:

- Move the navigation rail to `Sidebar` compositions while preserving visible labels, current-page
  state, skip navigation, and the 960 px minimum window width.
- Replace shell icon buttons with shared button sizes and tooltips where their visible label is not
  present.
- Replace global hand-composed error/success notices with `Alert` compositions.
- Replace lazy-view loading wrappers with `LoadingPanel` and `Spinner`.
- Move workspace and application selectors to `NativeSelect`.
- Replace the custom progress bar with `Progress` while keeping its numeric label.
- Convert workspace section navigation to the semantically correct shadcn composition without
  changing route or navigation-memory behavior.

Exit criteria:

- Every existing top-level view remains reachable by keyboard.
- Focus remains visible and returns correctly after overlays.
- Shell layout passes at 960×680, 1280×820, and 200% text scale in English and Chinese.
- Navigation, appearance persistence, and workflow-navigation tests pass unchanged or with
  presentation-only assertion updates.

### UI3 — Forms, preferences, and safety boundaries

Primary files:

- `src/lib/views/SettingsView.svelte`
- `src/lib/views/WorkspacesView.svelte`
- `src/lib/components/ContentLibraryPanel.svelte`

Deliverables:

- Migrate form groups to `Field` and standard input/select/textarea components.
- Use `Switch` for appearance preferences and keep `Checkbox` for consent and multi-selection.
- Use `NativeSelect` for simple finite choices and retain native WebView behavior.
- Move destructive workspace and CLI operations to `AlertDialog`.
- Standardize validation, helper text, consent descriptions, pending states, and result alerts.
- Introduce `Item`, `Empty`, and `LoadingPanel` on the first representative data-heavy views.

Exit criteria:

- Every field has a programmatic label and any error is associated and announced.
- Consent and destructive confirmation copy is unchanged in meaning.
- No raw interactive element remains in these files.
- Existing Tauri callbacks receive the same values and fire once per user action.

### UI4 — Core application workspace views

Primary files, in migration order:

1. `src/lib/views/OpportunitiesView.svelte`
2. `src/lib/views/ApplicationsView.svelte`
3. `src/lib/views/ProfileView.svelte`
4. `src/lib/components/IntakeReviewSummary.svelte`

Deliverables:

- Replace selectable lead, job, source, and evidence rows with `Item` compositions.
- Replace repeated empty, loading, warning, and preview panels with shared patterns.
- Standardize filters and decision forms with `Field` and select components.
- Use semantic `Badge` variants for freshness, status, privacy, and workflow state.
- Keep revision identities, source provenance, consent boundaries, and action callbacks unchanged.

Exit criteria:

- Empty, loading, selected, stale, recoverable-error, and success states are visually consistent.
- List rows remain keyboard operable and expose selected/current state programmatically.
- Source previews and confirmations preserve their existing privacy and commit boundaries.
- Feature logic tests and desktop parity checks pass.

### UI5 — Workflow, delivery, and Agent surfaces

Primary files, in migration order:

1. `src/lib/views/WorkflowView.svelte`
2. `src/lib/views/DeliveryView.svelte`
3. `src/lib/views/AgentView.svelte`

Deliverables:

- Replace disclosure groups with `Accordion` or `Collapsible`.
- Standardize workflow stage, review finding, projection, render, runtime, skill, proposal, and
  conversation rows with `Item`, `Card`, and semantic state components.
- Migrate all remaining controls and confirmation boundaries.
- Extract presentation-only sections from the 1,120-line Workflow view and 1,702-line Agent view
  where doing so makes component migration independently testable.
- Preserve state ownership, bridge calls, cancellation behavior, task revisions, and exact
  prepared-byte boundaries.

Exit criteria:

- No source in these views recreates a standard control, alert, loading, empty, or item pattern.
- Keyboard operation covers tabs, disclosures, dialogs, cancellation, and the conversation input.
- Long paths, hashes, bilingual labels, and 200% text do not overflow or hide required actions.
- Agent and delivery behavior tests, parity checks, and packaged accessibility smoke pass.

### UI6 — Cleanup, enforcement, and qualification

Deliverables:

- Remove unused imports, obsolete class strings, duplicate patterns, and superseded components.
- Add a source gate that rejects raw `<button>`, `<select>`, and manually styled standard controls
  outside `$lib/components/ui`, with small documented exceptions only when native semantics are the
  explicit design choice.
- Add a semantic-color gate that rejects raw palette utilities for status meaning outside the
  registry/theme layer.
- Add visual-regression coverage for the representative state matrix.
- Document how to add, customize, and update shadcn-svelte components safely.
- Measure production bundle size and startup against the existing baseline.

Exit criteria:

- All definition-of-done checks below pass.
- The old presentation path can be restored by reverting a bounded migration PR; no data migration
  or Rust rollback is required.
- The source gate and native release owners accept the new frontend artifact.

## 8. Pull-request sequence

Keep each PR releasable and separate registry churn from business-view markup changes.

| PR | Scope | Expected risk |
|---|---|---|
| 1 | UI0 baseline, gallery, and test-contract update | Low |
| 2 | UI1 tokens and new registry primitives | Medium |
| 3 | UI2 shell and workspace context | High |
| 4 | UI3 Settings, Workspaces, and content forms | Medium |
| 5 | UI4 Opportunities, Applications, and Profile | Medium |
| 6 | UI5 Workflow and Delivery | High |
| 7 | UI5 Agent and remaining shared panels | High |
| 8 | UI6 cleanup, source gates, visual baselines, and documentation | Medium |

Each PR should avoid changes to bridge types or Rust commands. If a real component limitation
requires a transport change, split that work into a separately reviewed prerequisite.

A reasonable implementation estimate is 10–15 engineering days for one engineer, including
component tests and visual baselines but excluding hosted native release queues. Re-estimate after
UI2 because shell behavior and WebView accessibility carry the highest uncertainty.

## 9. Verification strategy

### Per-edit focused loop

```console
pnpm --dir apps/canisend-desktop check
pnpm --dir apps/canisend-desktop test
pnpm --dir apps/canisend-desktop build
```

Add focused component or browser tests for the files changed in the PR. Tests must exercise
behavior, not merely assert that a source file contains a class name.

### Required interaction coverage

- Keyboard navigation and visible focus for shell, lists, tabs, disclosures, and dialogs.
- Focus trap and focus return for `Dialog` and `AlertDialog`.
- Native select value propagation and custom select keyboard behavior where custom select is used.
- Field labels, descriptions, invalid state, required state, and announced errors.
- Loading, empty, success, warning, recoverable error, destructive confirmation, and disabled
  states.
- Reduced motion and no motion-dependent information.
- 200% text scale, English, Simplified Chinese, light/dark, comfortable/compact density.
- Minimum 960×680 and default 1280×820 desktop window sizes.

### Source and parity gate

```console
cargo run -p xtask --locked -- desktop parity
cargo run -p xtask --locked -- release check
```

Run the release source check after dependency, configuration, contract, or broad view changes. The
existing fast CI remains the merge gate for Svelte/TypeScript checks, unit tests, and the production
frontend build.

### Native qualification

At UI2, UI5 completion, and final migration completion:

- build the exact staged macOS application;
- run `scripts/smoke_macos_gui_accessibility.sh` against that staged app;
- verify exhaustive Tab traversal, native file dialogs, IME behavior, window drag/resize, and
  zoom shortcuts in the native release matrix; and
- compare bundle size and stable-landmark startup timing with the current baselines.

## 10. Risks and mitigations

| Risk | Mitigation |
|---|---|
| CLI update overwrites local registry changes | Pin the workspace CLI, isolate updates, and review generated diffs before feature work. |
| Custom `Select` regresses desktop keyboard or WebView behavior | Prefer `NativeSelect` for simple choices; test custom popup components in the packaged WebView. |
| Sidebar migration breaks narrow windows or 200% text | Qualify 960×680 and 200% text in both languages before merging UI2. |
| Moving markup changes callback timing or value types | Keep state and handlers in place; add interaction tests around each migrated control. |
| Large views create review and regression risk | Extract presentation sections only and migrate one state family or view per commit. |
| Semantic state is still encoded with raw colors | Complete tokens first, extend Badge/Alert variants, and add the UI6 source gate. |
| Visual tests become brittle | Snapshot stable component states and accessibility roles; avoid asserting incidental spacing pixels everywhere. |
| New components increase bundle size or startup time | Add only used component families and measure the production artifact at stage gates. |
| Accessibility is assumed from Bits UI without integration testing | Test focus, names, announcements, and packaged WebView behavior at component and native tiers. |

## 11. Definition of done

The overall migration is complete only when all of the following are true:

- All interactive controls outside `$lib/components/ui` use a shadcn-svelte primitive or a
  documented semantic exception.
- All forms use shared field, control, description, and error patterns.
- All destructive actions use a consistent confirmation boundary.
- All alerts, loading panels, empty states, progress indicators, selectable rows, and disclosure
  groups use the unified components or product compositions.
- Feature views contain no raw palette color that communicates product status.
- Comfortable/compact, light/dark, reduced motion, English/Chinese, and 100–200% text scale remain
  supported.
- Keyboard, screen-reader, focus, contrast, and 44 px critical-target requirements pass.
- The 35 operation-family Svelte parity contract remains complete.
- `svelte-check`, Vitest, production build, desktop parity, and release source checks pass.
- Packaged macOS accessibility and startup qualification pass at the existing release tier.
- No runtime font, CDN, browser service, or new frontend access to workspace internals is added.
- Component-add/update governance and the final visual state matrix are documented.

## 12. First implementation checkpoint

Start with UI0 and UI1 only. The first implementation PR must not edit a business view. Its purpose
is to establish the state gallery, replace brittle source-string accessibility assertions, finish
the token contract, and add the first missing registry components through the pinned CLI. Begin
UI2 only after those foundations are reviewable and green.
