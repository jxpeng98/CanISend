# M2-NATIVE-001 — Desktop-platform qualification run review

Date: 2026-08-03

## Reviewed authority

- Repository: `jxpeng98/CanISend`
- Workflow: `desktop-platform-qualification`
- Run: `30742363439`
- Source: `cb2db0f772ff1931c84427becd4674c59acf9028`
- Event: manual workflow dispatch
- Started: `2026-08-02T09:44:31Z`
- Completed: `2026-08-02T11:18:50Z`
- Conclusion: `success`
- URL: <https://github.com/jxpeng98/CanISend/actions/runs/30742363439>

Every job completed successfully:

- Linux standard and portable one-host build, GUI/CLI/MCP smoke, and DEB/RPM/AppImage packaging;
- Windows standard one-host build, self-signing, GUI/CLI/MCP smoke, and NSIS/MSI/offline-WebView2
  packaging;
- native PDF preview under macOS WKWebView, Windows WebView2, and Linux WebKitGTK; and
- the native-preview matrix summary.

No shared failure or unsupported-platform-only failure required triage.

## Downloaded matrix evidence

The unexpired matrix artifact was downloaded and inspected through the GitHub API:

- artifact ID: `8832038448`;
- name:
  `native-pdf-preview-matrix-cb2db0f772ff1931c84427becd4674c59acf9028-30742363439`;
- archive SHA-256:
  `8e748a504aed6b2e15b7908f28fb2fc81c687a2dd4ff2fbe2ba4dd550e5f66aa`;
- matrix schema: `canisend.native-pdf-preview-matrix/v1`;
- matrix status: `passed`;
- fixture SHA-256: `a5a03ac574483b67eecd2160edf9529c0c36f50dae45613317982a343a5051cc`;
- fallback decision: no target requires PDF.js; direct-preview failure retains the
  review-required system-viewer fallback.

The three platform records bind the same 1,125-byte PDF fixture. Each has a non-empty rendered
frame, a platform screenshot digest, zero policy errors, and a passed production-versus-
qualification host-size record. Qualification-only host overhead remained between 1.0079% and
1.4694%.

## Roadmap effect and boundary

This satisfies the exact M2-NATIVE-001 instruction to inspect run `30742363439` to completion before
the task becomes Ready. It is historical pre-candidate evidence, not Alpha.6 release evidence. The
current source is newer than `cb2db0f`; M2-CANDIDATE-001 and M2-LIFE-001 must still run against the
one exact Alpha.6 candidate commit and packaged bytes before promotion.
