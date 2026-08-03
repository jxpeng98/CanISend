# Known limitations

This page separates deliberate product boundaries from work that is complete only in post-tag
source. It applies to the `1.0.0-alpha.5` development line; always treat the exact downloaded
release notes and artifact manifest as the installed-binary authority.

## Publication and qualification

- `v1.0.0-alpha.5` is the latest publicly qualified checkpoint. Later work on `main`, including
  additional Generic Pack, Agent v3, desktop, and source-gate coverage, is not a published
  Alpha.6 or Alpha.7 merely because the source version still says `1.0.0-alpha.5`.
- The five-target CLI matrices and the macOS GUI channel have different qualification evidence.
  A source build or local design preview is not a signed release artifact.
- Windows and Linux public GUI artifacts are not qualified. Their CLI targets remain separate
  release-matrix owners.
- Signing, Apple notarization, Windows Authenticode, package-manager lifecycle, and clean-tag
  provenance remain release-stage gates. Follow the release manifest; do not bypass an operating
  system warning or treat an ad-hoc signature as publisher authentication.

## Workflow Packs

- Only `org.canisend.generic-application` and `org.canisend.academic-job` are embedded. External
  Pack installation, publisher trust, Pack signatures, upgrade resolution, and marketplace
  discovery are not implemented.
- A Workspace has one exact Pack binding. There is no supported in-place Pack switch, ontology
  merge, or academic-to-generic conversion.
- The Generic Pack is intentionally small: a Primary Document and optional Supporting Document,
  with Pack-declared fields, Requirement categories, stages, renderers, and validators. It is a
  reference neutral workflow, not a complete domain Pack for every grant, tender, admission, or
  regulated submission.
- Workspace v2→v3 migration preserves the Academic Pack. It requires a reviewed digest and verified
  backup and does not make old records generic.

## Generic intake and authoring

- The Generic CLI currently creates an Application from reviewed bounded JSON. Direct URL, HTML,
  local-file, and PDF normalization into a generic v3 request is not implemented. The desktop
  accepts reviewed source text and an exact Requirement excerpt; users remain responsible for
  source transcription and review.
- The current generic flow is create, Plan, compose, review, approve, render, and export. Advanced
  domain-specific Evidence extraction, reusable taxonomies, conditional forms, budgets, portal
  fields, and electronic signatures require future Packs or services.
- Generic Deliverables use the embedded bounded renderer. Layout flexibility is intentionally
  narrower than an unrestricted Typst, LaTeX, office-suite, or browser environment.

## Documents and input formats

- Scanned or image-only PDFs are not supported because embedded OCR is not implemented. Use a
  trusted OCR tool separately, review its text, and import only the reviewed result.
- Encrypted or malformed PDFs, oversized inputs, excessive pages, unsafe URLs/redirects, and
  unresolved external renderer resources fail closed.
- Editable projections are not authoritative. User edits are preserved and must be reconciled;
  CanISend never silently imports an edited projection into SQLite or immutable blobs.

## Agent and provider integration

- CanISend does not provide a direct model API or manage provider credentials. The recommended
  path is a user-controlled Codex, Claude, or generic host; the optional in-App bridge is read-only.
- Agent v3 is exact-bound to the Generic Pack. Agent v2 is an Academic compatibility surface. The
  two tool sets do not silently fall back to each other.
- Approval and preview tokens are process-local, bounded, single-use, and short-lived. They do not
  survive a restart; a fresh review is required.
- A host's search, plugins, connectors, transcript, retention, account, and model behavior remain
  outside CanISend's authority.

## Explicit non-goals

CanISend does not log in to application systems, create accounts, bypass platform controls, fill or
click through portals, upload files, send email, acquire credentials, or submit an Application.
Local export always records `submission_performed: false`. The user reviews the final package and
performs any external submission independently.

CanISend also has no telemetry, hosted account, cloud synchronization, or automatic backup. A
Workspace and every backup contain private user-owned data; storage, encryption, retention,
sharing, and secure deletion remain the user's responsibility.
