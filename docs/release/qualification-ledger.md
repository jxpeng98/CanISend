# Native Release Qualification Ledger

[`release/qualification-ledger.json`](../../release/qualification-ledger.json) is the machine-readable progress
authority between the published Alpha and Stable. It records evidence references, never certificates, private keys,
tokens, application data, or copied private issue content.

## Stage transitions

| Workspace stage | Required ledger status | Meaning |
|---|---|---|
| Alpha | `pre-beta` | Controls may be prepared, but signed Beta evidence is absent. |
| Beta | `beta-qualifying` | Signing and native/package-channel qualification may run. |
| RC | `rc-qualifying` | Feature freeze is active and clean-tag matrices are being collected. |
| Stable | `qualified` | Every mandatory evidence class below is complete. |

Changing the Cargo workspace version without changing the ledger to the corresponding state makes
`xtask release check` fail. A prerelease ledger cannot set `stable_authorized: true`.

## Stable evidence requirements

Stable requires all of these in the committed ledger:

1. A frozen feature baseline commit. After that baseline, only release-blocker fixes, release evidence, and
   documentation changes are allowed.
2. One qualified Beta tag/source/run with archive-bound signing evidence for macOS arm64, macOS Intel, and Windows
   x86_64.
3. At least two successful RC matrices with different clean tags, source commits, and GitHub Actions run IDs.
4. Passed Beta-to-RC workspace upgrade, old-binary rejection or same-schema behavior, and restore-to-new-path
   evidence.
5. Passed five-target documentation and uninstall evidence with an exact native matrix run ID.
6. Passed Homebrew Cask, Scoop, and WinGet validation plus install/upgrade/uninstall evidence.
7. Final Stable release notes, rollback guidance, and explicit Stable authorization.

The ledger is necessary but not sufficient: referenced GitHub runs, public releases, checksums, attestations,
notarization results, Authenticode results, and package-manager validations must still be independently inspected.
An invented run ID or status string is not qualification evidence.

## Current boundary

The current Alpha ledger is `pre-beta`. It records five-target native archive lifecycle preparation from GitHub
Actions run `29637471699` and deterministic package-manager candidates. `prepared-native` proves the version-neutral
documentation/uninstall control on Alpha archives; it is deliberately weaker than `passed`, which requires the
signed RC-stage matrix named by the Stable gate. Beta signing, clean RC tags, version-pair migration, native channel
lifecycle, final notes, and Stable authorization remain pending. This matches the live repository signing audit and
deliberately prevents an unsigned or unevidenced Stable version bump.
