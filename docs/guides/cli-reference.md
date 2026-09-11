# CLI command reference

This reference describes Beta.8 and newer. Installed releases may lag behind;
use `canisend COMMAND --help` to check your binary. All existing long names remain supported.

## Common options

| Option | Meaning |
| --- | --- |
| `-w, --workspace DIR` | Select a workspace. Otherwise, commands find one in the current directory or its parents; `ws init` creates one in the current directory. |
| `-a, --application ID` | Select an application on commands that accept it. Get IDs from `app list`. |
| `-f, --candidate JSON_FILE` | Read an application request or worker result file. `--file` is an alias. |
| `--text` | Print readable text, including when output is redirected or piped. |
| `--json` | Print the structured response for scripts. |
| `-h, --help` | Show command-specific usage and required arguments. |

Workspace and output options work before or after the command. Choose one output mode:
terminal output defaults to readable text; redirected output defaults to JSON for compatibility.
Text errors go to stderr; structured application errors go to stdout. Invalid command syntax
uses stderr and exit code 2. `mcp serve` uses JSON-RPC over stdio and rejects both output flags.

Short names: `ws` = `workspace`, `app` = `application`, `source` = `profile-source`,
`links` = `association` under `profile` and `evidence`, `review show` = `review inspect`.
Aliases produce the same JSON operation names and data as their long forms.

```sh
canisend -w ./applications ws init --host codex
canisend -w ./applications ws upgrade
canisend -w ./applications app list --text
canisend -w ./applications app show -a APPLICATION_ID
canisend -w ./applications requirement list -a APPLICATION_ID --json
canisend -w ./applications app create --pack org.canisend.generic-application -f request.json
```

Setup prints an MCP registration command: run it, open the workspace in your Host, then
reconnect. For Claude Code, use `--host claude`. See the [walkthrough](quick-start.md)
for a complete application request file.

## Workspace and Host

| Command | Purpose |
| --- | --- |
| `ws init [--host HOST] [--scope project\|global] [--no-skills]` | Create a workspace; optionally install Skills. `--scope` requires `--host`; `--no-skills` skips the interactive choice and conflicts with `--host`. |
| `ws upgrade [--host HOST]` | Upgrade storage and installed project Skills; select or install one Host with `--host`. |
| `ws status` | Show workspace identity, version and application count. |
| `ws check` | Check data and generated files. |
| `ws backup DIR` | Create a verified backup in a new directory. |
| `ws restore BACKUP_DIR DIR` | Restore into a new or empty directory. |
| `ws repair` | Repair generated files while preserving user edits. |
| `host setup --host HOST` | Install or update Skills and print MCP setup instructions. |
| `host status --host HOST` | Show Skills status and MCP setup instructions. |
| `host remove --host HOST` | Remove unchanged managed Skills. |

`HOST` is `codex`, `claude` or `generic`. Host commands default to project scope;
add `--scope global` for user-wide Skills. Setup/status accept `--executable PATH` when
MCP should use a different absolute CLI path. Host commands leave MCP configuration for you
to register or remove. Upgrade and removal preserve locally modified managed files.

## Applications and documents

Use `-a ID` for the application on each selected-application command below.

| Command | Purpose |
| --- | --- |
| `app list` | List applications, IDs, states and revisions. |
| `app create --pack PACK_ID -f JSON_FILE` | Create an application from an opportunity and its requirements. |
| `app show -a ID` | Show progress and the next step. |
| `app pack show -a ID` | Show the bound workflow Pack and document types. |
| `app archive -a ID --expected-revision N` | Archive an application and retain its history. |
| `source list` | List imported source metadata. |
| `source import FILE --sensitivity public\|private-local` | Import a Typst, Markdown, text or JSON profile source. Private-local input also requires `--confirm-private-read`. |
| `profile links list -a ID` | List available profile sources and their application links. |
| `evidence links list -a ID` | List confirmed evidence and its application links. |
| `requirement list -a ID` | List requirement IDs, statements and confirmation states. |
| `requirement show -a ID --requirement ID` | Show one requirement and its revision. |
| `plan show -a ID` | Show the document plan, decisions and blockers. |
| `deliverable list -a ID` | List document IDs, titles, states and revisions. |
| `deliverable show -a ID --deliverable ID` | Show one document's metadata. |
| `review show -a ID --confirm-private-read` | Read current document content for review. Text output preserves paragraphs. |
| `export list -a ID` | List existing export directories and document counts. |
| `export show -a ID --destination DIR` | Verify an export and list its files. Use the workspace-relative directory from `export list`. |

Creating drafts, changing evidence links, approving and producing exports use the
[MCP workflow](agent-integration.md) in one connection. CLI `export` commands inspect
existing files. `--confirm-private-read` authorizes only the requested local read.
Revision checks reject stale writes; take `N` from the current `app show` result.

## Local workers

These commands coordinate draft work inside a workspace. `submit` saves a local worker result.
CanISend never submits an application to an external service.

| Command | Purpose |
| --- | --- |
| `local-task list -a ID` | List up to 100 recent tasks for an application. |
| `local-task prepare -a ID --expected-revision N` | Create a task for the current application revision. |
| `local-task show --task ID` | Show task state, generation and lease details. |
| `local-task claim --task ID --expected-generation N` | Claim work or renew an expired lease. |
| `local-task submit --task ID --expected-generation N --lease ID -f JSON_FILE` | Save a result under the current lease. |
| `local-task cancel --task ID --expected-generation N --lease ID` | Cancel a claimed task. |
| `local-task candidate-show --task ID --confirm-private-read` | Read the saved result; use `--text` for readable paragraphs. |

Use the generation from the latest task response and the lease ID from `claim`.
Task metadata never includes private result content.

## Diagnostics and integration

| Command | Purpose |
| --- | --- |
| `version` | Show version and build identity. |
| `doctor` | Verify the installation, resources and a PDF render. |
| `schema list` | List public schema IDs and versions. |
| `schema show ID` | Show schema metadata by ID or short name. |
| `resource list` | List bundled resources, versions and kinds. |
| `mcp serve [-a ID]` | Start the Host connection; optionally restrict it to one application. |

Diagnostics and catalogs work without a workspace. `mcp serve` requires one.
