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
   connection. Use `canisend-application-workflow` for a complete application, or `canisend-workspace` for setup.
3. Review and import `profile/profile-example.typ`, or your own Typst, Markdown, text, or JSON
   Profile Source through the CLI or Host. Use `canisend profile source import --help`.
4. Select a Workflow Pack and create an Application; see `canisend application create --help`.
   The files in `examples/generic-v4/` are fictional intake references.
5. Copy and edit the bundled Typst files in `templates/` when a Workflow Pack allows a custom
   template. The desktop App is not required.

`canisend.toml` and `.canisend/` are authoritative. The `applications/`, `jobs/`, and `agent/`
projection folders start empty and are populated by CanISend operations; do not add private
material there as a substitute for importing a Profile Source.
