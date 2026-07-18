# R11.4 Stable channel-manifest publication

**Date:** 2026-07-18

**Roadmap item:** R11.4 Stable archives and package-manager manifests

**Status:** Stable release path implemented; real qualified Stable publication pending

## Gap found

The native release workflow published verified platform archives, while every generated Homebrew, Scoop, and WinGet
file remained explicitly nonpublishing. Even a fully qualified Stable tag therefore had no path to place canonical
package-manager manifests in the public release unit.

## Release-unit change

Stable assembly now requires the canonical qualified ledger and generates five manifests directly from the final
signed archive identities:

- a dual-architecture Homebrew Cask;
- a Windows x86_64 Scoop manifest;
- WinGet version, locale, and installer manifests.

A sixth JSON asset binds their bytes and intended external repository paths to the final Git source, archive hashes,
native package qualification run, and final RC matrix. The release manifest lists all six files, `SHA256SUMS` covers
them, the existing OIDC provenance step attests them, and the existing release upload publishes them atomically with
the Stable archives. Public verification regenerates every manifest and rejects altered bytes or metadata.

## Authorization boundary

The publication record authorizes only `github-release-assets` and records `external_index_submission: false`.
Automatically changing a Homebrew tap, Scoop bucket, or `winget-pkgs` would be a different external action with
different ownership and review. This implementation provides exact review-ready files and repository paths without
claiming that those external indexes have accepted them.

## Remaining evidence

The R11.4 checkbox remains open until the repository reaches a genuinely qualified Stable ledger and the Stable tag
publishes the signed archives and these six assets. The current Alpha build cannot generate or preauthorize them.
