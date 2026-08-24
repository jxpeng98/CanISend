# Headless capability design

Reuse the existing adapter boundary:

```text
direct CLI --------\
                    -> canisend-app -> Core/Store/IO authority
persistent MCP ----/

desktop App: optional reader/operator of the same authority
```

The only expected product addition is a CLI `project|global` value enum and `--scope` on existing
host setup/status/remove leaves, passed into the existing application-facade requests. Project is
the default. Global root discovery and missing-home failure remain owned by the application facade.

Extend the existing packaged Agent v4 headless smoke instead of adding another harness. Fix only
gaps that smoke proves in Workspace initialization, canonical resources, mixed-Pack workflow,
guarded MCP mutations, export/recovery, or reopen. Skills remain guidance; no adapter writes
`.canisend` directly and approval tokens remain process-bound.

Rollback is the bounded product PR. Public Alpha.9 and release metadata remain unchanged.
