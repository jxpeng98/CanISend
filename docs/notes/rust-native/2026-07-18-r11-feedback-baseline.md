# R11.4 measured-feedback baseline

**Date:** 2026-07-18

**Roadmap item:** R11.4 preparation

**Status:** Public Alpha baseline captured; Beta/RC refresh and Stable publication pending

## Public evidence

At `2026-07-18T08:33:01Z`, the public `v0.7.0-alpha.1` release had 12 assets and each asset had one download. The five
native archives therefore had five downloads in total. These counts include maintainer download/verification and
cannot identify unique users, adoption, retention, or platform demand.

The repository had zero public issues in all states. No issue title or body was copied into the snapshot, and no
private workspace, advert, profile, application, provider, or credential data was queried. Default telemetry remains
disabled.

## Engineering evidence

Two cross-platform qualification defects are recorded separately from user feedback: Windows path normalization in
run `29631149914`, and the CRLF false contract drift first reproduced in failed run `29636580836`. Both inform the
portability candidate in the post-0.7 draft without being mislabeled as user feature demand.

## Stable gate

`xtask release check` validates internal snapshot counts, the public-metadata-only boundary, qualification finding
evidence, and the linked next-roadmap draft. Pre-Stable versions require roadmap status `draft`. A Stable workspace
version requires an RC-stage snapshot and roadmap status `published`, forcing a fresh measurement after Beta/RC
instead of promoting this Alpha baseline unchanged.
