# Design

## Boundary

This is an evidence-only validation of `main` on Apple Silicon macOS. The system under test is the
existing Rust workspace, CLI/MCP/host surfaces, frontend, and temporary macOS Design Preview App.
Only Trellis task records may change.

## Evidence flow

1. Capture the immutable Git revision and local toolchain context.
2. Run the repository's native workspace and end-to-end smoke scripts in temporary homes and
   workspaces.
3. Use `scripts/build_macos_design_preview.sh` as the single App build path. It already owns frontend
   checks, visual tests, native compilation, staging, ad-hoc signing, bundle verification, synthetic
   seed data, and receipt generation.
4. Launch only that isolated App and observe the primary product surfaces. Keep provider/application
   interaction local and synthetic.
5. Map command evidence and UI observations to the product contract, then report gaps by owner:
   product defect, environment limitation, deferred Windows work, or release-only qualification.

## Compatibility and safety

- Use the pinned Rust toolchain and a non-global invocation of the repository-declared pnpm version.
- Temporary artifacts stay outside release locations and are not published or notarized.
- Existing scripts choose disposable HOME/Workspace paths and must retain or report them only when
  needed for diagnosis.
- A failed check stops the completeness claim; diagnosis is allowed, but source repair requires a
  separately reviewed task because feature freeze is active.

## Tradeoffs

- The plan favors broad existing end-to-end scripts over new tests. This minimizes duplicate logic
  while covering the actual user contract.
- Native release matrices are not repeated locally. Their signing, packaging, installer, and
  platform ownership remains authoritative.
- The UI observation is representative, not a manual re-test of every frontend assertion already
  covered by the visual and unit suites.

## Rollback

No product rollback is needed because the task does not change product code. Stop processes opened
for the temporary App, remove disposable artifacts when safe, and leave diagnostic output intact if
a failure needs follow-up.
