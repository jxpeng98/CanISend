# R2a bound-server source evidence

Date: 2026-09-04
Baseline: `d7c0abfea46aa8d7b9f63d9b1ac7bd7fac93abea` plus uncommitted source changes.
Status: Partial R2a implementation; no new artifact, protected CI, or provider qualification.

## Implemented boundary

- Optional CLI `mcp serve --application APPLICATION_ID` validates an existing Application at startup.
  The original unbound CLI and Rust entry points remain compatible.
- All 32 Application-ID handlers use one server-instance guard before App facade/broker work.
  The four no-ID tools and both association-list tools reject Workspace-wide results in bound mode.
  The tool catalog remains 36. No consent token or user authority is minted by this binding.
- Thread start/resume explicitly select the user reviewer with approval policy `never`; every turn
  reasserts those fields and the named permission profile (see runtime integration below). This is not
  proof of effective process isolation or a working consent loop.
- Existing App Server fixtures cover body-free MCP item normalization and rejection of unqualified
  user-input, elicitation, and permission requests on both fresh and resumed sessions. Embedded
  CanISend MCP injection and private tools remain disabled.

## Exact local provider/schema observations

The inspected executable reports `codex-cli 0.152.0`. Generated schemas expose `approvalsReviewer`
for thread start/resume and turn start, including `user`. The read-only sandbox variant exposes
`networkAccess` but no restricted-read root list. This does not establish that every alternative
isolation mechanism is unavailable; it means the current read-only settings are insufficient proof.

The request catalog includes `item/tool/requestUserInput`, `mcpServer/elicitation/request`, and
`item/permissions/requestApproval`. A schema or synthetic request does not prove that actual MCP
calls require a user response, that required-server startup fails closed, or that approval can be
safely bound to the selected Application, tool, payload, and current turn.

At the 2026-09-04 baseline, no signed-in provider smoke, credential access/copy, global configuration
edit, or unrelated MCP startup was performed. Current fixtures assert denial rather than treating arbitrary questions or
model-provided booleans as user consent.

## Source checks — 2026-09-04

- `cargo test -p canisend-cli --locked --test mcp_protocol`: four existing tests passed. The new
  binding test initially exposed a missing academic fixture field and an incorrect test expectation
  for the JSON-RPC error envelope; both fixture/assertion issues were corrected.
- `cargo test -p canisend-cli --locked --test mcp_protocol application_binding_covers_every_tool_and_preserves_unbound_discovery`:
  passed after those corrections. Covers all 36 tools, both Generic/Academic binding directions,
  selected metadata reads, disabled enumeration, unchanged unbound discovery, invalid startup, and
  unchanged Workspace state. Guard errors use the top-level JSON-RPC invalid-params envelope.
- `cargo test -p canisend-gui --locked --features app-server-test-fixture --test app_server_protocol`:
  two passed, one existing signed-in smoke ignored. Fresh/resume policy consistency and body-free
  rejection checks passed; no real provider success is inferred.
- `cargo clippy -p canisend-cli -p canisend-mcp -p canisend-gui --all-targets --features canisend-gui/app-server-test-fixture --locked -- -D warnings`:
  passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- `cargo run -p xtask --locked -- release check`: passed. Existing release status still reports
  four drift items, three stage-blocking; 14 recorded freeze exceptions are unchanged. Local source
  validation does not qualify a new release or replace protected CI.
- Markdown links/anchors and retained JSON/JSONL: checked with the scoped local document checker.

## Real App Server probe — 2026-09-05

The opt-in [probe](../../../../scripts/probe_codex_app_server.py) uses Python 3.11+ standard library,
this exact installed native binary, a local Responses fixture, and a disposable stdio MCP server.
It creates a fresh child-only Codex configuration home, supplies no account credentials, and never
uses a real model endpoint. Reports contain booleans, counts, tool names, platform, version and binary
digest; temporary provider sessions contain synthetic data and are removed. The probe is development
qualification tooling, not a second product runtime. Run without Python `-O`:

```console
python3 scripts/probe_codex_app_server.py --codex /absolute/path/to/codex
```

The [retained report](r2a-codex-0.152.0-probe.json) records 18 passing assertions/groups on macOS arm64,
`codex-cli 0.152.0`, binary SHA-256
`166e0593c333c1c6412cc9cea72b6e1dfc4fc79b4813da02c511a3339c9b9593`.

| Observed behavior | Consequence |
| --- | --- |
| `prompt` plus `approvalPolicy=never` rejects a read-only MCP call before dispatch | The existing R1 deny policy cannot implement interactive CanISend consent |
| `on-request` plus explicit user reviewer emits `mcpServer/elicitation/request`; accept dispatches once, decline dispatches zero times | The tested MCP route is elicitation, not an assumed `item/tool/requestUserInput` flow |
| A later turn and a resumed session prompt again; declined turns send no new private tool result | A previous acceptance is not a session-wide grant in this fixture |
| Elicitation carries thread/turn/server and `_meta.codex_approval_kind=mcp_tool_call` plus exact `tool_params`, but no `itemId` or structured tool name | The fixture correlates one active MCP item. Production must reject absent/ambiguous matches; never parse the question text as authority |
| An unrelated fake MCP does not start during initialize; an empty `mcp_servers` override still starts it at thread creation; explicit `enabled=false` excludes it | Authentication-preserving bootstrap must constrain the complete effective server set; an empty map is insufficient |
| A missing required MCP rejects both start and resume, even with an empty map override | Required-server failure is enforceable by the installed version |
| Disabling shell/unified-exec/multi-agent features removes their execution tools; read-only still permits an outside-session `view_image` result | Tool switches and the legacy sandbox do not establish the private-read boundary |
| A named permission profile set before initialize and on start/turn/resume permits the inside-session PNG, denies the outside PNG, rejects a model-supplied command call, and permits the explicitly accepted fixture MCP read | This version has a viable restricted-read mechanism through experimental `permissions`; it is not present in the legacy read-only variant |

The integrated named profile uses `:minimal`, the literal canonical session directory and Codex installation
directory as read roots, with command networking disabled. It is selected with `permissions`, never
combined with `sandbox`/`sandboxPolicy`. The first prototype failed to start its filesystem helper
because it launched through `latest` while allowing the canonical installation path. Resolving the
executable before both spawn and policy construction fixed the fixture. An initially malformed PNG
fixture was replaced with a valid bounded one; the retained report comes from the final runnable script.

These are real local App Server/tool executions with a deterministic local model, not signed-in
OpenAI inference, product approval UI, cross-platform qualification, or complete effective-isolation qualification. The desktop now applies the named policy, but
inherited MCP/host skills, ambiguous/concurrent approvals, scope switching and real CanISend
mutations still need their owned implementation checks. The existing desktop fake elicitation now carries the
observed `_meta` shape and continues to assert rejection and body-free IPC until that integration.

Official references checked against the executable:
[App Server](https://learn.chatgpt.com/docs/app-server),
[configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference), and
[sample configuration](https://learn.chatgpt.com/docs/config-file/config-sample).
The installed behavior, not these moving pages alone, owns the capability findings above.

### Installed account availability

The explicit `--installed-account` mode reuses the installed Codex configuration home and applies
only process-level named permissions and tool/plugin/hook feature restrictions before initialize.
It calls `account/read` with `refreshToken=false`, returns availability booleans, and creates no
thread or model turn. It never opens/copies credential files or writes global configuration.
The [safe result](r2a-installed-account-probe.json) reports an existing account available on this
machine. This proves account recognition under the proposed bootstrap settings; successful signed-in
inference, closed inherited configuration and product consent still require integration checks.

```console
python3 scripts/probe_codex_app_server.py --codex /absolute/path/to/codex --installed-account
```

### Verification of this probe slice

- Initial `scripts/probe_codex_app_server.py --codex ...` run: all 17 then-current checks/groups passed; the exact
  report has since been refreshed by the runtime integration below. Python syntax inspection passed. No dependency was added.
- Desktop protocol test: two passed, one existing signed-in smoke ignored. The initial parallel run
  failed its success assertions in 0.18 seconds with the previous 150 ms startup budget; a focused
  rerun passed. Normal fixture startup now has two seconds, while the dedicated timeout case retains
  150 ms and its failure assertion. Production deadlines were not changed.
- `cargo clippy -p canisend-gui --all-targets --features app-server-test-fixture --locked -- -D warnings`:
  passed. `cargo fmt --all --check`, document links/JSON parsing, and `git diff --check` passed.
- `cargo run -p xtask --locked -- release check`: passed; existing four release drift items, three
  stage blockers and 14 freeze exceptions are unchanged. No protected CI or release qualification
  was performed.

## Named-policy runtime integration — 2026-09-05

The existing `CodexAppServer::connect` now canonicalizes the executable/session directory and
supplies the named filesystem/network policy before spawning initialization. It selects that same
profile on start/resume/turn, enables the required experimental API, and keeps explicit user/never
approval fields. The production start path rejects versions other than `codex-cli 0.152.0`; rejected
policies end the connection without a legacy fallback. Process overrides disable shell/unified exec,
multi-agent, shell snapshots, hooks, plugins, apps, skill dependency installation and web search.
No account files or global settings are edited. This integrates the restricted-read policy; it does
not establish a closed inherited tool/configuration set or enable private CanISend tools.

The updated real probe has 18 passing groups with the desktop's literal-directory CLI override
recipe. It verifies the recipe against inherited enabled execution features, and resumes with a
fresh named profile as the desktop does. Inside-image success, outside-image denial, forced-command
rejection and MCP accept/decline still pass. Installed account recognition also passes again; it
remains an account-only check, with zero model turns or token refreshes.

A new bounded configuration-reload regression proves that a required synthetic MCP added after
`config/read` is loaded at thread creation despite disabling every inventoried name. Therefore a
snapshot-and-disable implementation would leave a race and is not an accepted bootstrap solution.
The inventory check passes by observing that counterexample; it does not claim the race is fixed.

Verification:

- Desktop protocol: two passed, one signed-in smoke ignored. The same fixture checks process
  overrides before initialize, policy consistency, start/resume rejection without fallback, executable
  aliases and paths with spaces/quotes/Unicode, cleanup, and body-free errors.
- `cargo test -p canisend-gui --locked --lib codex_named_policy_rejects_unqualified_versions_and_profile_keys`:
  one passed; unsupported versions and malformed profile keys fail closed.
- Strict desktop Clippy passed. No new dependency, frontend flow, or registry migration was added.
- `cargo fmt --all --check`, `git diff --check`, Python AST parsing and the scoped document
  checker passed (23 Markdown files, 169 local links/anchors, seven retained JSON/JSONL files).
- `cargo run -p xtask --locked -- release check` passed; existing four drift items, three stage
  blockers and 14 freeze exceptions remain unchanged. No protected CI/native/signed-in product
  qualification was run. The existing macOS linker unwind-size warning remains non-blocking.

## Application session binding — 2026-09-05

The desktop now sends `selected_application_id` for the Workbench's selected Application instead
of treating its compatibility-view `job.id` as a retired Job ID. The shared runtime resolver verifies
that the Application exists in that Workspace v4. Catalog filtering, active scope keys, session
lookup/persistence and cancellation all carry the same typed distinction; legacy Job requests remain
available only through the legacy Workspace path. This is session binding, not MCP authorization.

The existing App-local registry writes bounded, body-free v3 rows with a separate `application_id`.
v1/v2 reads migrate in memory, preserve existing identities/metadata and do not rewrite the file;
only a successful save writes v3. Mixed Job/Application rows and malformed IDs are rejected. A Job
and an Application sharing UUID text cannot resume or remove each other's session. Rollback to an
older desktop requires its compatible session-cache copy, or a fresh cache; there is no Workspace
DB/schema migration and canonical product data is unaffected.

The Agent view, bridge and App handler now agree on Application scope. Switching Application,
Workspace or runtime clears provider-send confirmation, refreshes the scoped catalog and drops late
start/result/catalog/stream updates using an in-memory conversation epoch. Returning to a previous
Application can restore displayed messages but cannot restore its prior send confirmation. Clearing
a conversation invalidates its earlier callbacks as well.

Focused verification:

- App registry tests: five passed, including v1/v2 migration, same-UUID Job/Application separation,
  scoped lookup/removal, malformed/mixed identities and unchanged migration source bytes.
- Runtime scope tests: four existing scope regressions passed; the new dual-Pack regression passed
  after correcting the academic fixture category to `qualification`. It covers both Packs, wrong
  Workspace, mixed/missing Workspace bindings, distinct typed keys and exact cancellation isolation.
- Frontend: 40 tests passed across state, bridge commands and Agent v4 screen. The bridge checks all
  four scope-bearing commands; the reducer rejects an A -> B -> A stale event without changing status.
- Svelte/TypeScript check: zero errors/warnings. Strict App/desktop Clippy passed. The pnpm launcher
  attempted a global lockfile write and could not run under the sandbox; existing project tools were
  run directly with installed Node 26.8.1 instead, without changing dependencies or global files.
- Rust/frontend formatting and document checks passed. The source gate initially detected the
  expected domain-coupling inventory change: the Agent state/view no longer match legacy Job
  references, and the screen test is now classified as kernel. The checked inventory was refreshed
  from the existing scanner (184 matching files, 123 legacy-surface matches); scanner rules and
  release exemptions are unchanged. `cargo run -p xtask --locked -- release check` then passed,
  retaining four existing drift items, three stage blockers and 14 freeze exceptions. The document
  checker passed 23 Markdown files, 169 local links/anchors and eight JSON/JSONL files. No protected
  CI, real provider call, native artifact or signed-in qualification was performed for this slice.

## Host Skill discovery and denied-access feedback — 2026-09-05

The exact installed Codex 0.152.0 binary was probed using a temporary `CODEX_HOME`, a synthetic
standalone Skill and the existing loopback model/MCP. The report now contains 21 passing observation
checks, including counterexamples; this is not 21 isolation guarantees. The new three cases test
`features.skip_host_skill_discovery` off/on with an explicit Skill-directory read grant, then on
without that grant. `config/read` confirms the requested flag, but the synthetic description marker
reaches the local model in all three cases. No `skills/extraRoots/set` call is used in the retained
probe. Tool filesystem grants therefore do not establish a closed Skill metadata boundary, and the
skip flag is insufficient for this fixture. No real account, private Skill or online model is used.

ADR-RN-0022 requires reuse of the installed Codex sign-in state. The owner has been asked whether a
separate Codex configuration with a separate provider-managed login is acceptable, or whether to
retain that requirement and advance independent file-history work. No answer has been received and
no ADR, authentication flow, credential file or global configuration was changed. A separate config
is only a candidate: it still requires complete inherited-configuration and Skill qualification.
Private embedded MCP remains disabled.

The existing Agent Alert now reports denied command execution, file changes and additional
permissions in English/Chinese. It follows the existing body-free runtime event and existing refusal
handling; MCP elicitation is not presented as a host-access denial. New turns/conversations and scope
changes clear the notice. It has no approval action and uses the Alert's existing accessible role.

Focused verification:

- Frontend state/bridge/screen: 41 tests passed. The reducer covers all three refusal methods and
  excludes MCP elicitation; the v4 screen renders the alert in both locales and Application scopes.
- Svelte/TypeScript: zero errors/warnings. Impeccable's scoped detector returned no findings.
- After strengthening the per-method assertion, all eight state tests passed again. Frontend
  formatting, Python AST parsing, `git diff --check` and the scoped document checker passed
  (23 Markdown files, 169 local links/anchors, eight JSON/JSONL files).
- Exact native probe: 21 expected observations passed; body-free results retained in the existing
  JSON report, with `embedded_enablement: false`. The previous account-only result is unchanged.
- No Rust source, dependency, schema or release record changed in this slice. No protected CI,
  signed-in product smoke or native artifact qualification is claimed.

## Dedicated Codex configuration and browser sign-in — 2026-09-05

The owner accepted a separate configuration and separate provider-managed login. ADR-RN-0022,
the parent plan, R2 checklist and master roadmap now record that amendment. Earlier account-reuse
observations remain historical evidence; they no longer qualify the selected login path.

The runtime and login command share `agent-runtime/codex-isolated-v1` beside the App-local registry.
The child-only user directory and `CODEX_HOME` are separate from the external Codex installation's
configuration. Ambient provider tokens, launch context and configuration-root variables are excluded;
only executable search, OS temporary/locale and browser-display essentials are forwarded. Credentials
use Codex's own file store in `provider`, with ChatGPT login selected. No credential read, copy,
symlink or global edit is performed. Provider directories reject links/files and use Unix mode 0700.
Initialization must echo the exact selected `codexHome`. Session cleanup retains provider state.

The Agent page now starts Codex's official browser login through one bounded desktop command. Login
is serialized against other logins and Codex session/turn workers, times out after five minutes,
and retains zero stdout/stderr bytes. Starting a login closes cached provider connections, even if
the replacement login fails. The frontend adds
English/Chinese guidance and disables duplicate sends during sign-in. Every login attempt revokes
provider-send confirmation, including failed attempts; success starts a new conversation. Users must confirm provider sending again. Old external
thread IDs are not migrated; start a new conversation in the dedicated provider directory.

The exact native local probe now has 25 expected observations. A positive fixture proves the
synthetic user Skill appears under its external HOME, while the dedicated HOME excludes it. A
separate CODEX_HOME ignores required external MCP definitions, including changes after initialize.
With the production HOME/provider nesting, the named policy allows an image inside the session and
rejects a synthetic image in the provider-state directory.
These are local macOS arm64 results, not a claim that managed/admin/system sources or arbitrary
changes inside the dedicated provider home have been qualified. Existing private tools remain off.

Verification so far:

- Desktop App Server protocol: three passed, one real-provider smoke ignored. Coverage includes
  shared login/session directories, persistent provider state, authentication failure, wrong returned
  config home, bounded login timeout and exclusion of raw login output from errors.
- Directory-owner regression passed: reuse retains provider state; linked/file provider roots fail.
- Login/session lease regression passed: an active turn rejects login without changing pending state;
  an acquired login excludes turns/duplicate login and clears old pending state.
- Frontend state/bridge/screen: 42 tests passed across the affected suites after fixing the screen
  fixture to await its reactive tab rendering. Both locale buttons invoke the credential-free command;
  both failed and successful login revoke send consent; success starts a new conversation.
- Svelte/TypeScript: zero errors/warnings. Strict desktop Clippy passed. Impeccable returned no
  scoped findings. The scoped smoke test now requires the dedicated runtime root and never deletes
  persistent provider state during cleanup.
- `cargo run -p xtask --locked -- release check` initially caught the new `login_codex` leaf missing
  from the Tauri operation inventory. Registering that adapter-only leaf fixed it; the gate passed
  with 131 Tauri leaves and the existing four drift items, three blockers and 14 freeze exceptions.
  Public product operations and all 36 MCP tools are unchanged.
- Rust/frontend formatting, Python AST and document checks passed: 23 Markdown files, 169 local
  links/anchors and nine JSON/JSONL files. No protected CI or packaged-binary qualification ran.
- Actual dedicated Codex browser login was started with no credential reads/copies and no model
  requests. It did not complete within five minutes; the process was killed and reaped. This is a
  recorded timeout, not a verified authentication outcome. Retry from the new Agent login button.
  Private MCP and the signed-in product acceptance gate remain closed.

## Next acceptance work

Complete the dedicated browser login and qualify managed/system configuration and the effective
capability set without credential copying or global edits. External user Skill/MCP exclusion is
proven locally; do not substitute snapshot-and-disable for a closed effective server set.
Application session/registry binding is now integrated. After the provider gate passes, connect the proven elicitation shape to bounded, uniquely correlated
pending MCP items and the existing Agent view, with Application/revision/digest/provider grants and
existing preview/commit/verify owners. Cross-Pack and disposable signed-in product checks remain open.

No commit, feature-freeze exception, release tag, or package qualification is created by this work.
A future integration must bind the actual source commit and satisfy the existing freeze/release gates.

## CLI-first closeout review — 2026-09-06

App-first execution is now superseded by ADR-RN-0023. The source was retained in
`b06a402fcf59e8fae0ba3c8f351016aef39cd8d7`; this is partial source preservation, not R2 acceptance.
The earlier session-binding description must be read at its actual tested scope: the registry,
Rust resolver, bridge and standalone AgentView fixture pass selected Application identities.
The full App shell still supplies `selectedJobId` from its legacy dossier selection to AgentView,
while the current v4 selector is `selectedV4Application`; `list_application_dossiers` still uses
JobService. Therefore the full shell-to-v4 selection path is not proven and must be repaired and
exercised before claiming completed embedded Application binding. Component fixtures alone did
not establish that reachability. This is retained as an open R2 gate, not additional App work in
the CLI-first closeout. Embedded CanISend MCP/private tools remain disconnected.

Real dedicated sign-in, effective managed/system policy, correlated consent, cross-Pack embedded
mutation, durable origin and exact history remain unaccepted. Existing local fixtures do not
qualify any new native package, provider support, or formal user journey.
