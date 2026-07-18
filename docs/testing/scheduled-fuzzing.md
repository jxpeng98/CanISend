# Scheduled Rust Fuzzing

CanISend keeps fuzzing outside the fast pull-request and push test path. The `scheduled-fuzz` workflow runs every
Monday and can also be dispatched manually. It pins `nightly-2026-07-01`, `cargo-fuzz` 0.13.2, and
`libfuzzer-sys` 0.4.13 rather than using an unbounded moving toolchain.

## Targets

- `structured_inputs` parses bounded arbitrary JSON into the public task, discovery, plan, draft, review, and review
  disposition contract types.
- `intake_parsers` exercises UTF-8 normalization, HTML extraction, CSV/JSON/host discovery parsing, and rendered-PDF
  validation.
- `pdf_extract` sends bounded arbitrary bytes through the production PDF text extractor and its encrypted,
  malformed, text-unavailable, page, and time-budget error boundaries.

Each target receives five minutes and a 15-second per-input timeout. Targets run in parallel on Linux with a
20-minute job limit. A crash, panic, timeout, or memory failure makes the job fail and uploads only the generated
random reproducer for seven days; fuzz corpora contain no real advert, profile, application, provider, or credential
data.

## Local reproduction

Install the pinned nightly toolchain and cargo-fuzz version, then run the exact failing target:

```console
rustup toolchain install nightly-2026-07-01 --profile minimal
cargo +nightly-2026-07-01 install cargo-fuzz --version 0.13.2 --locked
cargo +nightly-2026-07-01 fuzz run TARGET -- fuzz/artifacts/TARGET/REPRODUCER
```

Do not delete or ignore a reproducible crash to make the schedule green. Add the minimized input to a non-private
regression test, fix the production boundary, run the focused test and fuzz target, and record the resolving commit
and Actions run in the Rust-native notes. A failed scheduled run is a release blocker until reproduced and resolved.
