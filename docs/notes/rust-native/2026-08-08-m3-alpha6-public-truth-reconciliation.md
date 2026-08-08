# Alpha.6 public truth reconciliation

**Date:** 2026-08-08

**Roadmap task:** `M3-TRUTH-001`

**Scope:** Reconcile the public Alpha.6 checkpoint before Workspace v4 implementation without
rewriting its tag, artifacts, contracts, or historical limitations.

## Public identity

- Tag: `v1.0.0-alpha.6`.
- Source commit: `e524a6d2168c5b71525149e575861c79548aaf13`.
- Public release:
  <https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.6>.
- Published: `2026-08-04T04:37:33Z`.
- Release state: public prerelease, not a draft.
- Public asset inventory: five standalone CLI archives, Apple Silicon macOS ZIP and DMG,
  qualification records, manifest, checksums, SBOM, notices, limitations, and release notes.

The checked-in `release/alpha-package-contract.json` binds Alpha.6 to:

- `canisend.agent/v3`;
- `canisend.workspace/v3`;
- `canisend.workflow-pack/v1`;
- academic Pack digest
  `3baa6d1a3ddf057ba1e5aaf02d8cabb037366b3651f5566bfcf2b2bb166a8d07`;
- generic Pack digest
  `ffe269ae905b7fac851d82719f989876c7d310216b12922be6a5dd1aff67b321`;
- resource-manifest digest
  `1db3b5b35eb6eb96da6c0caab31ff35e35cee932daf3a56efa2ebc9e8aa74476`; and
- the 17-migration tree digest
  `d6ba9d1fb7ba1a92374b97534019d1ca247dc3fc4a2dac4f4cf2ae3181b81dae`.

## Build-once and public verification evidence

The nonpublishing candidate run was
[`30875288165`](https://github.com/jxpeng98/CanISend/actions/runs/30875288165). It completed on the
exact source commit and passed the source gates, five standalone target builds, Apple Silicon
desktop ZIP/DMG build and smoke, Windows release tests, signing readiness, release assembly, SBOM,
checksums, and provenance generation.

The annotated-tag promotion and public verification run was
[`30877749074`](https://github.com/jxpeng98/CanISend/actions/runs/30877749074). It located the exact
unexpired candidate, skipped all product rebuild jobs, reverified the candidate and provenance,
staged the exact draft bytes, passed five CLI archive smokes plus the Apple Silicon ZIP/DMG smoke,
published the verified draft, downloaded every public asset, verified public checksums and update
identity, verified GitHub provenance, and recorded body-free public verification evidence.

The public release manifest asset has GitHub-recorded SHA-256
`7f5525ca425e8048c0f802adc73ff36f46b7e56850da8e2286d2857783cc866e`.

## M2 Issue and gate reconciliation

The Alpha.6 Milestone is closed with eleven closed Issues and no open Issue. Candidate, lifecycle,
promotion, and public-reverification outcomes are supported by the two exact native-release runs
above. The Issue bodies were not consistently updated from `planned` or `in progress` even when
their labels were changed to `state:verified`; Issue labels alone are not accepted as evidence.

`M2-AGENT-001` is the exception that cannot be reconstructed from the retained release evidence.
The candidate source gates include deterministic Host Agent smokes, but no body-free exact-candidate
record proving the Issue's required real Codex and Claude scenarios was retained. The public
Alpha.6 release is immutable, so this note records the provider evidence as **not proven** rather
than inferring it from the green release matrix.

ADR-RN-0020 removes old Skills and Agent v2/v3 compatibility from the Alpha.7 product contract.
The missing Alpha.6 real-provider record is therefore dispositioned as a historical validation gap,
not as work to recreate on obsolete host resources. Its replacement evidence is owned by
`M3-AGENT-001`, `M3-AGENT-002`, and `M3-EVID-004` against exact Agent v4/Alpha.7 bytes.

## Pending Beta records

`release/beta-readiness.json`, `release/beta-contract-freeze.json`, and
`release/feedback-snapshot.json` intentionally remain in their canonical pending-Alpha state. They
do not qualify Alpha.6 for Beta. The refresh tooling accepts only a public `v1.0.0-alpha.7` and must
later bind the exact Alpha.7 source, release run/URL, Workspace v4, Agent v4, Skills resources, and
both Pack digests. Treating these files as an Alpha.6 qualification ledger would freeze the wrong
contract.

## Outcome

- Alpha.6 public identity, build-once promotion, package lifecycle, and public-byte verification
  are reconciled.
- The unretained Alpha.6 real-provider evidence is explicit and is not presented as verified.
- No Alpha.6 tag, artifact, package contract, or historical evidence is changed.
- Alpha.7 implementation may proceed, but its candidate remains blocked until every M3 P0 source
  task and new Agent v4 provider-evidence task passes.
