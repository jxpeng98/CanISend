# Generic Application synthetic examples

CanISend ships five offline examples for the built-in generic Application Pack:

| Family | Fictional scenario | Deliverables |
|---|---|---|
| Grant | Neighborhood resilience pilot | Primary narrative and budget assumptions note |
| Admission | Public-interest design programme | Admission statement and portfolio note |
| Tender/proposal | Offline records-classification prototype | Proposal and compliance note |
| Professional job | Operations coordinator | Application document and evidence note |
| Internal dossier | Governance planning exercise | Governance dossier and supporting chronology |

All names, identifiers, dates, amounts, histories, and bodies are synthetic test data. The files
must never be populated from a real person, organization, application, advert, tender, or grant.

The examples are embedded resources under `examples/generic-v4/`. `canisend resource list --json`
shows their IDs, versions, sizes, and SHA-256 digests. Complete catalog export through the shared
App/desktop resource export copies the verified JSON files together with its integrity manifest.

## Run the offline qualification

From a source checkout:

```console
cargo test -p canisend-app --test generic_examples --locked
```

The test creates five temporary domain-neutral Workspace v4 fixtures and executes the real generic
flow through PDF export. It performs no network request, provider call, host launch, upload, or
submission. Every temporary Workspace is removed after validation.

Resource-only verification is available with:

```console
cargo test -p canisend-resources --test manifest \
  five_fictional_generic_application_examples_are_embedded_and_offline --locked
```

The authoritative fixture rules are in the
[Generic Application example fixture v1 contract](../contracts/generic-application-example-v1.md).
