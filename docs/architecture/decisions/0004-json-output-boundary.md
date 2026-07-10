# ADR-004: Keep Machine Output On Stdout And Diagnostics On Stderr

**Status:** Accepted

**Date:** 2026-07-10

## Context

Agents cannot reliably parse terminal tables, Rich formatting, warnings mixed with results, or output that changes with
terminal width. Existing users still depend on readable text commands.

## Decision

- Existing commands default to `--format text` during the alpha lifecycle.
- Supported Phase 1 operations accept `--format json`.
- JSON mode writes exactly one UTF-8 JSON object plus one trailing newline to stdout.
- JSON mode contains no ANSI escapes and is independent of terminal width and locale.
- Human diagnostics, when needed, go to stderr and receive the same privacy redaction as stdout.
- Text and JSON presenters consume the same typed service result.
- Known failures after argument parsing produce one JSON error envelope.
- Typer argument and usage errors remain conventional text with exit status 2 in Phase 1.

## Consequences

- Host agents no longer parse human success sentences or tables for supported operations.
- Existing text workflows remain compatible.
- Tests must cover stdout and stderr leak prevention independently.
- Commands not listed in Phase 1 remain text-only until their service contracts are designed.

## Rejected Alternatives

- Change every command to JSON by default: rejected as an avoidable compatibility break.
- Mix warnings into JSON stdout as prose: rejected because it invalidates machine parsing.
- Maintain separate text and JSON business logic: rejected because the results would drift.

## Revisit When

Revisit before making JSON the default or extending JSON handling to pre-parse Typer usage errors.
