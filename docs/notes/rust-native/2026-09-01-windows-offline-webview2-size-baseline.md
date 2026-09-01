# Windows offline WebView2 size baseline

Date: 2026-09-01

## Decision

Raise only the nonpublishing Windows offline WebView2 installer budget from 268,435,456 bytes
(256 MiB) to 301,989,888 bytes (288 MiB). Standard NSIS/MSI packages, the application payload,
the unified host, the frontend, and runtime-inclusive extracted-payload budgets remain unchanged.

## Same-target evidence

Both records use `x86_64-pc-windows-msvc`, the `release` profile, `opt-level=z`, fat LTO, one
codegen unit, symbol stripping, and panic abort.

| Measurement | Run `32811662016` at `cd40180f2ff8ac957276f1948ba88da428511a82` | Run `33549382112` at `5d994886ce2bfec327f08c4f6caa48c26cf232b3` | Delta |
| --- | ---: | ---: | ---: |
| Offline installer | 231,502,468 | 277,519,887 | +46,017,419 |
| Application payload | 44,989,918 | 44,996,574 | +6,656 |
| Unified host | 44,580,128 | 44,586,784 | +6,656 |
| Frontend | 971,424 | 971,865 | +441 |

The product payload is effectively unchanged while the runtime-inclusive installer grew by
46,017,419 bytes. The qualification log shows Tauri's `offlineInstaller` mode downloading the
current Microsoft WebView2 offline runtime through Microsoft's mutable download endpoint. This is
dependency/runtime movement rather than product growth. The new budget leaves 24,470,001 bytes of
headroom above the observed installer while preserving a bounded regression tripwire.

## Preserved controls

The package configuration, runtime source, target, release profile, signing smoke, URL and path
policy, symlink rejection, one-host validation, SHA-256 recording, privacy boundary, and native
WebView2 render qualification are unchanged. The existing standard-package and product-payload
budgets continue to catch CanISend growth independently. Pinning an older fixed WebView2 runtime
was rejected because it would create a separate runtime-maintenance and security-patching burden.

The earlier successful evidence is available at
<https://github.com/jxpeng98/CanISend/actions/runs/32811662016>; the later run that built the
installer successfully before rejecting its old size ceiling is
<https://github.com/jxpeng98/CanISend/actions/runs/33549382112>.
