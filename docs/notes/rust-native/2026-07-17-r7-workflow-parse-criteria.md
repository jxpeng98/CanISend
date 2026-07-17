# R7.1–R7.2 workflow, parse, and criteria implementation note

## Accepted scope

R7.1 and R7.2 establish the durable workflow kernel and the first reasoning-to-user-decision boundary. They do not
complete R7: evidence, matching, and the application plan remain in R7.3 through R7.5.

## Workflow kernel

The compiled `StageGraph` declares ten stages, exact dependencies, output kinds, and permitted execution modes. Graph
construction rejects duplicate stages, duplicate dependencies, missing dependencies, duplicate output producers, and
cycles. SQLite migration 5 stores the run's job revision plus each stage's status, execution mode, timestamps, and
output artifact reference.

`workflow start`, `status`, `begin`, `complete`, and `rerun` expose the kernel without returning private bodies.
Readiness, blockers, and next actions are derived from authoritative stage state. A job revision change resets parse,
stales only intake descendants, and preserves the independent evidence branch. A scoped rerun marks existing outputs
and leased descendant tasks stale rather than deleting history.

## Parse candidate boundary

The former synthetic single-criterion operation is replaced by `job.parse`. A task freezes the current job revision
and exact normalized source artifact revisions/hashes, claims the ready parse stage, and permits one ParsedJob output.
Host-agent and configured-provider modes use the same generated schema, Rust semantic validator, exact source-span
validator, task lease, stale check, and atomic commit path.

Configured-provider mode adds `send-to-configured-provider` to `read-private-inputs`. The CLI and store both refuse
provider-scoped input export unless the second consent is explicit. This mode does not make a model provider mandatory:
Codex, Claude, another host, or an optional provider adapter can produce the same bounded completion object.

## Source-bound criteria

Each proposed criterion records:

- the exact normalized source artifact identity, revision, and SHA-256;
- a half-open UTF-8 byte range;
- the verbatim quote selected by that range;
- confidence in thousandths;
- `confirmed = false` for agent/provider proposals.

Completion rejects out-of-scope sources, changed revisions or hashes, and spans whose bytes do not equal the quote.
The resulting ParsedJob artifact depends on every declared input revision.

`criteria proposed` shows the unconfirmed parse result. `criteria export` creates a new private editable JSON file
outside `.canisend/`. `criteria confirm` accepts user corrections only when every criterion is explicitly confirmed,
then revalidates schema, semantics, source scope, artifact revisions, spans, and quotes before atomically committing a
Criteria artifact and completing the user-decision stage. `criteria show` reads the confirmed artifact.

## Embedded host contract

The Codex, Claude, and generic packs now ship the job-parse prompt, ParsedJob, Criterion, and Criteria schemas, the
task descriptor/completion schemas, and a complete example. The packaged smoke follows the public CLI only: it
exports consent-scoped inputs, repairs an invalid candidate, commits and replays the parse, confirms criteria, reruns
parse, detects a source-change stale task, and finishes with `workspace check`.

## Verification

Local verification passed:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- 58 tests with `cargo test --workspace`;
- 23 generated public schemas and 29 embedded resources;
- `cargo build --release --locked`;
- packaged `scripts/smoke_host_agent.sh` using only the release binary.

GitHub Actions run `29620092129` first exposed a GNU `sed` portability error in the new smoke fixture. Commit
`12c5a42` replaced that expression with simple field extraction. Run `29620247965` then passed every gate in
1 minute 49 seconds.
