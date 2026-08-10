# Define Roadmap-backed Trellis control contract

## Goal

Document and apply the minimum local rules that make Trellis the just-in-time execution layer for
the authoritative 1.0 Roadmap.

## Requirements

1. Update `.trellis/spec/guides/project-control.md` with the authority layers, task metadata,
   lifecycle mapping, WIP limit, just-in-time child rule, and drift handling approved in the parent
   task.
2. Add a concise Trellis execution subsection to the Master Roadmap governance section without
   adding another backlog or roadmap.
3. Normalize the current Alpha.7 cohort child metadata and record, but do not remotely modify, the
   observed future GitHub wording drift.
4. Preserve existing uncommitted work and do not create M4/M5/M6 Trellis tasks early.

## Acceptance Criteria

- [x] Roadmap and Trellis project-control guidance describe one consistent authority and lifecycle.
- [x] The current Alpha.7 child carries the required Roadmap/GitHub/evidence metadata.
- [x] No product code, Trellis runtime script, custom status, hook, dashboard, or GitHub state is
      changed.
- [x] `git diff --check` and `cargo run -p xtask --locked -- release check` pass.

## References

- Parent: `08-10-1-0-roadmap-trellis-control`
- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md`
- `.trellis/spec/guides/project-control.md`
