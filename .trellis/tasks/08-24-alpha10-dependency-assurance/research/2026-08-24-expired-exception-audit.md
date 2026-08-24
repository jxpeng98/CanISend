# Expired dependency-exception audit — 2026-08-24

## Blocker

`cargo run -p xtask --locked -- release check` stops at the first expired entry:
`RUSTSEC-2024-0320 review is overdue on 2026-08-17`. All 23 policy entries share that review and
expiry date. No policy date was changed during this audit.

## Lock and tool evidence

- Third-party lock fingerprint:
  `d2807a35172dc853ad98f7e128f1cbc4737b61aac8cb31f4ddf56c18b05ed903`.
- Third-party packages: 751.
- Exact CI tool: `cargo-deny 0.19.5`.
- Official Apple Silicon archive checksum verified as
  `0cf28e019edb3708ba9755b8c822864ee6d6175d6fc167956972e78ea9ff0be3`.
- Fresh `cargo-deny 0.19.5 check advisories bans licenses sources`: exit 0, with no new
  unaccepted advisory.

## Reachability groups

| Group | IDs | Current boundary | Permanent removal |
|---|---:|---|---|
| Typst parser/font/macro paths | 7 | Fixed, bounded generated source; embedded verified fonts; no YAML, serialized input, bibliography, CSL, or XML input | Upgrade the relevant Typst paths or block before exposing those inputs |
| Tauri Linux GTK compile graph | 11 | Linux public artifacts are CLI-only; Linux GUI remains outside the 1.0 support line | Remove when Tauri drops GTK3 or before publishing Linux GUI |
| Tauri `urlpattern` rust-unic path | 5 | Only checked-in capabilities and application patterns are accepted | Remove when Tauri drops rust-unic or before accepting user patterns |

The two vulnerability entries, `RUSTSEC-2026-0194` and `RUSTSEC-2026-0195`, remain in the first
group. `crates/canisend-io/src/render.rs` contains no `bibliography(` or `publication(` invocation.
The embedded CV template contains exactly one `publication(` occurrence: its declaration-only
helper. `xtask` enforces both facts.

## Upgrade feasibility

- `typst 0.15.1`, `hayagriva 0.10.1`, and `tauri 2.11.5` are already the latest published direct
  roots used by this lock.
- `two-face 0.5.2` is newer, but the affected version is selected by Typst; changing it is not an
  isolated direct upgrade.
- A dependency replacement would widen this P0 task into renderer or desktop-platform migration
  and would not be the smallest safe Alpha.10 unblock.

## Maintainer decision

The maintainer accepted reauthorization of only the unchanged 23 entries for one 14-day window:
`reviewed_on=2026-08-24`, `review_by=expires_on=2026-09-07`. Any new advisory, lock drift,
changed reachability, or expanded input/platform boundary must block instead of inheriting this
decision.
