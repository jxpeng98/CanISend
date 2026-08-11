# Install CanISend

CanISend is distributed as a platform-specific native executable. End users do not install Python, Rust, Node.js,
Java, SQLite, Typst, fonts, or a package manager runtime.

## Supported native targets

The `1.0` line publishes these standalone CLI archives:

- macOS arm64: `aarch64-apple-darwin` (`.tar.gz`);
- macOS Intel: `x86_64-apple-darwin` (`.tar.gz`);
- Linux x86_64 with glibc: `x86_64-unknown-linux-gnu` (`.tar.gz`);
- Linux x86_64 static musl: `x86_64-unknown-linux-musl` (`.tar.gz`);
- Windows x86_64: `x86_64-pc-windows-msvc` (`.zip`).

The Apple Silicon macOS desktop GUI is distributed as
`CanISend-VERSION-aarch64-apple-darwin.dmg` and a portable
`CanISend-VERSION-aarch64-apple-darwin.zip`; neither is a sixth CLI target. Linux arm64 is not
supported in the `1.0` line. Choose a CLI archive by operating system, CPU architecture, and—on
Linux—the available C library. `ldd --version` normally identifies a glibc system; use the musl
archive for a musl distribution or when the glibc archive cannot start because its loader is
unavailable.

## Install from a release archive

1. Download the archive for the operating system plus the published `SHA256SUMS` file from the same release.
2. Verify the checksum before extracting.
3. Extract the complete bundle. Keep `LICENSE`, `THIRD_PARTY_NOTICES.md`, and the embedded-asset license directory
   with the binary when redistributing it.
4. Move `canisend` (`canisend.exe` on Windows) to a directory on `PATH`, or invoke it by absolute path.
5. Run the native self-check.

Every archive has exactly one top-level directory named `canisend-VERSION-TARGET`. Keep that directory intact while
testing the release; moving only the executable is appropriate after verification.

macOS or Linux checksum verification:

```console
shasum -a 256 canisend-ARCHIVE
```

Linux systems with GNU coreutils may use `sha256sum` instead. Windows PowerShell:

```powershell
Get-FileHash .\canisend-ARCHIVE -Algorithm SHA256
```

After extraction:

```console
canisend version
canisend doctor
canisend --workspace /path/to/workspace workspace status --json
canisend --workspace /path/to/workspace host status --host codex --json
```

`doctor` must report verified embedded resources and schemas, an embedded Typst renderer, disabled system-font and
runtime-package lookup, and `Python runtime: not required`. Workspace and host status must identify clean Workspace
v4 and current Agent v4 resources. Do not continue with a binary that fails these checks.

## Install the macOS desktop application

On Apple Silicon, download `CanISend-VERSION-aarch64-apple-darwin.dmg`, `SHA256SUMS`, the release
manifest, and the matching macOS GUI DMG qualification JSON from the same release. Verify the
checksum and GitHub provenance before opening the image. Its read-only volume contains exactly:

```text
Applications -> /Applications
CanISend.app
CanISend.app.manifest.json
```

Verify the companion manifest and the ad-hoc application signature as described in the
[release verification guide](release-verification.md), then drag `CanISend.app` to the included
Applications link. Retain the checksum file, qualification record, and notices with the verified
download. The portable ZIP remains available when a mountable image cannot be used; it contains
exactly `CanISend.app` and `CanISend.app.manifest.json`. Because this free channel is not Developer
ID signed or notarized, macOS may require the normal per-application Open Anyway confirmation
after verification. Never disable Gatekeeper globally.

The application includes the complete version-matched CLI and MCP dispatcher in
`CanISend.app/Contents/MacOS/canisend-gui`. It does not package a second CLI executable or require
a separately installed CLI for GUI operations.

## Install the CLI from the desktop GUI

The macOS-first GUI provides a **Command line** page that copies its signed, version-matched unified
host to `~/.local/bin/canisend`. Installation is user-scoped and does not require a language
runtime, package manager, Rust toolchain, Homebrew, or administrator access.

The page detects only the CanISend version at the exact destination; it does not inspect Python,
package managers, or their environments. An older or version-unaware CanISend interface is offered
as a one-click migration/upgrade. That user action preserves the previous file or symlink for
restoration during GUI-managed uninstall. A newer installed version is never downgraded. CanISend
also refuses to overwrite or remove a managed binary whose recorded SHA-256 digest no longer
matches.

**Check for updates** makes a user-invoked, body-free request to the public CanISend GitHub Releases
endpoint. Preview builds consider published prereleases; Stable builds remain on the Stable
channel. The check sends no workspace, job, profile, source, or document data and does not download
or execute an installer.

The preview does not edit shell profiles. If needed, add `~/.local/bin` to `PATH` using the
copyable command shown in the GUI, open a new terminal, and verify:

```console
canisend version --json
canisend doctor
```

The first packaged GUI channel began at `1.0.0-alpha.1` on Apple Silicon; the latest publicly
qualified checkpoint is
[`v1.0.0-alpha.8`](https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-alpha.8). Verify the
exact release manifest because later source is not a published update. Intel macOS users can use
the standalone CLI archive during Alpha.
Scheduled Intel GUI compilation is a development regression check, not an installable package.
Beta and later require exact-candidate Intel compilation evidence; native Intel runtime
qualification is required before CanISend makes an Intel GUI support claim.

For complete checksum, SBOM, manifest, and GitHub provenance verification, follow the
[release verification guide](release-verification.md).

## Release signing status

The five standalone `1.0.0-alpha.*` CLI archives may be unsigned under the explicit Alpha policy.
The `1.0.0-alpha.*` macOS GUI always ad-hoc signs its bundled CLI, GUI executable, and outer
application, and publishes canonical qualification evidence bound separately to the final desktop
ZIP and DMG.
Beta, release-candidate, and Stable community builds fail closed unless both standalone macOS CLI
executables have verified ad-hoc integrity signatures and the Windows executable has a verified
self-signed Authenticode signature. Each release publishes canonical JSON evidence bound to the
final archive hash.

These free signatures are not publisher identities. macOS builds are not Developer-ID signed or notarized; Windows
builds are not signed by a publicly trusted certificate and have no public timestamp. Gatekeeper, Unknown Publisher,
or SmartScreen warnings can therefore occur. Confirm the tag, checksum, and GitHub attestation before making a
one-binary exception through the operating system's normal security UI. Never disable Gatekeeper, SmartScreen,
antivirus, or execution policy globally to run CanISend.

For Beta or later on macOS, verify the extracted executable before running it:

```console
codesign --verify --strict --verbose=4 ./canisend-VERSION-TARGET/canisend
codesign --display --verbose=4 ./canisend-VERSION-TARGET/canisend
```

The display output must show `Signature=adhoc`, identifier `io.github.jxpeng98.canisend`, and the hardened-runtime
flag. A Gatekeeper assessment may reject the binary because no Apple publisher trust or notarization exists; the
release evidence records this limitation explicitly.

For Beta or later on Windows PowerShell:

```powershell
$signature = Get-AuthenticodeSignature `
  .\canisend-VERSION-x86_64-pc-windows-msvc\canisend.exe
$signature.Status
$signature.SignerCertificate.Subject
$signature.TimeStamperCertificate
```

`Status` is expected to be `NotTrusted` or `UnknownError`, the subject must be `CN=CanISend Community Build`, and no
timestamp certificate may be present. Compare the artifact-specific thumbprint and hash with the published signing
evidence. Continue with the complete [release verification guide](release-verification.md) before using private data.

## Package-manager candidates

The repository contains review candidates for Homebrew Cask, Scoop, and WinGet, but none is currently a supported or
published installation channel. Do not infer that `brew install`, `scoop install`, or `winget install` will find
CanISend from their public repositories. The candidates are generated from verified Alpha release bytes to exercise
URL, SHA-256, architecture, and nested-archive behavior before signed Beta/RC validation.

See the [package-manager candidate guide](../../packaging/README.md) for the source-binding model and the remaining
native validation gates. Stable installation commands will be documented here only after final signed artifacts pass
Homebrew, Scoop, and WinGet install, upgrade, and uninstall tests.

## Build from source for development

Building is not the end-user installation path. Developers need the repository-pinned Rust 1.97.0
toolchain:

```console
cargo build --release --locked
./target/release/canisend doctor
```

## Upgrade and uninstall

Before replacing a binary, run `workspace check` and create a verified backup for each important workspace. Replace
only the executable and bundled notices; never copy a new binary into `.canisend/`.

To uninstall, remove the executable and its notice bundle. Workspaces are ordinary user-owned directories and are
not deleted automatically. Delete them and their backups only after making an explicit data-retention decision.

Opening a workspace with a new binary may apply an append-only Rust-era migration. Follow the complete
[upgrade, rollback, and uninstall guide](upgrade-and-rollback.md); rolling back the executable does not downgrade an
already migrated workspace.
