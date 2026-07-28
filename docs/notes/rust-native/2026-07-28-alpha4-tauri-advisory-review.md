# Alpha.4 Tauri transitive advisory review

**Date:** 2026-07-28

**Trigger:** Alpha.4 Candidate run
[`30345504176`](https://github.com/jxpeng98/CanISend/actions/runs/30345504176)

**Status:** Reviewed in source; replacement Candidate required

## Finding

The Candidate source gate passed formatting, Clippy, the complete workspace suite, generated
contracts, release contracts, timing checks, and compiler-cache checks. Its pinned `cargo-deny`
step then refreshed RustSec data and reported sixteen new `unmaintained` findings. It reported no
vulnerability finding, license failure, banned dependency, wildcard dependency, or unapproved
source.

Ten findings cover the GTK3 `atk`, `gdk`, and `gtk` crates plus their system, X11, Wayland, and
macro companions. `proc-macro-error` is a compile-time dependency of that same graph. Dependency
tracing binds all eleven findings to Tauri's Linux GUI backend. CanISend publishes the Tauri
desktop only for Apple Silicon macOS; its Linux GNU and musl release artifacts contain only the
standalone CLI and do not link GTK.

Five findings cover the `rust-unic` crates reached through `urlpattern` and `tauri-utils`. That
path consumes the application's checked-in Tauri configuration and capability patterns. CanISend
does not accept user-authored Tauri configuration or URL patterns.

## Decision and invariant

`deny.toml` records every advisory ID separately with its bounded reachability reason. The
advisory gate remains enabled over the complete five-target lock graph; there is no wildcard,
severity-wide, crate-wide, or global unmaintained waiver.

This decision is valid only while all of the following remain true:

- the GUI release target is macOS only;
- Linux and Windows archives contain the CLI rather than a Tauri GUI;
- Tauri capability, CSP, IPC, and application URL patterns remain checked-in product resources;
- none of the listed advisories describes a vulnerability; and
- dependency-tree review continues whenever Tauri or its plugins change.

A Linux GUI release, user-authored Tauri configuration, a newly reachable input surface, or a
vulnerability advisory for one of these crates is a release blocker until the dependency is
upgraded, removed, or isolated.

## Required verification

- pinned `cargo-deny` advisories, bans, licenses, and sources checks;
- `cargo run -p xtask --locked -- release check`;
- fast Svelte/Rust source gates; and
- a replacement non-publishing Alpha.4 Candidate before the annotated tag is pushed.
