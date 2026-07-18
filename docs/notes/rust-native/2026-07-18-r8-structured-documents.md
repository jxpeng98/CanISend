# R8.1 structured document contract note

## Authority boundary

R8 drafting uses two contracts. `canisend.document-candidate/v2` is the host-agent/configured-provider output and has
no document, section, claim, or placeholder identity fields. `canisend.document/v2` is the durable core output;
CanISend adds stable UUIDv7 identities, revisions, the exact generating task, executor, prompt resource, actor, and
timestamp.

Both contracts bind the document to the exact current ApplicationPlan artifact and exact planned-document identity
and revision. R8.2 must validate these references inside the task completion transaction before assigning identities.

## Structured content model

A document contains bounded ordered sections. Each section has a typed purpose, optional heading, body, and explicit
claims. The first alpha supports all four kinds already frozen by R7.5:

- Cover Letter;
- CV;
- Research Statement;
- Teaching Statement.

Cover Letters require exactly one opening and closing. Research and Teaching Statements require their corresponding
section. A CV requires at least one education, experience, or publications section. Individual text fields, section/
claim counts, total body bytes, citations, and placeholders are bounded before any state mutation.

## Claim and citation policy

Every claim declares one of four classifications:

- `applicant-fact` requires one or more exact Evidence item revisions and permits only evidence citations;
- `job-requirement` requires one or more exact Criterion revisions and permits only criterion citations;
- `user-intent` is an explicit non-evidence statement and cannot carry a citation;
- `non-factual` is an explicit rhetorical/structural statement and cannot carry a citation.

This makes missing support structurally visible without pretending semantic validation alone can determine whether an
agent deliberately misclassified prose. R8.2 will validate cited identities against the task's exact EvidenceMatches
inputs; R8.3 will independently detect undeclared/unsupported prose and prohibited-claim conflicts.

Citations include a bounded purpose explaining why the exact target supports the claim. Duplicate target revisions
within one claim are rejected.

## Placeholder and generation policy

Placeholders use unique, portable lowercase keys and retain an instruction, required flag, and optional resolution.
An unresolved required placeholder is valid draft state, not package readiness; R8.3/R8.4 will surface it as a review
or readiness blocker.

Generation metadata accepts only matching host-agent/host-agent or configured-provider/configured-provider actor and
mode pairs. An agent cannot submit this metadata because it exists only in the durable output contract.

## Executable fixture and resources

The synthetic Cover Letter fixture now satisfies the generated candidate schema and semantic validator with exact
Evidence citations and explicit user-intent claims. The candidate and durable schemas are embedded in the binary and
the self-contained Codex, Claude, and generic host packs.

Local acceptance passed 64 Rust tests, Clippy with warnings denied, 30 deterministic schema checks, 38 embedded
resource checks, release compilation, and the packaged intake-to-plan smoke with the expanded host pack. R8.2 can
now add bounded draft tasks without inventing a second content model.
