# Packaged host-agent smoke transcript

This transcript is implemented by `scripts/smoke_host_agent.sh`. Every state transition goes through the packaged
`canisend` binary; the harness never reads or writes SQLite, blobs, or any other `.canisend/` path.

```text
canisend agent capabilities --json
  -> task.lifecycle = available

canisend agent assets export --host codex --destination <external>/codex-pack --json
  -> AGENTS.md, prompt, example, schemas, and canisend-agent-pack.json

canisend --workspace <workspace> workspace init --json
canisend --workspace <workspace> job create ... --json
canisend --workspace <workspace> job import JOB_ID --file job-advert.md --json
canisend --workspace <workspace> task prepare --job JOB_ID --operation job-parse --json
  -> exact job revision, artifact revisions/hashes, consent request, lease, and candidate schema

canisend --workspace <workspace> task inputs TASK_ID --destination <external>/inputs --json
  -> exit 3, consent.required, no destination created

canisend --workspace <workspace> task inputs TASK_ID --destination <external>/inputs \
  --allow-private-read --json
  -> only declared inputs plus a versioned integrity manifest

<host writes invalid canisend.task-completion/v2 under <external>>
canisend --workspace <workspace> task complete --file invalid-completion.json --json
  -> exit 3, candidate.semantic_invalid, JSON-pointer details, remediation; task remains prepared

<host writes valid canisend.task-completion/v2 under <external>>
canisend --workspace <workspace> task complete --file completion.json --json
  -> committed artifact, idempotent = false
canisend --workspace <workspace> task complete --file completion.json --json
  -> same artifact, idempotent = true

canisend --workspace <workspace> criteria export --job JOB_ID --destination criteria.json --json
  -> editable source-bound proposal with `confirmed = true` as the pending user decision
<user reviews or corrects criteria.json>
canisend --workspace <workspace> criteria confirm --job JOB_ID --file criteria.json --json
  -> exact source spans are revalidated and a confirmed Criteria artifact is committed

canisend --workspace <workspace> workflow rerun --job JOB_ID --stage parse --json
canisend --workspace <workspace> task prepare ... --json
<source is re-imported through canisend, changing the job revision>
canisend --workspace <workspace> task complete --file stale-completion.json --json
  -> exit 4, task.stale, prepare-again remediation

canisend --workspace <workspace> workspace check --json
  -> ok = true
```

Candidate and exported input files live outside internal state. The only component authorized to mutate internal
state is the CanISend binary itself.
