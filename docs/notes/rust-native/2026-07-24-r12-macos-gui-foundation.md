# R12 macOS GUI foundation checkpoint

**Date:** 2026-07-24

**Status:** First usable vertical slice implemented and locally accepted

## Delivered

- Added `canisend-app`, a typed application facade over existing workspace, job, bounded input,
  resource, and workflow services.
- Added `canisend-gui`, a native eframe/egui desktop executable with AccessKit enabled.
- Added a body-free per-user workspace registry with atomic persistence and removal that never
  deletes workspace data.
- Added Overview, Jobs, Workspaces, and Diagnostics navigation.
- Added Command line navigation with native CLI source/destination, PATH, and active-command
  diagnostics.
- Added workspace create/register/switch/status/check/backup actions.
- Added job search, archived visibility, create/detail/archive, local file/PDF import, supplied-URL
  import, workflow start, and stage/blocker timeline.
- Added explicit UI consent language for private local reads and user-invoked network fetches.
- Added serialized background dispatch and visible operation status so SQLite, I/O, network, PDF,
  backup, and renderer work does not run in the UI event loop.
- Added design tokens, light/dark presentation, compact density, visible labels, and the first-run
  workspace chooser.
- Added the architecture ADR, execution plan, GUI guide, and CLI-to-GUI parity authority.
- Added a shared SHA-256-tracked CLI installer with clean install, update, version-aware
  migration, rollback restoration, newer-version downgrade refusal, and modified-install refusal.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo clippy -p canisend-app -p canisend-gui --all-targets -- -D warnings` passed.
- `cargo test --workspace --locked` passed 137 tests; one release-only performance test remained
  ignored by its existing policy.
- `cargo test -p canisend-cli --test binary_contract --locked` passed all 16 CLI/Agent contract
  tests.
- `cargo run -p xtask --locked -- release check` passed schemas, resources, documentation, internal
  versions, release policies, and the five-target CLI contract.
- `cargo build -p canisend-gui --release --locked` produced a 50,079,824-byte native arm64 Mach-O.
- `codesign --verify --strict` accepted the linker-generated ad-hoc signature.
- Native launch smoke opened a stable `CanISend` window with an `1120 × 740` content size; a second
  launch after contrast corrections produced the same window identity and size.

## Preserved invariants

- The GUI never spawns the CLI or an arbitrary shell.
- Terminal integration copies only the known bundled CLI; it does not run shell installation
  scripts or edit shell profiles.
- The GUI opens the same Rust v2 workspace used by CLI and Agent v2.
- Routine read models and the registry do not retain private source bodies.
- File, PDF, and URL inputs use existing bounded adapters.
- Registry removal does not delete workspace data.
- Existing CLI/Agent v2 response and binary contracts remain green.
- The application prepares material and does not submit applications.

## Remaining R12 work

- Route the remaining CLI action families through `canisend-app` and make the CLI a thin adapter.
- Add workspace restore/repair and workflow begin/complete/rerun screens.
- Add criteria, evidence, match, plan, discovery, agent task, document, review, package, render, and
  export screens.
- Add deterministic GUI harness coverage plus native VoiceOver, IME, high-DPI, text scaling,
  reduced-motion, and file-dialog qualification.
- Build and verify a macOS `.app` bundle, application icon, notices, ad-hoc signing evidence,
  packaged workflow smoke, upgrade/rollback, uninstall retention, and Intel build.
- Defer Windows and Linux GUI packaging until the macOS Alpha path is accepted.

## 2026-07-25 terminal bridge update

The next macOS slice added:

- typed `cli.install.status`, `cli.install`, and `cli.uninstall` actions in `canisend-app`;
- SHA-256-managed user-level installation with atomic replacement, explicit preservation of a
  pre-existing file/symlink, restoration on uninstall, and refusal to overwrite/remove changed
  managed content;
- exact sibling and `CanISend.app/Contents/Resources/bin/canisend` discovery;
- the Command line GUI page with installed/bundled CanISend versions, active-command/PATH
  diagnostics, one-click migration/upgrade, background lifecycle actions, copyable verification
  commands, and scrollable content;
- a macOS `.app` staging script, plist, nested version-matched CLI, executable hash manifest,
  legal/privacy material, and free ad-hoc integrity signatures.

Focused verification passed seven `canisend-app` tests, four `canisend-gui` tests, strict Clippy,
all sixteen CLI binary-contract tests, and the source release check. An optimized Apple Silicon app
bundle passed deep `codesign` verification, launched with the embedded CLI, exposed the new page
and controls through AccessKit, detected an existing CanISend command without changing it, and made
the lower command controls reachable by scrolling. The user's real terminal installation and
workspaces were not mutated during the smoke.

## 2026-07-25 version and update refinement

The terminal bridge now checks only CanISend product versions. It does not discover Python,
package-manager, or runtime environments. Fixed version probes run directly against the exact
destination under short time and output bounds; version-unaware earlier interfaces remain eligible
for a user-invoked migration with rollback. Older versions receive an Upgrade action, same/unknown
versions receive a Migrate action, and newer versions cannot be downgraded.

The same page now has a manual **Check for updates** action backed by the allowlisted, bounded public
GitHub Releases adapter. Preview builds include published prereleases and Stable builds ignore
them. The action sends no workspace content, does not run automatically, and never downloads or
executes an installer.
