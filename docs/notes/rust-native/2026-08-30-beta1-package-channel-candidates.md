# Beta.1 package-channel candidate evidence

Date: 2026-08-30

## Qualified input

- Tag: `v1.0.0-beta.1`
- Source commit: `6e1397b79031cad54e794ccdc9edca2153f23b3e`
- Candidate run: `33281162734`
- Release-manifest SHA-256:
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`
- `SHA256SUMS` SHA-256:
  `3af34e9ac644ef4dabc550b3af57c3a5dc587bcd34e35457fcc5f8ea3653950a`
- Qualification merge: `43dc80b0fb5e3accc602795c8e3b706e0bce8fea`

The retained independent public download still contained all 20 assets used for qualification.
The existing release verifier rechecked all 19 manifest-managed files, checksums, manifest fields,
signing records, and native archive identities before generation. No artifact was rebuilt.

## Deterministic output

The existing `xtask release channels` command generated and immediately exact-revalidated six
files under `packaging/candidates/v1.0.0-beta.1`:

| File | SHA-256 |
|---|---|
| `candidate-source.json` | `d95588723a1fc4f66fd4d59aea002fdd9e7336385845b166295b055036b63141` |
| `homebrew/Casks/canisend.rb` | `e49921e00071893afa1b506e24fa7c69b751646932de76cad8652dfbf2dba746` |
| `scoop/bucket/canisend.json` | `4778e304b62c871ab5d55a0a3047461a340cc8c0afdfeaa65f2271f9fed8efeb` |
| WinGet installer | `3e4a4b4a10f5644f1b80a273fbf95ed8e1ea57b875ef7c92ecd66c83560e440e` |
| WinGet locale | `8c750ce51ba094753d42f6f8e283dedc59b43565f7eb15616ed9c933f3b3f8b9` |
| WinGet version | `05d6619b5aa06feb2f92c26f87e3f2a17a04e26d1416e63fd102e2fa1dda81b8` |

The source record binds the exact release tag, source commit, manifest digest, and these public
archives:

| Target | Archive SHA-256 | Size |
|---|---|---:|
| `aarch64-apple-darwin` | `52d982f8a5a8cc9a2eb564df5994b42e0825734ef086e3da432846acb2352522` | 22,043,810 |
| `x86_64-apple-darwin` | `246580fac393fab2b7a00a9b7358c455bf9136ba24d0ba29abb1e55b7b196243` | 23,734,135 |
| `x86_64-pc-windows-msvc` | `fcf716be63dda6627d13392638e9ccd7c6829668080abc5516a149fe9aea1db8` | 22,265,975 |

## Product-metadata boundary

The shared renderer previously described every version as an academic-job tool. Current 1.0
candidates now use `Prepare evidence-bound applications and submissions locally`, general
application/Agent/CLI tags, and `GPL-3.0-only`. A version boundary at the accepted Alpha.6 generic
framework transition preserves the exact historical 0.7 description, tags, and MIT license. No
historical candidate file changed.

## Verification

- The existing focused channel-rendering regression passed for historical paths, hashes, license,
  generic description, and generic tags.
- Rust format and xtask Clippy passed.
- The final source gate regenerated and accepted every historical and current candidate tree,
  including the new Beta.1 set.
- No full local workspace suite, native rebuild, host/provider matrix, package-manager lifecycle
  run, or extended assurance was repeated. Protected Fast CI owns the remaining workspace gate.

## Publication boundary

`candidate-source.json` retains `candidate_only: true` and `publication_authorized: false`. These
files are local review candidates. This work did not modify a Homebrew tap, Scoop bucket,
winget-pkgs, GitHub release, tag, public artifact, qualification ledger, feature-freeze state, or
external package index. Native lifecycle qualification and publication remain later release gates.

This note retains no credential, token, private application content, transcript, prompt, or host
path.
