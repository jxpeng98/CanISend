# Output, Diagnostics, and Privacy

> How CanISend reports progress without collecting user content.

---

## Overview

CanISend has no default telemetry, analytics, crash upload, or background reporting. It does not
use a general runtime logging framework. CLI and `xtask` produce bounded deterministic output;
release and dogfood records are body-free.

## Output Channels

- Standard output is for requested machine/user results.
- Standard error is for actionable failures and warnings.
- Do not add debug prints, hidden network reporting, or a new logging dependency for convenience.

## Structured Records

Machine-readable records use versioned JSON schemas and bounded identifiers. Human diagnostics
name the operation and remediation without copying private bodies. Examples include
`crates/canisend-cli/src/lib.rs` and `xtask/src/main.rs`.

## What to Record

- Version/stage/digest facts owned by release tooling.
- Body-free operation outcomes, error codes, and explicit remediation.
- Consent and approval receipts by identifiers and revisions, not document content.

## What Not to Record

Never record Profile, advert, application, Evidence, Deliverable, proposal, or transcript bodies;
provider credentials/tokens; private Workspace paths in public evidence; or cross-Application
content. Refer to `docs/release/support-policy.md` and the active Roadmap evidence rules.
