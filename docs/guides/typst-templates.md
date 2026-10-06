# Versioned Typst templates

The Agent guidance and this guide are generated from the CLI's committed template pins.
An executable upgrade supplies current templates; `workspace upgrade` refreshes installed
project Skills. Follow [upgrade and rollback](upgrade-and-rollback.md) for backups and conflicts.

<!-- canisend:typst-preview:start -->
## Typst Preview templates

For user-owned standalone Typst files, import these exact published packages. No template
repository checkout or copied package implementation is needed. Keep the full version in every
import; change it only during a reviewed upgrade, then compile and inspect the affected PDFs.

| Material | Package import | Entry point |
| --- | --- | --- |
| Academic CV | `@preview/modernpro-cv:2.1.2` | `cv` |
| Cover letter | `@preview/modernpro-coverletter:1.0.3` | `coverletter` |
| Research or teaching statement | `@preview/modernpro-coverletter:1.0.3` | `statement` |

Define the same reviewed `profile` values inline in each document. These fictional values are
syntax examples, not applicant facts; replace them with user-reviewed information:

```typst
#let profile = (
  name: [Fictional Candidate],
  contacts: ((text: [candidate\@example.invalid], link: "mailto:candidate@example.invalid"),),
)
```

Use one of the following snippets in each file, after its profile definition. For a CV, prefer
one column for academic applications and reliable PDF reading order:

```typst
#import "@preview/modernpro-cv:2.1.2": cv, section, summary
#show: cv.with(profile: profile, theme: (font: "Libertinus Serif"), options: (last-updated: false))
#section("Research Profile")
#summary[Replace with a reviewed research summary.]
```

For a cover letter, supply reviewed recipient values and write short paragraphs:

```typst
#import "@preview/modernpro-coverletter:1.0.3": coverletter
#show: coverletter.with(
  profile: profile,
  recipient: (organization: [Fictional Institution], greeting: [Dear Search Committee,]),
  theme: (font: "Libertinus Serif"),
)
Replace with reviewed letter text.
```

Use the same package for each statement; set its title and headings to the actual requirements:

```typst
#import "@preview/modernpro-coverletter:1.0.3": statement
#show: statement.with(profile: profile, title: [Research Statement], theme: (font: "Libertinus Serif"))
= Research agenda
Replace with reviewed statement text.
```

Compile a user-owned file with `typst compile cv.typ cv.pdf` (or the corresponding letter or
statement paths) in the external Typst CLI. Its first import of a version requires a package
download; cached packages work offline. Ensure the selected font is available to that compiler.
Escape Typst syntax in user text, including `\@` in content blocks; omit optional icons and
photos unless requested. Inspect contacts, page breaks and extracted PDF text before delivery.

CanISend-managed Deliverables use the verified template in their exact bound Workflow Pack,
through structured content, review and guarded export. The embedded renderer works offline and
does not compile arbitrary Preview imports or user-authored Typst. A standalone PDF is not a
CanISend-reviewed export. Do not replace managed projections with these examples or treat template
placeholders as Evidence. Existing Applications retain their original Pack/template identity.

After upgrading the CLI, run `canisend --workspace PATH workspace upgrade` to refresh project
Skills, including this template guidance; global Skills use `host setup --host HOST --scope global`.
Reconnect the Host and reread the installed Materials Skill for current pins and APIs. Bundled
Host guides delegate authoring details to this Skill; a durable pointer in user-owned `AGENTS.md`
does not need new version numbers on each template upgrade. Exported Agent packs still contain
resource snapshots; use the selected Workspace/Host's managed Skills for ongoing work, and
regenerate a pack when its protocol or workflow instructions change. Never overwrite a user's
`AGENTS.md` or `CLAUDE.md`.

API references: [CV](https://typst.app/universe/package/modernpro-cv/),
[letters and statements](https://typst.app/universe/package/modernpro-coverletter/),
[package versions and caching](https://github.com/typst/packages/blob/main/README.md).
<!-- canisend:typst-preview:end -->

## User-owned AGENTS.md

For a user-owned `AGENTS.md`, a durable instruction can point to the upgraded Skill rather than
copy its template implementation:

> For CanISend authoring, follow the installed `canisend-materials` Skill. Use the exact Typst
> Preview versions and entry points in its template guidance; retain the Application's bound Pack
> for managed Deliverables.

Project initialization and upgrade do not replace this user-owned file. Review it when changing
the Host or authoring workflow; current package versions arrive through managed Skill updates.
The bundled Codex, Claude and generic guides use this routing: project-local Materials Skills live
at `.agents/skills/canisend-materials/SKILL.md`, `.claude/skills/canisend-materials/SKILL.md` and
`skills/canisend-materials/SKILL.md`, respectively. If the selected installation is missing, run
`canisend --workspace PATH workspace upgrade --host HOST`; without `--host`, upgrade refreshes only
existing installations. Global Skills use scoped `host setup`. Preserve customizations when a
managed-file conflict is reported, then reconnect the Host and reread the updated Skill.

## Maintainer synchronization

After a reviewed template update, synchronize the local upstream sources as described in the
[template execution plan](../architecture/typst-template-preview-execution-plan.md).
That command updates bundled templates, preserves historical Pack bytes, advances changed guide
versions, and updates the Preview pins and usage in root `AGENTS.md`, the managed Materials Skill
and this guide together. Exported Host guides retain stable Skill links and upgrade instructions;
their resource versions advance only when that routing changes.

To refresh or check only the instructions using committed source pins, without cloning template
repositories:

```sh
python3 scripts/sync_typst_templates.py --guidance-only
python3 scripts/sync_typst_templates.py --guidance-only --check
python3 -m unittest discover -s scripts -p test_sync_typst_templates.py
cargo run -p xtask --locked -- source check
```

Guidance-only synchronization does not upgrade template implementations, change an Application's
Pack, or select a new upstream release. Review a template update before advancing its committed
pins. Fast CI checks guidance drift and the synchronization regressions.
