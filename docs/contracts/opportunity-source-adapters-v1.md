# CanISend Opportunity-source adapter contract v1

**Status:** Implemented for the built-in academic reference Pack

**Pack authority:** verified `canisend.workflow-pack/v1` capability declarations

**Host authority:** the compiled `canisend-io` discovery adapter registry

## Boundary

An Opportunity-source adapter is kernel-owned executable behavior selected by declarative Pack
data. A Pack may reference a registered capability ID, but cannot supply parser code, destination
rules, credentials, network policy, persistence logic, or promotion behavior. An adapter is
eligible only when both conditions are true:

1. the exact verified Pack Manifest declares the capability in `capabilities.intake_adapters`; and
2. the host registry maps that capability to one compiled `DiscoverySourceKind` and its bounded
   capability descriptor.

Unknown capabilities fail during Pack verification. Registered but undeclared capabilities are
absent from the Pack-qualified catalog and fail before DNS, HTTP, or Workspace access during both
preview and commit.

## Built-in registrations

| Pack capability | Host source kind | Destination policy |
|---|---|---|
| `canisend.discovery.rss-atom` | `rss-atom` | user-selected absolute HTTP(S) endpoint plus the shared public-destination policy |
| `canisend.discovery.jobs-ac-uk` | `jobs-ac-uk` | HTTPS on `jobs.ac.uk` or `www.jobs.ac.uk` only |
| `canisend.discovery.greenhouse` | `greenhouse` | HTTPS on `boards-api.greenhouse.io` with `/v1/boards/{token}/jobs` shape |
| `canisend.discovery.lever` | `lever` | HTTPS on `api.lever.co` or `api.eu.lever.co`, `/v0/postings/{site}`, and `mode=json` |

Every registration is network-only, supports a payload-derived cursor, preserves missing prior
records as history, and declares a maximum of 1,000 accepted items per refresh. Registration does
not grant consent or initiate a request.

## Pack-qualified operation

1. Load and verify the exact Pack snapshot, including its capability references.
2. Build the catalog by intersecting Pack declarations with the compiled adapter registry.
3. Bind the catalog response to Pack ID, version, and content digest.
4. On preview, map the requested source kind to one capability and require its Pack declaration
   before endpoint parsing, DNS resolution, or HTTP.
5. Require explicit network consent, apply the shared public-destination/redirect policy, cap the
   response at 4 MiB, verify content type, and parse at most 1,000 leads.
6. Produce a normalized dry-run report with source kind/name/URL, observation time, payload cursor,
   accepted candidates, and body-bounded diagnostics.
7. On commit, derive the capability again from the reviewed report and repeat Pack eligibility
   before opening the Workspace.
8. Store source identity, endpoint, refresh policy, cursor, timestamps, per-lead source digest,
   refresh receipt, and audit event transactionally.
9. Promotion remains an explicit user action. It preserves the discovery source link and is
   idempotent; adapters never submit or create external state.

The existing Agent v2, CLI `job`, and desktop discovery surfaces resolve the exact built-in
`org.canisend.academic-job` Pack as their compatibility context. Canonical generic Pack selection
and v3 cross-surface routing remain separate roadmap work.

## Failure and replay rules

- A Pack capability mismatch is a permanent input error and performs no network or Workspace
  access.
- Insecure provider URLs, cross-provider hosts, embedded credentials, private destinations,
  redirect escape/downgrade, misleading content, and oversized payloads fail closed.
- A commit accepts only an uncommitted normalized report whose source kind is a registered network
  adapter and still declared by the supplied exact Pack.
- Refresh upserts by stable source/candidate identity, records changed and removed leads, retains
  history, and never automatically merges fuzzy suggestions.
- Promotion replay returns the original promoted Job in the bounded v2 academic compatibility
  path rather than creating a duplicate.

## Verification evidence

- `registered_capabilities_have_one_exact_network_adapter_mapping`
- `provider_destination_policy_rejects_insecure_cross_host_and_wrong_shape_urls`
- `provider_payload_limit_matches_the_registered_catalog_limit`
- `pack_declaration_filters_catalog_and_fails_before_network_or_workspace_access`
- `refresh_is_deterministic_preserves_history_and_promotes_idempotently`
- existing offline jobs.ac.uk, RSS/Atom, Greenhouse, and Lever normalization fixtures
