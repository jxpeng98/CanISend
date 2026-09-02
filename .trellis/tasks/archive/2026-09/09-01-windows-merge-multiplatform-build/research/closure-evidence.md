# Windows and multi-platform closure evidence

**Closed:** 2026-09-02
**Final protected head:** `9c0d5c5bf809bf8c2aa15344abb7f6b2acd9e4f7`

## Integrated history

- The original Windows checkpoint `42036258fa07fe25e9b42a358b7e64088cbd502e` and macOS
  evidence commit `aaa4e98a8e788fc34d86357efc9749e795e5bd4f` remain ancestors of the final
  protected head.
- [PR #213](https://github.com/jxpeng98/CanISend/pull/213) merged the reviewed Windows repair
  head `467eaae70abe8503aa48350d13ca334b318f3f40` as
  `5d994886ce2bfec327f08c4f6caa48c26cf232b3`.
- [PR #214](https://github.com/jxpeng98/CanISend/pull/214) merged the offline WebView2 packaging
  budget follow-up head `b57cab45abf262450c6d37ad3a4d22f9e3990842` as the final protected head.
- Local `main` and `origin/main` were identical and the worktree was clean before closure.

## Protected checks and packaging evidence

- PR #213 passed dependency assurance and all six Fast CI jobs on the reviewed merge ref.
- Final-head Fast CI run `33562667950` and dependency-assurance run `33562667940` passed.
- Final-head desktop-platform-qualification run `33562710073` passed Windows/Linux packaging and
  native PDF preview checks on macOS, Linux, and Windows.
- The qualification run uploaded a `314757912`-byte compressed Windows package bundle and a
  `146039513`-byte compressed Linux package bundle. These are GitHub artifact sizes, not the
  installed footprint or an individual installer size.

## macOS-to-Windows cross-build

The consolidated cross-build used Apple Silicon macOS, Rust 1.97.0, the
`x86_64-pc-windows-msvc` target, and `cargo-xwin`. The built artifacts were:

| Artifact | Bytes | SHA-256 | Identity |
| --- | ---: | --- | --- |
| `target/x86_64-pc-windows-msvc/release/canisend.exe` | 49,610,752 | `2b347b0ce19058f90c98d502cb28e2766654ca88c74e8bca316acf9771795006` | PE32+ x86-64 console |
| `target/x86_64-pc-windows-msvc/release/canisend-gui.exe` | 63,314,944 | `a3dbf6d678b9145740ede7672b6da0e608ab2895b2a07f247b6c618ec3324df9` | PE32+ x86-64 GUI |

The CLI reports embedded revision `5d994886ce2b`. PR #214 changed only release-size policy,
documentation, the freeze ledger, and the size-budget value in `xtask`; it did not change CLI or
GUI product source. Exact final-head package behavior is therefore attributed to qualification run
`33562710073`, not to these earlier binaries. Cross-compilation is compilation evidence only.

## Native Windows runtime evidence

The runtime check used Windows 11 Enterprise Arm64 under Parallels 27.0.0. The x86-64 artifacts
ran through Windows x64 emulation, so this proves Windows API, registry, process, Vite, and WebView2
behavior but is not native x64 hardware certification.

- The copied GUI hash matched the macOS artifact and launched the real WebView2 desktop host.
- English and Simplified Chinese PATH guidance identified the current-user
  `HKCU\Environment\Path` target and the need to open a new terminal.
- With explicit consent, the product added one `%LOCALAPPDATA%\CanISend\bin`-equivalent entry.
  Independent verification observed one entry of registry type `ExpandString`.
- The original PATH value was restored byte-for-byte with its original type and zero CanISend
  entries. Refreshing the GUI returned the status to **Not configured**.
- No CLI destination or install manifest was created during this PATH-only check.
- On a stock portable Windows toolchain, the first cold `pnpm test:accessibility` run passed 12
  tests while two Axe scans hit the unchanged 30-second timeout. One unchanged warm retry passed
  all 14 tests in 54.0 seconds. Protected Windows CI also passed the same suite.

## Cleanup and residual limits

- Guest test processes, copied binaries, frontend sources, portable Node/pnpm/Chrome tools, and
  registry snapshots were removed. The guest was returned to its prior suspended state.
- Host-side temporary downloads and validation scripts were removed. Repository status remained
  clean before these closure records were added.
- The temporary verifier initially mishandled Windows short-path and slash equivalence. Only that
  verifier was corrected; product code was unchanged. This is retained here as test-harness
  evidence rather than promoted to a product specification.
- No signing, notarization, Authenticode, publication, support-policy expansion, or native x64
  hardware claim was made.
