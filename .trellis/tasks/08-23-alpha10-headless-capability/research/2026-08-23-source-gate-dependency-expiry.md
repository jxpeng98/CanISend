# Source-gate dependency-exception expiry

## Failure

The final Tier 2 command reached dependency governance and stopped at
`RUSTSEC-2024-0320 review is overdue on 2026-08-17`. The machine authority contains 23 entries
whose `review_by` and `expires_on` values are all 2026-08-17, so correcting only the first entry
would not restore the gate.

## Current evidence

- Third-party lock fingerprint remains
  `d2807a35172dc853ad98f7e128f1cbc4737b61aac8cb31f4ddf56c18b05ed903` over 751 packages.
- The repository-pinned `cargo-deny` is 0.19.5. Its Apple Silicon archive SHA-256 matched the
  official release checksum `0cf28e019edb3708ba9755b8c822864ee6d6175d6fc167956972e78ea9ff0be3`.
- A fresh `cargo-deny 0.19.5 check advisories bans licenses sources` completed successfully with
  the existing exact ignores and no new unaccepted advisory.
- Official crates.io lookup reports Typst 0.15.1, Tauri 2.11.5, and Hayagriva 0.10.1 as current;
  the lock already uses those versions.
- `yaml-rust 0.4.5`, `quick-xml 0.38.4`, and `paste 1.0.15` remain transitive through current Typst;
  the GTK3 set remains in Tauri's Linux GUI graph while CanISend publishes Linux CLI only.

## Boundary and recommendation

This is an existing release-integrity blocker, not a regression from `M3-HEADLESS-001`. Do not
silently extend dates inside the headless product PR. Create a separately reviewed P0 dependency
assurance task that either removes an exception through a real dependency/input change or records
a fresh maintainer reachability review, policy dates within the existing 14/30-day limits, the
updated assurance guide and dated body-free evidence. Then rerun the source gate on the unchanged
headless product diff.
