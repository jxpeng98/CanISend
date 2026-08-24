# Alpha.10 dependency-exception review

Date: 2026-08-24

## Outcome

The maintainer reauthorized the unchanged 23-entry dependency exception set for one 14-day
window. Every entry is reviewed on 2026-08-24; its next review and hard expiry are both
2026-09-07. This is a bounded risk acceptance, not dependency remediation.

## Bound facts

- Third-party `Cargo.lock` fingerprint:
  `d2807a35172dc853ad98f7e128f1cbc4737b61aac8cb31f4ddf56c18b05ed903`.
- Third-party package count: 751.
- Advisory tool: checksum-verified `cargo-deny 0.19.5`; the official Apple Silicon archive SHA-256
  is `0cf28e019edb3708ba9755b8c822864ee6d6175d6fc167956972e78ea9ff0be3`.
- Policy authority: `release/dependency-advisory-exceptions.json`, matched exactly to `deny.toml`.

## Reachability review

The seven Typst-path entries remain limited to fixed bounded generated source and embedded,
release-verified fonts. CanISend still accepts no YAML, bibliography, CSL, XML, serialized Typst
input, user-authored Typst, external package/file, or user/system font input. In particular, the
two `quick-xml 0.38.4` vulnerabilities remain behind an unreachable bibliography/parser path; the
fixed projector contains no helper invocation and the embedded CV helper remains declaration-only.

The eleven GTK3 entries remain compile-graph-only for Tauri's nonpublished Linux GUI backend;
published Linux artifacts remain CLI-only. The five Tauri `urlpattern` rust-unic entries still
receive only checked-in capability and application patterns, never user-authored patterns.

Upgrading `typst 0.15.1`, `hayagriva 0.10.1`, or `tauri 2.11.5` cannot currently remove these
paths because those direct roots are already the latest published versions used by the lock. The
per-entry removal conditions remain authoritative. A new advisory, lock change, expanded input or
platform, changed reachability, missed review, or expiry blocks the release gate.

## Verification

- `cargo run -p xtask --locked -- dependencies fingerprint` returned the bound 751-package facts.
- Exact `cargo-deny 0.19.5 check advisories bans licenses sources` passed with no new unaccepted
  advisory.
- `cargo run -p xtask --locked -- dependencies check` passed with 23 reviewed exceptions, two
  vulnerabilities, and 751 third-party packages.
- `cargo run -p xtask --locked -- release check` passed on the combined Alpha.10 headless worktree.

No Alpha.10 candidate, tag, package, or publication is authorized by this note. Protected CI still
owns integration of the reviewed source.
