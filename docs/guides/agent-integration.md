# Agent integration

CanISend is designed to run beside Codex, Claude, or another agent host. The native binary owns validation,
revisioning, privacy scopes, workflow state, storage, and rendering. The host agent owns conversation and bounded
semantic reasoning.

## Export a self-contained host pack

```console
canisend agent assets export --host codex --destination ./canisend-codex-pack
canisend agent assets export --host claude --destination ./canisend-claude-pack
canisend agent assets export --host generic --destination ./canisend-generic-pack
```

Each pack includes host instructions, operation prompts, public schemas, examples, and an integrity manifest. It is
versioned for `canisend.agent/v2` and does not depend on source-repository files after export. Give the selected pack
to the host according to that platform's local instruction mechanism.

## Discover current state

An agent should never infer capabilities from prose alone:

```console
canisend agent capabilities --json
canisend --workspace ./applications agent context --job JOB_ID --json
canisend --workspace ./applications workflow status --job JOB_ID --json
```

Treat only `available` capabilities as executable. Context is intentionally body-free; it tells the host which job,
stage, blockers, task modes, consents, and next actions exist without disclosing private text.

## Bounded task loop

1. Prepare a task with the operation named by workflow status.
2. Inspect the returned descriptor and consent requests.
3. After user approval, export only declared inputs.
4. Ask the host to produce JSON matching the descriptor's output schema and embedded prompt.
5. Complete the task with the same task ID, lease ID, expected job revision, and input revisions.
6. Follow structured validation remediation or prepare a new task if stale.

```console
canisend --workspace ./applications task prepare \
  --job JOB_ID --operation job-parse --mode host-agent --json

canisend --workspace ./applications task inputs TASK_ID \
  --destination ./agent-work/TASK_ID \
  --allow-private-read --json

canisend --workspace ./applications task complete \
  --file ./agent-work/TASK_ID/completion.json --json
```

Candidate validation is schema-first and semantic. An invalid candidate leaves the lease prepared and returns stable
violation codes plus JSON pointers. Replaying the identical accepted candidate is idempotent. If a source/profile
revision or lease changes, completion returns `task.stale`; prepare again and do not reuse the old candidate.

## User-only decisions

An agent may propose evidence, matches, drafts, and semantic review findings. The user remains responsible for:

- confirming or correcting job criteria;
- confirming, excluding, or revising profile evidence;
- choosing apply, hold, or skip and confirming the plan;
- resolving required placeholders and review dispositions;
- consenting to private read/provider send/local export;
- checking the final package and submitting outside CanISend.

No command, capability, or host pack authorizes application submission.

## Protocol behavior

Use `--json` or capture stdout to receive exactly one versioned envelope. Diagnostics never share stdout with the
JSON object. Exit classes are stable: 0 success, 2 CLI usage, 3 validation/consent, 4 state conflict/stale, 5 external
I/O/provider failure, and 6 internal invariant failure. See [agent protocol v2](../contracts/agent-protocol-v2.md)
for the complete fields and error registry.
