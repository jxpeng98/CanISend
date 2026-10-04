# CanISend Workspace

This Workspace can contain academic and generic Applications at the same time. Each Application
chooses its own Workflow Pack; the Workspace itself has no mode.

## Start here

1. If Skills were not installed during initialization, run
   `canisend --workspace . host setup --host codex` (or `--host claude`). Project scope installs
   into `.agents/skills/` for Codex or `.claude/skills/` for Claude Code. Use `--scope global`
   only when you want these Skills available across projects in your user home directory.
2. Run the MCP registration command returned by setup, then open this Workspace in the selected
   Host and reconnect/discover its tools. `host status` checks managed resources, not a live MCP
   connection. Use `canisend-application-workflow` for a complete application, or
   `canisend-workspace` to build your Profile and manage this Workspace.
3. If you have no prepared Profile, ask the Host: "Use canisend-workspace to help me build my
   Profile through conversation. Ask a few questions at a time, show me factual summaries to
   review, and save my progress." You can start before choosing an opportunity or Application.
   The Host keeps editable drafts in `inputs/profile-interview/` by default, separates uncertain
   details and future intentions, and reads those drafts when you resume. Without file tools,
   it provides copyable text. Review the Profile before importing it with the required consent.
   If you already have material, review and import your own Typst, Markdown, text, or JSON
   Profile Source through the CLI or Host. `profile/profile-example.typ` is a fictional example;
   see `canisend profile-source import --help`.
4. Select a Workflow Pack and create an Application; see `canisend application create --help`.
   The files in `examples/generic-v4/` are fictional intake references.
5. Copy and edit the bundled Typst files in `templates/` when a Workflow Pack allows a custom
   template. The desktop App is not required.

Importing a Profile Source does not confirm Evidence or associate it with an Application.
The Host guides those guarded steps after you choose an Application. Workspace backup preserves
imported Profile Sources; keep a separate copy of unfinished interview drafts.

`canisend.toml` and `.canisend/` are authoritative. The `applications/`, `jobs/`, and `agent/`
projection folders start empty and are populated by CanISend operations; do not add private
material there as a substitute for importing a Profile Source.
