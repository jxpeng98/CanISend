# Generic Application example fixture v1

**Format:** `canisend.generic-application-example/v1`

**Status:** Internal, versioned, offline fixture format implemented by GF4-EXAMPLE-001. It is not
a Workflow Pack, public submission format, provider payload, or promise of long-term SDK
compatibility.

## Purpose

The fixture proves that `org.canisend.generic-application` is not coupled to academic work. Four
embedded, fully fictional examples exercise grant, admission, tender/proposal, and professional
job Applications through the same Pack and canonical flow.

Each fixture contains:

- a unique synthetic scenario and family ID;
- the exact generic Pack ID;
- typed opportunity and Application metadata;
- a reviewed UTF-8 source body;
- Requirement statements that occur exactly once in that source;
- a user decision and Pack-qualified Plan;
- one primary and one supporting Deliverable; and
- a safe export slug.

The test runner derives exact UTF-8 byte spans from the unique Requirement excerpts. Application
IDs, revisions, snapshot digests, Blob identities, projection paths, and export paths are always
created by the real runtime; fixtures never forge durable state.

## Synthetic-data boundary

Every fixture must set:

```json
{
  "synthetic": true,
  "data_policy": "fictional-only-no-real-personal-data"
}
```

It must also carry the exact reviewed synthetic notice. Organization names begin with
`Fictional`, titles begin with `Synthetic`, identifiers begin with `SYN-`, and bodies explicitly
deny real-world identity or authority. Fixtures contain no URL, email address, `.canisend` path,
credential, contact detail, live system, or network instruction.

These structural checks supplement human review; they do not make arbitrary data anonymous.
Contributors must never replace these fixtures with redacted or pseudonymized real Applications.

## Offline executable validation

For every embedded fixture the integration suite:

1. verifies the resource digest and version;
2. deserializes with unknown fields denied;
3. checks the synthetic-data policy and unique family/scenario IDs;
4. initializes a fresh generic Workspace;
5. creates the Application with exact Requirement spans;
6. confirms the Plan and composes both Deliverable kinds;
7. privately reads and byte-compares every reviewed body;
8. records explicit approval;
9. creates the managed projection and renders both PDFs;
10. validates PDF bytes, manifest counts, stage completion, and Workspace integrity; and
11. asserts `submission_performed: false`.

The runner uses no URL adapter, provider, host process, network fixture, or external renderer.

## Embedded resource IDs

- `example.generic-v3.grant`
- `example.generic-v3.admission`
- `example.generic-v3.tender-proposal`
- `example.generic-v3.professional-job`

They participate in the ordinary resource manifest, SHA-256 verification, catalog listing, and
complete public-catalog export.
