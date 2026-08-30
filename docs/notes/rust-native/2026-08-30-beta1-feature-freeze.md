# Beta.1 feature-freeze activation evidence

Date: 2026-08-30

## Boundary

This body-free note records the exact repository feature-freeze transaction for qualified public
`v1.0.0-beta.1`. It contains no Workspace, Application, profile, candidate, provider, transcript,
credential, token, private Issue body, or local user path.

The published Beta artifact source remains
`6e1397b79031cad54e794ccdc9edca2153f23b3e`. The separate repository history baseline is the
latest protected preparation merge `acf25dc483643ca9be0210320775708da116b715`.

## Protected preparation

- Control-boundary PR: [#207](https://github.com/jxpeng98/CanISend/pull/207), exact head
  `ffeba42007e96e9be348f4737dc856388b499bdd`, merge
  `bd83617efd3f1513d0635721566a4ad895311626`, Fast CI run `33288394375`, dependency-assurance run
  `33288394360`, 7 required checks passed.
- State-independent policy PR: [#208](https://github.com/jxpeng98/CanISend/pull/208), exact head
  `5729dc950dad2b4c76866842af0d3ef1f9e61893`, final baseline merge
  `acf25dc483643ca9be0210320775708da116b715`, Fast CI run `33289215836`, 6 checks passed.

The preparation retained executable and policy-bearing Trellis paths under exact freeze review and
allowed only `.trellis/tasks/` plus `.trellis/workspace/` as automatic bookkeeping. Root
`RELEASE.md` remained policy-bearing and was made state-independent before the final baseline.

## Activation transaction

Both dry-run and write used exact clean `HEAD`
`acf25dc483643ca9be0210320775708da116b715`. Their schema, baseline, two paths, before digests,
after digests, and next action matched exactly; only mode and `writes_performed` changed.

| Path | Before SHA-256 | After SHA-256 |
|---|---|---|
| `release/feature-freeze-exceptions.json` | `c49b11a05bd07197e3334a00298c7e8e2be6f9f90e28013d8a88d1046205cb6b` | `557ae86e9334f6ada1b1328c1775ea21c0f0dee05d0d4bf6156bd6b305000084` |
| `release/qualification-ledger.json` | `7bff07d6e375fe879aae4e3d5f6e65c84c8f2cdcb223508108b818047b1415d0` | `3a6868c2d9596bf8db23621c03d0c66d749691c0497988deb28ef84daae3bbf2` |

The two machine files were committed alone as
`f0894235c3d56e88f402ef385f5b2ecd46e9c193`. Both now report `frozen` at the same baseline. The
qualification ledger retains allowed classes `documentation`, `release-blocker`, and
`release-evidence`; the exception record starts with zero entries.

## Protected activation review

- PR: [#209](https://github.com/jxpeng98/CanISend/pull/209)
- Exact reviewed head: `329967d71331768ada47f089374dfb464d4751a8`
- Protected merge: `e8487fb53cba1d42fef9f96acf5d5c9d9284188d`
- Fast CI run: `33289756272`
- Required checks: 6 passed, 0 failed

## Local and merge source gates

`cargo run -p xtask --locked -- release check` passed after the activation transaction and
automatic documentation/control reconciliation. Its feature-freeze result was frozen at
`acf25dc483643ca9be0210320775708da116b715` with zero exceptions. `git diff --check` and Trellis
task validation also passed. The same source gate passed again on protected merge
`e8487fb53cba1d42fef9f96acf5d5c9d9284188d`, proving that its first-parent path set remained
automatic.

## Non-claims

This transaction does not create or move a tag, rebuild or publish artifacts, modify an external
package channel, run a participant cohort, prepare RC.1, or authorize Stable. The activation PR
passed its final source gate and protected checks before Issue #77 became Verified.
