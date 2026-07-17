# Synthetic Rust-Native CLI Transcript

This transcript defines the intended v2 product flow. IDs and hashes returned at runtime are captured into shell
variables by a host; literal fixture IDs below describe relationships rather than a compatibility promise.

## Initialize and import

```text
canisend workspace init --workspace ./demo --json
canisend profile evidence import --workspace ./demo --file profile-evidence.json --json
canisend job create --workspace ./demo --title "Lecturer in Economics" --institution "Northbridge University" --json
canisend job import --workspace ./demo --job <job-id> --file job-advert.md --json
```

Every command emits exactly one `canisend.agent/v2` envelope on stdout. Progress and diagnostics use stderr.

## Prepare and complete parsing

```text
canisend task prepare --workspace ./demo --job <job-id> --stage parse --mode host-agent --json
canisend task complete --workspace ./demo --task <task-id> --candidate expected-criteria.json --json
```

The completion command validates the candidate and commits it atomically. There is no separate compatibility
`submit`/`apply` sequence.

## Match and plan

```text
canisend workflow run --workspace ./demo --job <job-id> --until plan --json
canisend workflow status --workspace ./demo --job <job-id> --json
```

Deterministic stages may complete locally. A stage that needs an agent or user decision returns a blocker and a next
action without changing unrelated state.

## Draft and review

```text
canisend task prepare --workspace ./demo --job <job-id> --stage draft --document cover-letter --mode host-agent --json
canisend task complete --workspace ./demo --task <task-id> --candidate cover-letter-candidate.json --json
canisend workflow run --workspace ./demo --job <job-id> --until package --json
canisend package check --workspace ./demo --job <job-id> --json
```

The package remains human-review-required while warning/review findings have no recorded disposition or required
manual documents are missing.

## Export and render

```text
canisend package export --workspace ./demo --job <job-id> --format markdown --json
canisend render --workspace ./demo --job <job-id> --format pdf --json
```

The PDF render uses embedded Typst libraries, templates, and default fonts. It does not invoke Python, Node, Java, or
an external `typst` executable.
