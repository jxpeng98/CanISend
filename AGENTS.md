# CanISend

Local-first Rust framework for evidence-bound applications. The kernel owns evidence,
consent, review, export, recovery and audit; declarative workflow Packs supply domain
vocabulary and templates. Academic jobs are one reference Pack. CLI delivery is current;
GUI work and full native release qualification remain paused.

## Working here

- Follow the user's authorized outcome. Reuse the current branch and existing plan;
  record scope, actual checks and remaining work once. Create a PR when remote rules
  require it; preserve user changes and use Conventional Commits.
- Accepted ADRs own decisions, the [roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md)
  owns ordering, and machine records/exact artifacts own release facts. See
  [project control](.trellis/spec/guides/project-control.md) for integration and release details.
- Read the relevant [engineering guide](.trellis/spec/backend/index.md) for the layer
  being changed. Existing `.trellis/` plans and specs are reference documents, not
  execution gates. Do not regenerate Trellis hooks or adapters without an explicit request.
- Keep domain rules in the shared application/kernel layer. Adapters use the application
  facade; they must not write SQLite, Blobs or managed projections directly.
- Preserve evidence, consent, path integrity and recovery controls. Synthetic user
  responses belong only in isolated tests; never answer a real user's consent form.

For repository-owned parser/dependency assurance, see the
[scope and test guide](docs/development/defensive-assurance-routing.md).

## Verification

Use the smallest owning regression. Rust changes also need formatting and relevant
Clippy. Documentation-only changes need relevant link/config checks, not Rust tests.
At a resource, public-contract, CI or integration milestone, run
`cargo run -p xtask --locked -- source check`; Fast CI owns the complete remote suite.
Packaging changes also need exact-package installation and lifecycle checks.
Report source checks, remote CI, artifact qualification and real-user acceptance separately.
