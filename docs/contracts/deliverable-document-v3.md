# Structured Deliverable content v3

Media type: `application/vnd.canisend.deliverable+json`.
Format: `canisend.deliverable-document/v3`.
Schema resource: `schema.v3.deliverable-document`.

The existing draft/revision request keeps `kind`, `title`, `media_type` and `content`.
For this media type, `content` is a JSON string containing the document below. The
immutable Blob digest binds fields, sections, classifications and citations together.
No database migration or implicit profile extraction is involved.

```json
{
  "format": "canisend.deliverable-document/v3",
  "fields": [],
  "sections": [{
    "id": "future-work",
    "heading": "Future work",
    "body": {
      "text": "I intend to develop open teaching resources.",
      "role": "intent",
      "evidence": [],
      "requirements": []
    }
  }],
  "unresolved_fields": []
}
```

Fields have a unique Pack-local `key` and a `value` with the same text structure as
`body`. Sections have unique Pack-local IDs and preserve their supplied order.
Headings and titles are literal captions; they remain subject to editorial review.
Text is literal Unicode, including paragraph breaks. Typst expressions and Markdown
markers in text never become executable markup. Put headings in the heading field.

Each field/body declares one role:

- `evidence-bound`: at least one exact confirmed Evidence citation, with `id`,
  `revision` and `sha256`; `requirements` is empty.
- `requirement-bound`: at least one exact confirmed Requirement citation with `id`
  and `revision`; `evidence` is empty.
- `intent` or `non-factual`: both citation arrays are empty. These labels require
  actual review and cannot be used to disguise unsupported facts.

The kernel checks 4 MiB input, at most 64 fields, 128 nonempty sections, 64 unresolved
field keys, 32 citations of each type per value, 64 KiB per value and 256 KiB total
value text. Captions are bounded to 512 bytes. Unknown keys, duplicate field/section
IDs, unsupported controls and malformed references fail without echoing private text.
Drafts may contain missing citations and unresolved fields so the user can correct
them; approval and export cannot waive readiness failures.

## Readiness and authority

Composition checks the complete Required Plan set independently of Pack counts.
Approval and export repeat that check against the current confirmed Plan and exact
Deliverable revisions. The bound Pack's per-kind validators then execute:

- `evidence-traceability`: cited revisions must be currently associated, confirmed,
  non-excluded Evidence and appear in that Deliverable's exact used inputs.
- `unsupported-claims`: the declared role must have the appropriate citations.
- `citation-integrity`: Evidence digests/revisions and confirmed Requirement revisions
  must match; Evidence catalog bytes are verified from their immutable provenance.
- `placeholder-free`: unresolved keys and case-insensitive `[TODO`, `[TBD` or
  `[PLACEHOLDER` markers in values/captions block readiness.
- `review-complete`: approval requires ReviewRequired state; export requires Approved
  state obtained from the authorized review commit. Content revision clears approval.

Unknown capabilities or unsupported parameters fail closed. Placeholder-free accepts
one optional `required_evidence_fields` array of bounded field keys. Academic Pack
1.0.3 uses it for `candidate-name` and `email`, both with EvidenceBound values.
Domain field names belong to the Pack/template, not to kernel branching.

Machine validation proves declared provenance integrity and state, not natural-language
entailment or completeness of extracted claims. Review must compare every assertion,
classification, caption and contact with its sources. Source association alone does
not confirm Evidence or authorize extracting unrelated fields.

## Projection and compatibility

The embedded renderer projects reviewed fields and ordered sections through the exact
Pack template using escaped Typst strings. It has no additional file/network/package
authority. Managed editable content uses `.json`; PDFs and render manifests retain
their existing revision/hash bindings. Export rechecks readiness after rendering and
before publishing files, including Evidence changes during rendering.

Historical `text/plain` and `text/markdown` remain readable and revisable. Markdown is
literal. A Pack declaring readiness validators requires explicit structured revision
before approval or export; legacy text is not silently labeled non-factual. Historical
Pack bundles and previously exported artifacts remain bound to their original bytes.

The five bundled generic examples are fictional format/lifecycle demonstrations,
explicitly labeled non-factual. They do not establish evidence-grounded real-user
acceptance. The academic integration regression uses an imported fictional profile,
source-span confirmation, explicit associations, structured identity and four PDFs.
