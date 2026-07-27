# ADR-RN-0015: Replace egui with a Tauri and Svelte desktop UI

- Status: Accepted
- Date: 2026-07-27
- Decision owner: CanISend maintainer

## Context

CanISend's application, contracts, storage, intake, rendering, CLI, and release-integrity layers are
already Rust-native. The current desktop presentation layer is implemented with `eframe`/egui.
It reached complete CLI/GUI operation-family parity, but its visual system and component model no
longer provide the product quality or iteration speed required for the 1.0 desktop experience.

This decision changes only the desktop presentation/runtime boundary. It does not move domain
logic into JavaScript and does not change the local-first workspace authority.

## Decision

The supported desktop application will use:

- Tauri 2 as the Rust-owned desktop runtime and command boundary;
- Svelte 5 with TypeScript and Vite for the bundled frontend;
- shadcn-svelte components backed by Bits UI accessibility primitives;
- Tailwind CSS with checked-in semantic design tokens; and
- Lucide Svelte for product-interface icons.

The frontend must not define hand-drawn SVG icons, icon fonts, emoji navigation icons, or generated
brand approximations. A missing icon must be selected from Lucide before another established icon
family is considered.

Node.js and pnpm are build-time dependencies only. Tauri embeds the compiled static frontend in the
desktop executable, so end users continue to receive a standalone application without a Node,
Python, browser-server, or package-manager runtime requirement.

`canisend-app` remains the only product application facade. Tauri commands validate bounded input,
map serializable transport DTOs, call `canisend-app`, and return typed success/error envelopes.
The Svelte frontend must never read or mutate `.canisend/` internals directly.

## Migration boundary

The existing egui binary remains the public desktop implementation until the Svelte application
passes the committed 35-operation parity manifest and packaged macOS qualification. A new
`canisend-desktop` workspace member is developed beside it. The final cutover is atomic:

1. rename the qualified Tauri binary to the existing `canisend-gui` release executable;
2. update macOS staging, startup, accessibility, and release-integrity checks;
3. remove the egui crate, dependency, fonts, notices, and renderer-specific evidence; and
4. keep the CLI, workspace, Agent v2, schema, and package contracts unchanged unless separately
   authorized.

## Security and privacy

- Tauri capabilities are allowlisted per window; no global shell or filesystem capability is
  enabled.
- File and directory choices use the maintained Tauri dialog plugin and user gestures.
- Network-backed operations still require the existing explicit consent types.
- Private reads and exports still require their existing explicit consent types.
- Frontend logs, browser storage, and error messages must not persist private document bodies.
- Content security policy permits only bundled application assets and explicitly required Tauri
  IPC endpoints.

## Consequences

The repository gains a build-time JavaScript toolchain and WebView-based accessibility/runtime
qualification. Fast CI therefore separates frontend checks from Rust domain tests, while native
release qualification builds the frontend once before the Tauri executable.

The Beta transition is postponed until the Svelte desktop reaches parity and the egui runtime is
removed. The first releasable migration checkpoint is expected to be another explicitly authorized
Alpha, not an automatic Beta.
