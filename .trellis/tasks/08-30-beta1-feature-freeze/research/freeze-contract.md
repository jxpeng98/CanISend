# Feature-freeze contract research

## Authorities inspected

- `release/qualification-ledger.json`
- `release/feature-freeze-exceptions.json`
- `docs/release/feature-freeze.md`
- `docs/release/stage-transitions.md`
- `xtask/src/main.rs`
- Active 1.0 Roadmap and Issue #77

## Findings

1. The machine ledger already contains qualified signed Beta.1 at artifact source
   `6e1397b79031cad54e794ccdc9edca2153f23b3e`; freeze state is canonical planned/null/empty.
2. `activate-feature-freeze` requires a full lowercase commit equal to current HEAD, and write mode
   additionally requires a clean worktree. It renders exactly two files and does not publish.
3. A read-only preflight at exact local HEAD
   `29c009182bba8d22e5d758373770935771d5bcde` reported `writes_performed: false`, with only:
   - `release/feature-freeze-exceptions.json`
   - `release/qualification-ledger.json`
4. A manually extrapolated full hash was rejected before mutation. Final execution must always use
   `git rev-parse HEAD` output verbatim.
5. Freeze history walks every commit in `BASELINE..HEAD` and checks each commit's first-parent
   changed paths. Because protected merges create a merge commit, nonautomatic preparation must
   merge before the baseline is chosen.
6. Current automatic paths include documentation and bounded release evidence but not Trellis.
   Normal task/archive/journal records would therefore require exceptions after activation, even
   though they do not change product or release tooling.
7. The minimum safe adaptation is to exempt only `.trellis/tasks/` and
   `.trellis/workspace/`. `.trellis/scripts/`, `.trellis/spec/`, `.trellis/workflow.md`, platform
   adapters, and all product paths remain reviewed exceptions.
8. Protected preparation PR #207 merged as
   `bd83617efd3f1513d0635721566a4ad895311626`. A subsequent dry-run/write audit found that root
   `RELEASE.md` is intentionally nonautomatic but still contained mutable pending-state wording.
   The unpushed local activation at that baseline is therefore superseded. The minimum safe fix is
   a docs/control amendment that makes `RELEASE.md` state-independent before selecting the final
   protected baseline; activation still uses the existing two-file transaction.

## Verification ownership

- Existing focused tests own HEAD binding, exact two-file rendering, planned-state rejection, and
  exact post-baseline path history.
- Extend the existing policy test for the two new record prefixes and representative rejected
  Trellis control paths.
- One source gate per final PR head plus protected Fast CI owns cross-workspace behavior. No native
  rebuild, full local workspace run, public-asset reverification, or external channel test is
  required.
