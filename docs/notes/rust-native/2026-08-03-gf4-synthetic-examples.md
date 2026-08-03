# GF4-EXAMPLE-001 — Offline synthetic generic examples

Date: 2026-08-03

## Outcome

CanISend now ships four embedded, fully fictional `canisend.generic-application-example/v1`
fixtures for grant, admission, tender/proposal, and professional job Applications. All four run
through the same exact `org.canisend.generic-application` Pack and canonical Application flow.

The examples are executable qualification inputs, not illustrative JSON that can drift from the
runtime. Their test runner derives exact UTF-8 Requirement spans, creates real revisioned state,
stores immutable content Blobs, confirms Plans, composes both Deliverable kinds, reads the exact
private bodies, records approval, builds managed projections, renders PDFs, verifies export
manifests, and runs Workspace integrity checks.

## Synthetic-data boundary

- Every file declares `synthetic: true` and
  `data_policy: fictional-only-no-real-personal-data`.
- Organizations are explicitly fictional, scenario and tracking IDs use `SYN-`, and titles are
  explicitly synthetic.
- The files contain no URL, email address, contact detail, credential, live system, `.canisend`
  path, or network instruction.
- Bodies explicitly avoid real biography, employment history, organization relationships,
  procurement authority, live data access, or submission authority.
- Contributors are instructed never to derive a fixture from redacted or pseudonymized real data.

## Resource delivery

The four JSON files are ordinary `ResourceKind::Example` entries. Build-time generation gives
them typed IDs, immutable sizes and SHA-256 digests, and inclusion in verification, listing, and
complete public-catalog export:

- `example.generic-v3.grant`;
- `example.generic-v3.admission`;
- `example.generic-v3.tender-proposal`; and
- `example.generic-v3.professional-job`.

## Verification evidence

- `cargo test -p canisend-app --test generic_examples --locked`
  - 1 scenario-matrix test passed;
  - 4 fresh Workspaces completed;
  - 8 Deliverables were privately compared, approved, rendered, and PDF-validated;
  - every terminal stage completed with `submission_performed: false`.
- `cargo test -p canisend-resources --test manifest
  four_fictional_generic_application_examples_are_embedded_and_offline --locked`
  - resource identity, version, format, family, synthetic policy, exact Pack, and offline markers
    passed for all four resources.

## Remaining boundary

GF4 source implementation is now complete. GF5 must register and verify canonical cross-surface
operations, run the two-Pack semantic parity matrix, and replace the pre-generic quick start,
Agent, desktop, privacy, backup, upgrade, and limitations documentation. The broader Alpha gate
also retains approval-broker, operation-registry, native qualification, governance, CI, and real
user/dogfood evidence requirements from the active 1.0 roadmap.
