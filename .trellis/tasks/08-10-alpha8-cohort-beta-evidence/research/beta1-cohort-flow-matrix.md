# Beta.1 cohort flow matrix

## Status and boundary

This is a planning-only, body-free execution template for Issue #70. It does not authorize
participant contact, provider sends, global host changes, private-body access, or data retention.
`P01`–`P08` are empty cohort slots, never participant identities. Any identity-to-slot mapping
stays outside the repository and requires consent.

All attempts use exact public `v1.0.0-beta.1` from source
`6e1397b79031cad54e794ccdc9edca2153f23b3e`. Generic family labels come from the existing offline
fixture inventory, but consented participants use their own reviewed local material; synthetic
fixtures do not become user evidence.

## Counting rules

- A supported flow is complete only when its stated positive or expected-refusal outcome occurs.
- An expected refusal counts as complete only when authority is unchanged and the receipt reports
  the intended boundary.
- Every supported attempt that starts installation or an existing supported installation enters
  the unassisted denominator. Later success never removes an earlier product failure.
- Exclude only participant withdrawal or documented external-host outage.
- Maintainer operation of the App, CLI, Codex, or Workspace after an attempt begins makes that
  attempt assisted. Pre-session written instructions and observation do not.
- Per-attempt notes are reduced to aggregates, coverage tokens, exclusions, and minimum-safe Issue
  numbers; no participant mapping or Application body is committed.

## Minimum matrix

| Flow | Slot | Surface / locale | Pack / family | Intake and Deliverables | Required outcome |
|---|---|---|---|---|---|
| C01 | P01 | App / en | Generic / professional-job | pasted text; `primary-document` + `supporting-document` | Clean App initialization, project-scoped Codex setup, keyboard-only guarded compose/review/export, `submission_performed: false` |
| C02 | P01 | App + Codex / en | Academic / research-fellowship | URL; `cover-letter` + `cv` + `research-statement` | Create in the C01 Workspace, complete guarded review/export, preserve both exact Pack bindings |
| C03 | P01 | CLI + App / en | mixed Workspace | C01–C02 authority | Back up, restore to a new path, check both Applications, reopen App, and reconcile receipts |
| C04 | P02 | CLI + Codex / zh-Hans | Generic / grant | local text file; `primary-document` + `supporting-document` | Clean CLI-only initialization and project-scoped Codex guarded compose/review/export |
| C05 | P02 | headless Codex + App / zh-Hans | Academic / teaching-focused | text PDF; `cover-letter` + `cv` + `teaching-statement` | Operate with App closed, reopen, and reconcile unchanged Pack/revision/receipt identity |
| C06 | P02 | MCP / zh-Hans | mixed Workspace | C04–C05 authority | Cross-Application Profile/Evidence association is refused without mutation |
| C07 | P03 | App + Codex / en | Generic / tender-proposal | URL; `primary-document` + `supporting-document` | VoiceOver guarded compose/review/export completes with traceable claims and no submission |
| C08 | P03 | App + Codex / en | Academic / research-fellowship | pasted text; `cover-letter` + `cv` | Intentional Evidence gap and unsupported claim block readiness/export without mutation |
| C09 | P03 | App + Codex / en | Generic / professional-job | local text file; `primary-document` | Add reviewed Evidence after a gap, then complete guarded review/export without unsupported claims |
| C10 | P04 | App + Codex / zh-Hans | Generic / professional-job | local text file; `primary-document` + `supporting-document` | Keyboard-only flow at 200% text scale completes without clipped or unreachable blocking control |
| C11 | P04 | App + Codex / zh-Hans | Academic / teaching-focused | URL; `cover-letter` + `cv` + `teaching-statement` | Complete in the C10 Workspace and preserve independent Application associations |
| C12 | P04 | CLI / zh-Hans | unsupported legacy boundary | bounded legacy Workspace and old-Skill fixtures | Both inputs are refused before mutation and direct the user to clean Workspace v4 setup |
| C13 | P05 | CLI + Codex / en | Generic / grant | text PDF; `primary-document` + `supporting-document` | Explicitly consented global Codex setup/status, guarded review/export, then managed host-resource removal |
| C14 | P05 | App + Codex / en | Academic / research-fellowship | local text file; `cover-letter` + `cv` + `research-statement` | VoiceOver flow completes in the same Workspace with exact Pack isolation |
| C15 | P06 | App + Codex / zh-Hans | Generic / tender-proposal | pasted text; `primary-document` + `supporting-document` | Guarded review/export completes at 200% scale with body-free receipts |
| C16 | P06 | headless Codex + App / zh-Hans | Academic / teaching-focused | text PDF; `cover-letter` + `cv` + `teaching-statement` | App-closed guarded operation and reopen reconciliation complete without submission |
| C17 | P07 | CLI + Codex / en | Academic / research-fellowship | URL; `cover-letter` + `cv` + `research-statement` | Project-scoped Codex guarded review/export completes from CLI-initialized Workspace |
| C18 | P07 | App + Codex / en | Generic / professional-job | local text file; `primary-document` | Wrong-Application Evidence association is refused, correct association is approved, and export completes |
| C19 | P08 | App + Codex / zh-Hans | Generic / grant | URL; `primary-document` + `supporting-document` | Chinese keyboard and VoiceOver flow completes with explicit consent and no submission |
| C20 | P08 | CLI + Codex / zh-Hans | Generic / tender-proposal | local text file; `primary-document` | Guarded review/export and final Workspace check complete with body-free receipts |

One eight-slot window is the shortest schedule. If only five to seven consented users are
available initially, a second bounded window fills the remaining slots; it does not replace or
discard earlier eligible attempts.

## Aggregate worksheet

The checked-in cohort record derives only these totals:

| Metric | Numerator | Denominator / rule |
|---|---|---|
| Unassisted completion | eligible attempts reaching their stated outcome without maintainer operation | every eligible started attempt, except withdrawal or documented host outage |
| Audited claim traceability | audited factual claims bound to confirmed Evidence | all audited factual claims |
| Backup/restore success | measured mixed-Workspace restores that pass integrity and reopen checks | all measured mixed-Workspace restore attempts |
| Unsupported audited claims | count of unsupported claims surviving audit | must equal zero |
| No-submission understanding | participants confirming Exported is not Submitted | every cumulative participant |

The minimum passing aggregate is at least 8 cumulative users, at least 20 completed flows,
unassisted completion at least 80%, 100% audited-claim traceability, 100% measured backup/restore,
zero unsupported audited claims, 100% no-submission understanding, and zero unresolved P0/P1
cohort blockers.
