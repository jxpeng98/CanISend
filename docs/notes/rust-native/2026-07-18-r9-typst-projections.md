# R9.2 safe Typst projection note

## Outcome

Every supported structured document can now be exported as a self-contained editable `.typ` file alongside its
Markdown and JSON projections. A four-document package produces 13 managed files: 12 document projections and one
package manifest.

The new `typst-source` projection kind is part of the public projection, package-export-manifest, and reconciliation
contracts. It must reference a Cover Letter, Research Statement, Teaching Statement, or CV artifact and use a `.typ`
path beneath the exact job tree.

## Template and data projection

`template.application-document` is a typed embedded resource used by all four document kinds. Generated source
contains the complete template followed by a deterministic data dictionary with document identity, revision, kind,
title, section identity/body, claim/citation counts, and resolved field values. Source-artifact ID, revision, and hash
are preserved in non-executable comments.

The template uses only embedded Libertinus Serif, A4 layout, bounded styles, and the projected in-memory data. It does
not import a file or package. R9.3 will compile a fresh authoritative source generated from the structured document;
it will not trust an edited `.typ` projection as render input.

## Escaping and field gate

All user-originated strings are emitted as Typst quoted-string literals. The projector escapes quotes, backslashes,
newlines, carriage returns, tabs, and remaining control characters; Unicode remains UTF-8. Tests use text containing
a syntactically valid-looking `#read("/private/...")` expression, brackets, backslashes, and CJK text. All four
document kinds compile successfully, and PDF extraction recovers the expression as literal visible text.

Every document placeholder must have a resolution before Typst source generation. The projector returns only the
unresolved count, not the field key, instruction, or document body. Package readiness already blocks unresolved
required fields; this additional gate also prevents an optional unresolved template field from silently disappearing
from a rendered document.

## Editable projection boundary

Typst source participates in the same managed projection table as Markdown and JSON. The export receipt records its
exact source artifact, generated hash, observed hash, and edit state. `package reconcile` detects a changed `.typ`
file. `package replace` restores a newly generated source from the structured artifact, and `package copy-as-new`
remains available when the user wants to preserve an edit. All reconciliation receipts assert
`authoritative_changed: false`.

## Verification

Local acceptance passed:

- formatting and full-workspace Clippy with warnings denied;
- 72 Rust tests, including all document kinds, adversarial escaping, unresolved fields, real in-process compilation,
  PDF text recovery, package export, Typst edit detection, explicit restore, and upstream invalidation;
- 38 generated public-schema checks and 49 embedded-resource checks;
- locked release compilation to a 48,774,112-byte macOS arm64 binary;
- packaged host-agent smoke with `doctor` exercising the embedded compiler.

The preceding R9.1 checkpoint passed GitHub Actions run `29627072260` in 8 minutes 53 seconds. R9.3 can now add PDF
artifacts without inventing a second source-generation path.
