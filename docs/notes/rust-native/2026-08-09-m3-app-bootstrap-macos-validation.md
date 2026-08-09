# M3 App bootstrap packaged-macOS validation

**Date:** 2026-08-09

**Roadmap item:** `M3-BOOTSTRAP-001`

**Product source:** merge `0a2e8b5671dba08d6357842989942cffe5f6999e`

**Data:** synthetic, body-free, local temporary directories only

## Isolation correction

The first design-preview attempt used the production bundle identifier. macOS WebKit reused the
production App's local navigation domain even though the launcher supplied a temporary `HOME`.
The preview was closed immediately before any mutation and was not accepted as evidence.

The validation harness was then corrected to restage the preview with bundle identifier
`io.github.jxpeng98.canisend.design-preview`, display name **CanISend Design Preview**, a refreshed
ad-hoc signature, and refreshed companion hashes. It also replaced compatibility-era `job create`
fixtures with one generic and one academic clean-v4 Application in the same synthetic Workspace.
The release source gate now requires those isolation and fixture markers.

## Packaged App journey

The corrected `release-alpha` App passed bundle, host, signature, size, and companion-integrity
verification. Its isolated App UI showed only the synthetic registry and separate WebKit state.
Using the packaged UI, without a terminal operation in the user journey, the validation then:

1. opened **Workspaces → Create workspace**;
2. entered a display alias;
3. kept Codex selected and selected Claude;
4. selected a pre-created empty temporary directory through the native directory dialog;
5. confirmed the dialog's no-Profile, no-private-content, no-Application, and no-mode boundary;
6. completed setup; and
7. closed and reopened the App.

After reopen, the new alias remained selected and the Workspace health surface reported schema 20.
A body-free filesystem/status check confirmed:

- `canisend.workspace/v4` with zero Applications;
- both managed Agent v4 Skill manifests and all four Skills per host;
- `.codex/config.toml` and `.mcp.json`;
- the registry alias and canonical temporary path;
- no Profile or private artifact body; and
- successful standalone `workspace status` while the App was not required as state authority.

## Local artifact identities

These hashes identify the non-publishable local validation preview; they are not release
qualification or public Alpha.7 evidence:

- unified host SHA-256:
  `96a728ab917101523b1f69cfc1377b7283f896a6a6abfeb411de5a390c771482`;
- companion integrity manifest SHA-256:
  `a006ef959d0b90a3ee56216057b93f6c3ae87552d2be7923c516e0a358a9648b`;
- preview receipt SHA-256:
  `e11993cada589be27751f1ec06f437b85c2f22eefb23f46c2625a08d20801993`.

The preview was ad-hoc signed, not notarized, and fixed `publication_allowed` to `false`. Alpha.7
still requires an exact clean-tag candidate, native release qualification, public download, and
independent public-byte reverification.
