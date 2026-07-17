# Registered Stage Orchestration Example

`registered-parse.example.yaml` demonstrates the Stage 5 host-neutral orchestration contract. Replace the example
worker command with a local Codex, Claude, or custom command that accepts the configured prompt mode and writes
exactly one schema-valid Parse candidate JSON object to stdout.

Preview without creating a TaskSpec or launching a worker:

```bash
canisend orchestrate \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --plan examples/orchestration/registered-parse.example.yaml \
  --allow-private-sources \
  --dry-run
```

Run only after approving Tier 2 full-advert access:

```bash
canisend orchestrate \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --plan examples/orchestration/registered-parse.example.yaml \
  --allow-private-sources
```

The registered task must not add `inputs`, `outputs`, `writes`, or `edits_profile_input`. CanISend prepares the
immutable TaskSpec, supplies its declared reads/consents, and treats stdout only as candidate bytes. Validation,
terminal claim, and authoritative promotion remain core-service operations.

The placeholder command intentionally fails until replaced. It does not contact a provider or include credentials.
