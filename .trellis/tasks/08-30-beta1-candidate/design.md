# Beta.1 candidate and same-byte publication design

## Boundary

This task is release operation and evidence reconciliation over already-staged Beta.1 source. It
does not change product bytes or introduce release machinery. The existing `native-release`
workflow is the single implementation path.

## Identity chain

~~~text
protected main S
  -> nonpublishing workflow run C
  -> complete release-assets artifact A
  -> independent candidate review I
  -> annotated tag T peeling to S
  -> promotion run P locating A
  -> public prerelease R
  -> fresh public download V
~~~

The task passes only when `S` is
`6e1397b79031cad54e794ccdc9edca2153f23b3e`, `C/A/I/T/P/R/V` all agree on
`v1.0.0-beta.1`, and candidate/public manifest plus checksum bytes match. The tag is created only
after `C/A/I` pass, which keeps a failed candidate nonpublic and untagged.

## Existing workflow ownership

Candidate mode already runs:

- release identity, signing readiness, source gates, and Windows release tests;
- five-target CLI packaging and the supported Apple Silicon desktop ZIP/DMG build;
- archive smokes, community native-signature checks, manifest/SBOM/checksum assembly;
- GitHub provenance attestation and one 30-day complete release-assets artifact.

Promotion mode already:

- locates a successful unexpired candidate by tag name and exact tagged source commit;
- downloads and reverifies the candidate plus provenance;
- creates or safely resumes a draft and uploads the original candidate bytes;
- brokers the draft bytes to five CLI and one desktop native smoke jobs;
- publishes a prerelease only after those smokes pass; and
- downloads the public release, verifies checksums, prerelease/update identity, and attestations.

No additional wrapper or duplicate CI is needed.

## Independent review

The maintainer-side review downloads the candidate into a fresh temporary directory and uses the
existing `xtask release verify-candidate` / `release verify` paths plus `gh attestation verify`.
It inspects the canonical Apple ad-hoc and Windows self-signed signing records and retains only
body-free IDs, digests, counts, URLs, and boolean outcomes.

After publication, a second fresh directory receives all public assets. Its checksums, manifest,
attestations, source identity, release state, and candidate/public byte comparison are checked
before project-control reconciliation.

## Failure and rollback

- Candidate failure: do not tag or publish. Diagnose the failed owning job. Any changed source
  requires a new protected source commit and a new candidate run.
- Candidate review mismatch: do not tag. Never patch or replace an artifact manually.
- Promotion infrastructure failure with unchanged identities: use the existing exact-tag resume
  path. Use `finalize-verified-release` only when its fail-closed preconditions accept the failed
  tag run.
- Integrity, source, or tag mismatch after tag creation: do not move the tag or overwrite public
  bytes. Record the failure and use a sequential prerelease if changed product bytes are required.
- Control PR failure: public Beta.1 remains immutable; fix only the body-free documentation and
  governance projection in a new protected commit.

## Compatibility and migration

None. The candidate packages the already-reviewed Beta.1 source. Workspace migration, Agent/Skill
contracts, dual-Pack behavior, and Codex-first evidence are unchanged and owned by prior protected
work.
