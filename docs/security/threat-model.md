# CanISend Rust-Native Threat Model

**Review state:** R10.1 release-candidate baseline

**Reviewed:** 2026-07-18

**Scope:** Native CLI, workspace storage, local/remote intake, discovery adapters, host-agent task exchange, structured
document generation, embedded Typst rendering, export, backup/restore, embedded resources, and native packaging.

## Security objectives

CanISend must preserve the confidentiality and integrity of private application material while allowing a user to
coordinate local agents and explicitly chosen providers. It must not submit an application, transmit private data,
follow instructions embedded in imported material, overwrite unmanaged files, or treat an editable projection as
authoritative. A malformed or malicious input must fail within documented size, path, state, and network boundaries.

Availability is bounded for I/O, parsers, and generated render source. CanISend is a single-user local tool, not a
multi-tenant service or privilege boundary. A process already running as the user can read the user's workspace and is
outside this model.

## Assets and classifications

| Classification | Examples | Required handling |
| --- | --- | --- |
| `public` | Version, capabilities, schema IDs, adapter catalog | Safe in normal CLI output and release metadata |
| `private-local` | Job advert bodies, profile sources, evidence, drafts, reviews, PDFs | Local verified blobs; body-free routine status and audit metadata |
| `provider-bound` | Exact task input revisions approved for configured-provider use | Create-new private bundle only after per-operation read and send consent |
| `secret` | Future provider credentials or signing material | Never stored in workspace, SQLite, task bundles, logs, or release archives |

The current native product has no provider HTTP client and accepts no provider token. `configured-provider` mode only
creates a consent-bound local bundle; the invoking user or host is responsible for any later transmission.

## Trust boundaries and data flow

1. Local files, PDFs, URLs, feeds, provider APIs, CSV/JSON imports, agent candidates, and edited projections are
   untrusted.
2. Intake adapters normalize bounded content into immutable SHA-256-addressed blobs and revisioned metadata.
3. Task descriptors select exact private revisions. Consent-gated export copies only that declared scope to a new
   external directory.
4. Agent output crosses back through structural, semantic, identity, source-span, lease, revision, and dependency
   validation before one atomic commit.
5. Structured reviewed documents are authoritative. Typst is regenerated from a fixed embedded template with every
   data string escaped; edited `.typ` files never enter the compiler.
6. Explicit export copies verified artifacts to new or empty destinations. Export is not submission.

The SQLite database and content-addressed blob store form one authoritative workspace. Markdown, JSON, Typst, PDFs,
agent packs, task bundles, backups, and release archives are projections or transport artifacts and do not acquire
authority merely because they exist on disk.

## Threat register

| ID | Threat | Control | Automated evidence |
| --- | --- | --- | --- |
| T01 | A malicious URL targets loopback, link-local, private, reserved, or metadata services | Only HTTP(S), no userinfo or proxy inheritance, checked DNS answers pinned into the client, private/reserved IPv4 and IPv6 rejected | `remote::tests::url_and_address_policy_rejects_credentials_and_non_public_ranges`; `remote::tests::local_server_covers_redirect_size_mime_timeout_and_private_address_policy` |
| T02 | A redirect bypasses SSRF or a provider host allowlist | Redirects are manual, limited to five, re-resolved and rechecked before every request; HTTPS cannot downgrade; provider adapters require an exact case-insensitive host match on every hop | `remote::tests::provider_redirect_host_policy_is_exact_and_case_insensitive`; local redirect fixture |
| T03 | Remote content causes decompression, MIME-confusion, oversized-body, or timeout abuse | Non-identity content encoding rejected, declared and streamed byte limits enforced, MIME sniffing must agree, connect/request timeouts applied | `remote::tests::content_sniffing_rejects_misleading_mime_and_html_is_normalized`; remote size and timeout fixtures |
| T04 | A malformed, encrypted, scanned, or oversized PDF corrupts state or exhausts resources | Regular non-symlink file requirement, 16 MiB input/text limits, parse and encryption checks, page limit, typed no-text result, no commit before extraction succeeds | `pdf::tests::text_pdf_is_extracted_by_page_and_invalid_inputs_are_typed`; `candidate::tests::file_must_be_regular_json_and_not_a_symlink` |
| T05 | Prompt injection in an advert/profile/document controls a host agent | Every bundled agent guide and operation prompt labels bodies as untrusted data; task scopes contain exact immutable revisions; candidate fields and references are validated | Embedded resource integrity test; end-to-end leased task completion test; prompt/resource drift gate |
| T06 | Candidate JSON writes arbitrary paths, changes identities, or invents evidence | Unknown fields denied, strong IDs/revisions/digests, portable `SafeRelativePath`, task-owned IDs, source spans, evidence membership, and exact expected inputs validated | contract validation tests; `safe_relative_path_rejects_escape_and_internal_state`; task E2E tests |
| T07 | Symlink, path escape, device name, race, or overwrite redirects an export into managed/private state | Internal symlinks fail closed; portable paths reject `..`, drive prefixes, control characters, reserved Windows devices, trailing dot/space; sensitive exports use create-new files and new/empty destinations | `workspace_and_blob_symlinks_fail_closed`; task/package/render export tests; path primitive tests |
| T08 | A blob is replaced, truncated, or substituted | Blob location is derived from a validated digest; reads rehash and enforce a limit; writes are immutable create-new operations; workspace check audits referenced and unreferenced blobs | `blobs_are_bounded_immutable_verified_and_auditable`; workspace check tests |
| T09 | A corrupt database or forged backup is restored | SQLite integrity and workspace identity checks; backup database/config/blob manifest digests; exact reference set; no symlinks; restore into a new destination | `verified_backup_restores_into_new_workspace`; R10.2 cross-platform interruption matrix |
| T10 | Concurrent agents replay, race, or complete stale work | Immediate SQLite transactions, two-second busy timeout, leases, expected job/input revisions and hashes, single status transition, idempotent exact replay, downstream invalidation | leased task E2E; `expired_lease_is_durably_marked_stale`; R10.2 concurrency tests |
| T11 | A configured provider receives more private material than approved | Separate `read-private-inputs` and `send-to-configured-provider` consents; manifest digest binds exact declared revisions; exporter re-verifies scope and writes only those blobs | configured-provider consent assertions and task input export E2E |
| T12 | Logs or routine responses disclose private bodies or provider payloads | No telemetry or logging framework; routine status/audit entries contain IDs, counts, hashes, action names, and body-free reasons; explicit show/export responses are user-requested data access | private sentinel assertions in workflow/task/profile CLI and store tests; static dependency/source review |
| T13 | An edited Typst projection injects code or becomes authoritative | Renderer reloads structured blobs, escapes every string into a Typst literal, uses a fixed embedded template and in-memory world, disables system fonts, and ignores managed `.typ` projections | projection escaping tests; full revision-bound render/export E2E with malicious edited `.typ` fixture |
| T14 | Embedded rendering consumes unbounded CPU/memory or reads external resources | Generated source capped at 1 MiB, PDF at 16 MiB, elapsed budget checked, in-memory world, embedded fonts only, no package/network/filesystem lookup; output PDF reparsed | render limit and forbidden-read tests; cross-platform render probe; dependency policy gate |
| T15 | Embedded resources or release contents are replaced | Compile-time declared resource inventory and SHA-256 digests, no resource symlinks, runtime verification, release notices, staged bundle checks, locked Cargo graph | resource manifest tests, `xtask release check`, packaged-binary smokes; R11 checksums/SBOM/provenance/signatures |
| T16 | Successful drafting, review, render, or export is mistaken for permission to submit | Contracts and manifests require `submission_performed: false`; no submission command or browser automation exists; agent guides repeat the boundary | package/render schema tests and E2E assertions |

## URL and redirect audit conclusion

The transport owns redirects rather than delegating them to the HTTP library. Each hop is parsed, stripped of its
fragment, checked for credentials and scheme, resolved without environment proxies, rejected if any answer is
non-public, and pinned to the checked addresses. HTTPS downgrade is rejected. R10.1 additionally binds jobs.ac.uk,
Greenhouse, and Lever redirects to the adapter's exact host allowlist before the next request is sent. Generic job and
RSS URLs may redirect to another public host because cross-domain publisher/CDN redirects are an intended feature.

DNS rebinding is mitigated for each request by passing only the already checked address set to the client. A new DNS
resolution is performed and rechecked for each redirect hop.

## Archive, resource, and path audit conclusion

The product does not ingest ZIP, TAR, or other general-purpose archives. Native backups are directory manifests, and
restore rejects symlinks, special files, digest mismatches, missing/extra referenced blobs, database corruption, and
non-empty destinations. If archive ingestion is introduced, it requires a new threat review before implementation.

Embedded resources are a compile-time closed inventory. The build rejects undeclared files, missing files, duplicate
IDs/paths, unsafe relative paths, and symlinks; runtime verification rehashes every byte. Agent packs use new files in a
new/empty private directory. Workspace writes use validated relative paths, checked parent directories, and
create-new/atomic replacement semantics according to whether the projection is managed.

## Privacy, output, and provider-payload audit conclusion

There is no background telemetry, crash upload, tracing subscriber, log file, or provider HTTP transport. stdout is
either a requested human result or the versioned JSON response; stderr contains a single typed command error. Routine
workflow, task, workspace, discovery, and audit results expose metadata rather than source bodies. Commands that show a
structured private document are explicit local reads, not routine logging.

For configured-provider work, CanISend constructs a private directory from `private_read_scope` only after both consent
flags are present. Every item is checked against the task's persisted scope and current immutable revision. The
manifest records only IDs, revisions, paths, and hashes. The content files contain the exact approved blobs; no
workspace config, unrelated evidence, audit data, credentials, environment variables, or implicit context is added.

## Dependency and license audit

The complete five-target release graph is checked by pinned `cargo-deny 0.19.7` policy in `deny.toml`. CI fails for a
new advisory, an unapproved license, wildcard dependency, Git dependency, or non-crates.io registry source.

The 2026-07-18 audit found no advisory in direct CanISend dependencies. It identified these transitive Typst findings:

| Advisory class | Dependency | Decision |
| --- | --- | --- |
| CPU and memory denial of service | `quick-xml 0.38.4` through `citationberg`/`hayagriva` | Audited exception: bibliography/CSL/XML entry points are absent from the fixed template; all user values are escaped literals; the Typst world has no external files; source is capped at 1 MiB |
| Unmaintained | `bincode`, `paste`, `yaml-rust` | Audited exception: transitive Typst syntax/citation implementation; no CanISend serialized/YAML input reaches them |
| Unmaintained | `rustybuzz`, `ttf-parser` | Audited exception: only embedded, release-verified fonts are loaded; system/user font discovery is disabled |

These are narrow reachability exceptions, not a general severity waiver. They must be reviewed before R11.2 beta and
whenever the Typst stack changes. User-authored Typst, bibliography/CSL, YAML/XML decoding, external packages/files, or
user/system fonts are release blockers until the affected stack is upgraded or isolated in a preemptible bounded
worker. An advisory shown reachable by the existing fixed-template path is also an immediate release blocker.

The license policy permits only the explicitly listed SPDX licenses. Embedded font licenses and notices are outside
Cargo package metadata and are copied verbatim into every native bundle.

## Residual risks and required follow-up

- The Typst library does not expose a safe cancellation hook. The ten-second budget detects an overrun after compile or
  PDF export but cannot preempt it. The fixed-template/escaped-input boundary is therefore mandatory for this release.
- Filesystem checks and writes can still race with another local process running as the same user. Create-new files,
  symlink rejection, SQLite transactions, and immutable blobs reduce impact; CanISend is not a hostile multi-user
  filesystem service.
- PDF parsing is in-process. Input/page/text limits reduce exposure, but scheduled corpus/fuzz work remains required by
  the final release checklist.
- Windows ACL inspection is less expressive than Unix mode checks. Cross-platform restore and clean-machine tests are
  required before stable.
- Release signature and provenance verification are R11 gates; until then, locally built binaries are development
  artifacts.

At the R10.1 baseline there is no unresolved critical or high-severity finding reachable through a supported product
flow. Any change that weakens a mandatory invariant above reopens the security review.
