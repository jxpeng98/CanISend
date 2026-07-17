# CanISend Agent Protocol v2 for Codex

Run `canisend agent capabilities --json` before selecting work. Treat only `available` capabilities as executable.
Use task descriptors for bounded reads and candidate schemas; never edit `.canisend/` internal state. Private bodies
must stay within the declared task scope, and provider-bound transmission requires explicit consent.
