# Stage 4G connected workflow local qualification

**Date:** 2026-07-30

**Baseline:** `v1.0.0-alpha.4` at `b42817e812a4444dbcd2dd9f5c4c3c2ed50a96a9`.

**Candidate:** local annotated tag `v1.0.0-alpha.5`.

## Outcome

The connected Svelte workflow passed its focused source gates and a local Apple Silicon macOS
package lifecycle. The workspace, exact internal dependency pins, lockfiles, desktop metadata,
release notes, workflow default, package contract, GUI parity scope, and measured performance
baseline all identify `1.0.0-alpha.5`. The reviewed source is bound locally by the sequential
annotated tag `v1.0.0-alpha.5`.

This record proves that the tagged source can produce and run the expected ad-hoc-signed ZIP and
DMG on the development Mac. It is local qualification, not public release evidence: the tag has
not been pushed, no GitHub release was created, and the five-target native GitHub release matrix
has not run.

## Source and browser evidence

- Svelte check completed with zero errors and zero warnings.
- Five frontend test files completed with 25 passing tests, including an architecture guardrail
  that returns a newly selected workspace to external-host handoff and clears rendered
  conversation state.
- The production frontend build completed without the former 500 kB chunk warning.
- Focused application, desktop, and MCP Rust tests passed; the desktop suite completed with 32
  passing tests, including exact-scope process cancellation and a runtime-evidence regression
  that forbids inferred authentication or host configuration. The application suite completed with
  60 passing tests and one intentionally ignored public-endpoint test; it now proves that a stale
  workspace session binding cannot block an active workspace.
- Strict Clippy checks passed for the application, CLI, MCP, and GUI targets.
- The source release gate reported 35 CLI/GUI and 35/35 Svelte parity operations.
- Browser checks covered English and Simplified Chinese, dark mode, the 960 px minimum window,
  global route restoration, lazy Agent loading, and the absence of horizontal overflow.

## Native package evidence

The first Stage 4G native release-alpha build completed in 2 minutes 7 seconds; after regenerating
the production Svelte assets, the final Alpha.5 versioned rebuild completed in 49.88 seconds:

```text
cargo build --profile release-alpha --locked \
  -p canisend-cli -p canisend-gui --features canisend-gui/custom-protocol
```

The standard macOS packaging script created a pre-tag portable ZIP and DMG with the
version-matched bundled CLI and ad-hoc signatures. Hashes for that performance-baseline package
were:

- ZIP: `413ebb75d31fda61a6f7254045587e939c1a3b6569fd60a231ec6f82739d67b2`
- DMG: `11cbc8a489faacf2d8f62969ac4d97c46b61639a9c7fcf68a95bdbf05a28ba1d`

These digests are not clean-tag publication identities. The CLI embeds its Git source revision,
so the post-commit clean-tag build intentionally produces different bytes. Exact clean-tag
digests belong to the external native qualification record generated after the source commit and
must not be committed into that same source commit as if they were a reproducible fixed point.

The ZIP smoke verified bounded extraction, frozen top-level layout, final-byte integrity,
version matching, bundled CLI diagnostics, the documented quickstart, the external-host Agent
workflow, and packaged GUI launch. The read-only DMG smoke verified its checksum, frozen layout,
`/Applications` link, manifest, and nested and outer ad-hoc signatures.

The packaged accessibility smoke used an isolated HOME and workspace to verify the Svelte
navigation and main landmarks, automatic CLI status, consent-gated PATH repair with an exact
managed `.zprofile` block, localized revisioned profile initialization, bilingual native control
names, 200% text scale, Command-0 reset, reduced motion, tab switching, clean quit, and
route/locale restoration after relaunch. A fixed local fake Codex executable performed no network
request and proved that the App can cancel the exact running scope without saving partial output.
The harness then created `fixture-session-1`, quit the packaged App, relaunched it with the same
isolated HOME and workspace, and verified that the next turn used the exact
`codex exec --sandbox read-only resume --json --skip-git-repo-check fixture-session-1 -`
invocation. It forces the
fixture through an isolated PATH and fails if a real Codex home is created, so the automated gate
cannot silently fall through to a user runtime or provider. Both new and resumed Codex turns now
carry an explicit read-only sandbox argument; Claude turns retain explicit plan mode.

The Agent MCP configuration view reports nine read-only/preview tools separately from four
approval-gated writes. The MCP protocol suite verifies that the same Rust category constants
match every advertised `readOnlyHint`, so the GUI cannot drift from the host-visible permission
contract. The packaged accessibility smoke also prepares the MCP configuration on the default
external-host surface and asserts both counts before entering the optional in-App bridge.

Runtime discovery reports only the observed executable path and bounded version output. The
packaged Agent bridge labels authentication, provider access, search, MCP, skills, and plugins as
host-managed and unverified; its accessibility smoke rejects the former inferred “Local sign-in”
claim before allowing the fixture turn.

The bundled CLI returned the full shared profile-source read model (`workspace`,
`profile_revision`, and `sources`) after initialization; a binary-contract regression test protects
that GUI/CLI parity.

The CLI lifecycle smoke rebuilt the release CLI and passed migration, update, rollback, uninstall,
and workspace-retention coverage in an isolated local fixture.

The startup gate now discovers the stable main-content landmark by accessibility name instead of
assuming an obsolete fixed WebView child index. Each of its five trials uses the repository-owned
fake Codex runtime and a minimal PATH, so runtime discovery cannot invoke a user provider or
package-manager shim. The isolated Alpha.5 result passed with a 1535.746 ms median, 1613.504 ms
maximum, 60,470,016-byte GUI executable, and 113,577,984-byte apparent App bundle.

## Remaining publication gates

- Run consented real-account Codex and Claude turns through their normal local configuration;
  automated tests already prove exact host-owned session resume with fixed local runtimes and
  never transmit private workspace context.
- Push the annotated tag only after explicit authorization, then qualify the exact candidate bytes
  in the five-target native GitHub release matrix before creating a public GitHub release.
