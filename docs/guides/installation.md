# Install CanISend

CanISend is distributed as a platform-specific native executable. End users do not install Python, Rust, Node.js,
Java, SQLite, Typst, fonts, or a package manager runtime.

## Supported native targets

The initial release line verifies these archives natively:

- macOS arm64;
- Linux x86_64 GNU;
- Windows x86_64 MSVC.

Additional targets listed in the release roadmap are not supported until their packaged-binary matrix passes.

## Install from a release archive

1. Download the archive for the operating system plus the published `SHA256SUMS` file from the same release.
2. Verify the checksum before extracting.
3. Extract the complete bundle. Keep `LICENSE`, `THIRD_PARTY_NOTICES.md`, and the embedded-asset license directory
   with the binary when redistributing it.
4. Move `canisend` (`canisend.exe` on Windows) to a directory on `PATH`, or invoke it by absolute path.
5. Run the native self-check.

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
canisend agent capabilities
```

`doctor` must report verified embedded resources and schemas, an embedded Typst renderer, disabled system-font and
runtime-package lookup, and `Python runtime: not required`. Do not continue with a binary that fails this check.

## Preview signing status

Alpha and preview archives may be unsigned until the R11 signing/notarization gates are complete. Verify the
published checksum and release provenance. Do not disable operating-system security globally. Stable installation
instructions will name the exact notarized/signed artifacts and supported package-manager channels after those
channels pass the release matrix.

## Build from source for development

Building is not the end-user installation path. Developers need the pinned Rust toolchain:

```console
cargo build --release --locked
./target/release/canisend doctor
```

## Upgrade and uninstall

Before replacing a binary, run `workspace check` and create a verified backup for each important workspace. Replace
only the executable and bundled notices; never copy a new binary into `.canisend/`.

To uninstall, remove the executable and its notice bundle. Workspaces are ordinary user-owned directories and are
not deleted automatically. Delete them and their backups only after making an explicit data-retention decision.
