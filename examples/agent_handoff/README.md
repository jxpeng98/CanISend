# Host-Neutral Agent Handoff

This fake-data contract example demonstrates how two shell-capable hosts can resume the same CanISend application
workflow from durable workspace state. It does not copy a prompt, transcript, or provider session between hosts.

Host A and host B both request the current context with the same command:

```bash
canisend agent context \
  --workspace <workspace> \
  --job jobs/<job-id> \
  --format json
```

The response contains versioned protocol metadata, a safe job summary, a derived workflow snapshot, relative or
opaque artifact references, missing fields, consent requirements, blockers, and next actions. It does not contain the
full advert, profile, or package body.

`expected_capabilities.json` records the stable agent capability subset. `expected_context_shape.json` records the
required response shape and private body fields that must never appear. Dynamic values such as `request_id`, package
version, job ID, hashes, and derived readiness are intentionally not frozen in the fixtures.

The accepted shell contract now covers durable preparation, guarded candidate submission, application, cancellation,
promotion, recovery, and status for Parse, Confirm, Evidence, and Match. Evidence and Match are deterministic-only;
they do not need a platform API, MCP transport, hosted service, or configured provider. A fresh host resumes with the
same `agent context` and `stage status` commands, then may run:

```bash
canisend extract-profile-evidence --workspace <workspace>
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage evidence --mode deterministic --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage parse --mode deterministic --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage confirm --mode deterministic --format json
canisend stage run --workspace <workspace> --job jobs/<job-id> --stage match --mode deterministic --format json
```

The Evidence run snapshot, candidate, and catalog may duplicate private profile bodies and are retained until the user
removes the run or job. Agent context, workflow control records, and Match output expose only privacy-safe artifact
references, hashes, IDs, reason codes, and counts. Match classifications remain `review_state=proposed`; this example
does not claim that Decision, Brief, or required-document planning is implemented.
