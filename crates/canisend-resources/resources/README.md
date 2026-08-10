# CanISend Workspace

This Workspace can contain academic and generic Applications at the same time. Each Application
chooses its own Workflow Pack; the Workspace itself has no mode.

## Start here

1. Review and import `profile/profile-example.typ`, or import your own `.typ`, Markdown, text, or
   JSON Profile Source.
2. Create an Application in the App, or run `canisend application create --help`.
3. Use the files in `examples/generic-v4/` as fictional intake references.
4. Copy and edit the bundled Typst files in `templates/` when a Workflow Pack allows a custom
   template.
5. Run `canisend host setup --help` to connect Codex or Claude Code outside the App.

`canisend.toml` and `.canisend/` are authoritative. The `applications/`, `jobs/`, and `agent/`
projection folders start empty and are populated by CanISend operations; do not add private
material there as a substitute for importing a Profile Source.
