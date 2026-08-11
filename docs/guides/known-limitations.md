# Known limitations

It applies to the published `v1.0.0-alpha.8` checkpoint and later source with the same version.
It is not publication evidence; the exact downloaded release notes and manifest remain the
installed-binary authority.

## Publication and qualification

- `v1.0.0-alpha.8` is the latest publicly qualified checkpoint. It was built once from
  `35e7c822ea2f469ab726a31b5d08e622f6810c55`, promoted without rebuilding, downloaded, and
  independently reverified.
- `v1.0.0-alpha.7` and earlier tags remain immutable historical checkpoints.
- Windows and Linux public GUI artifacts are not qualified. Their standalone CLI targets have
  separate native release-matrix owners.
- Community signatures do not establish an operating-system-trusted publisher. Notarization,
  Authenticode, provenance, and package-manager lifecycle remain release-stage gates. Never disable
  an operating-system security control globally.

## Clean v4 compatibility boundary

- Alpha.7 and later initialize only `canisend.workspace/v4` and use `canisend.agent/v4`.
- Alpha.6-or-earlier Skills, Agent v2/v3 requests, job aliases, host-resource layouts, and Workspace
  v2/v3 files are unsupported. They fail before mutation; there is no hidden migration or
  compatibility negotiation.
- Approval tokens are process-local, short-lived, bounded, and single-use. Guarded mutations must
  remain in one running App or MCP process; a restart requires a fresh preview.
- Only `org.canisend.generic-application` and `org.canisend.academic-job` are embedded. External
  Pack installation, publisher trust, signatures, upgrade resolution, and marketplaces are not
  implemented.

## Applications, intake, and authoring

- One Workspace may contain both Packs, but one Application cannot merge Packs.
- Shared Profile Sources and Evidence are not implicitly visible across Applications. Users must
  review explicit associations and any private-read consent.
- URL, pasted text, local files, and text PDFs are bounded inputs. Unsafe redirects, oversized
  input, invalid spans, stale digests, duplicates, and missing consent fail closed.
- The Generic Pack is a small reference workflow, not a complete regulated-domain form, budget,
  portal, or electronic-signature system.
- Editable projections are not authority. CanISend preserves user edits and requires explicit
  reconciliation rather than silently importing or overwriting them.

## Documents and host integration

- Scanned or image-only PDFs require a separate trusted OCR tool and user review.
- User-authored Typst, external Typst packages/files, unrestricted system fonts, browser
  automation, and portal automation are outside the current scope.
- CanISend does not provide model credentials or a hosted provider. Codex, Claude Code, and other
  MCP clients own their authentication, conversations, plugins, search, and retention.
- Exact Codex CLI, Claude Code, Claude Desktop, and bounded MCP-host dogfood passed Alpha.8
  qualification. Real invited-user evidence remains a Beta-readiness gate.

## Product boundary

CanISend does not log in, create accounts, acquire credentials, bypass platform controls, fill
portals, upload files, send email, or submit an Application. Local export always records
`submission_performed: false`; the user reviews and performs any external submission.

CanISend also has no default telemetry, hosted account, cloud sync, or automatic backup. A
Workspace and every backup contain user-owned data; storage, encryption, retention, sharing, and
secure deletion remain the user responsibility.
