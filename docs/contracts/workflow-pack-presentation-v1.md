# Workflow Pack presentation v1 contract

## Purpose

The Pack presentation read model is the read-only boundary between a verified Workflow Pack and
the desktop presentation layer. It resolves Pack-owned vocabulary, fields, categories, stages,
and Deliverables before the Svelte UI renders them. The shared UI must not maintain a second list
of academic stage labels or the four academic Deliverable kinds.

The first implementation resolves the checked-in `org.canisend.academic-job` Pack. Installed Pack
selection and generic Application creation remain owned by GF4-UI-001.

## Admission and authority

`Application::built_in_academic_pack_presentation` loads the embedded Pack through the normal
bounded verification path. The presentation operation never accepts an unchecked manifest, never
changes Workspace authority, and returns an `ActionReceipt` with operation
`workflow-pack.presentation`.

The returned Pack binding contains the exact Pack ID, semantic version, and content digest used to
produce the presentation. Callers must discard an older response when a newer locale request is in
flight.

## Locale selection

The desktop host requests `en` or `zh-CN`. Pack locale selection uses the shared localization
runtime:

- `en` selects the exact English locale;
- `zh-CN` selects the compatible `zh-Hans` locale;
- a missing selected translation falls back to the Pack default locale;
- every resolved label reports its source locale and whether default fallback was used;
- an unresolved required label fails the read operation instead of inventing UI text.

The desktop may display a stable local identifier only as a defensive fallback when presentation
metadata is unavailable. It must not substitute a hard-coded academic translation table.

## Read model

The response exposes:

- exact Pack binding and requested, selected, and match-kind locale metadata;
- selected Pack vocabulary;
- Opportunity and Application fields, including type, required state, choices, and resolved labels;
- Requirement and Evidence categories and their Pack-defined fields;
- topologically compiled stages with qualified IDs, dependencies, output, and execution modes;
- ordered Deliverables with qualified IDs, labels, and cardinality.

All IDs remain stable machine identifiers. Labels are presentation only and may change without
changing identity.

## Academic v2 compatibility

The read model may expose `legacy_task_operation` for a Deliverable only when the verified academic
Pack has an exact, explicit Agent v2 draft mapping. The desktop builds its draft-operation choices
from these mappings. A missing mapping produces no draft choice; the UI never guesses one from a
Deliverable label.

The legacy Opportunity creation form continues to map the academic Pack's single required
Opportunity field to the v2 `institution` boundary. This is a bounded academic compatibility path,
not a generic kernel requirement. GF4 replaces it with canonical v3 field submission.

## Desktop behavior

- locale changes request a fresh presentation and ignore superseded responses;
- stage cards and the selected-stage heading render Pack labels;
- document, render-preview, iframe-title, and live-region labels render Pack Deliverable labels;
- the Opportunity compatibility form renders its Pack field label and required state;
- task routing treats a declared draft operation as a delivery-material route without enumerating
  the academic four-document set;
- missing presentation data falls back to the stable identifier without preventing recovery.

## Verification

Focused Rust tests prove exact English selection, compatible Simplified Chinese selection, Pack
binding, compiled inventory, vocabulary, fields, categories, and bilingual labels. Frontend tests
prove typed command transport, custom stage and Deliverable labels, unknown-ID fallback, dynamic
draft choices, generic draft routing, and accessible final-PDF titles.
