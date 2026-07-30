# CanISend Stage 4H agent efficiency plan

**Status:** Source implementation complete; external-provider dogfood remains a release gate.

**Architecture decision:** CanISend remains the local-first application state, validation, and
visualization layer. Codex, Claude, or another host remains the Agent runtime, session authority,
search surface, and plugin/connector environment.

## Goal

Reduce the distance between selecting an application in CanISend and continuing useful work in an
external Agent, while making the reusable academic-application instructions smaller, more
discoverable, and more stage-aware.

## Product invariants

- CanISend never becomes a second model client or transcript owner.
- Handoff data remains body-free: paths, public IDs, workflow status, and commands only.
- Host-discoverable skills contain workflow instructions; MCP contains live bounded actions.
- CanISend remains authoritative for revisions, validation, user decisions, and exports.
- No workflow may edit `.canisend`, bypass consent, invent evidence, or submit an application.
- Installing an update must not overwrite a user-modified skill.

## H1 — Progressive skill family

- Add `canisend-application` as the context-first orchestration entrypoint.
- Add focused intake, materials, and review skills.
- Keep each `SKILL.md` concise and front-load its trigger description.
- Generate Codex `agents/openai.yaml` metadata with explicit default prompts.
- Replace the duplicated host-wide 16/17-step guides with small durable boundaries.

## H2 — Safe project installation

- Install Codex skills under `.agents/skills`.
- Install Claude skills under `.claude/skills`.
- Record a host-specific digest manifest outside CanISend internal state.
- Make installation idempotent.
- Upgrade only a file whose current digest matches the prior managed manifest.
- Reject symlinks, unexpected file types, invalid manifests, and local modifications.
- Expose the same operation through the Rust application facade, CLI, and desktop bridge.

## H3 — Optimized start point

- Keep the raw host launch command and body-free prompt in the typed handoff.
- Add a safely quoted one-step command using the host's supported initial-prompt argument.
- Invoke the orchestration skill explicitly.
- Tell the host to read CanISend context, follow exact `next_actions`, avoid repeated questions,
  continue safe inspections/previews, and pause only at decisions or blockers.
- Keep the complete task mechanics in stage skills instead of copying them into every start point.

## H4 — Connected GUI flow

- Turn “prepare handoff” into one explicit “prepare AI workspace” action.
- Install/update managed skills before generating the handoff.
- Make the one-step command the primary call to action.
- Show the installed skill status, recommended skill, and current next action together.
- Keep the raw prompt and context command visible as transparent manual fallbacks.
- Preserve the existing optional in-App read-only bridge and guarded MCP configuration.

## H5 — Distribution and documentation

- Include discoverable skill layouts in every self-contained host pack.
- Keep Codex-only metadata out of Claude and generic packs.
- Document direct project installation separately from standalone pack export.
- Update upgrade guidance so skill updates use the managed installer.

## H6 — Verification

- Validate every skill with the skill-creator structural validator.
- Prove fresh install, no-op reinstall, managed upgrade, local-edit refusal, and host-specific paths.
- Test the CLI binary, application facade, desktop command, TypeScript bridge, and localized UI.
- Run formatter, Svelte check/tests/build, affected Rust tests, strict Clippy, release check, and the
  complete workspace suite.

## Exit

Stage 4H exits when one action in the desktop prepares current host-discoverable skills and a
body-free one-step start command, both Codex and Claude can discover the correct project skills,
local modifications fail closed, and all source gates pass. Real-provider testing remains
separately consented release evidence and is not simulated by local fixtures.
