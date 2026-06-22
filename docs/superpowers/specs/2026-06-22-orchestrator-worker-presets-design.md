# Orchestrator Worker Presets Design

## Goal

Improve CanISend orchestration so users can coordinate Codex, Claude, and Antigravity/agy workers
without hard-coding one Claude CLI invocation style.

## Approach

The orchestrator keeps the existing custom `command` contract and adds worker presets through a new
`kind` field. Supported kinds are `codex`, `claude`, `antigravity`, `agy`, and `custom`. `antigravity`
and `agy` resolve to the same default worker behavior.

Preset defaults provide a command, prompt delivery mode, and friendly validation behavior. Explicit
worker fields still win, so users can override `command` or `prompt_mode` for their installed CLI
versions.

## Worker Configuration

Existing YAML remains valid:

```yaml
workers:
  custom-reviewer:
    command: "my-stdin-compatible-model-cli"
```

Preset-based YAML is also valid:

```yaml
workers:
  reviewer:
    kind: claude
    prompt_mode: stdin
```

Antigravity can use either spelling:

```yaml
workers:
  agy-reviewer:
    kind: agy
    command: "agy --print"
```

## Prompt Modes

The first implementation supports:

- `stdin`: pass the generated task prompt to the worker process stdin.
- `arg`: append the generated task prompt as the final command-line argument.
- `none`: write `prompt.md` into the task artifact directory but do not pass prompt text to the
  worker process.

`stdin` remains the default for compatibility with existing custom commands.

## Validation

Unknown worker kinds fail plan loading with a clear error. `custom` workers, and workers without a
`kind`, must define `command`. Preset workers may omit `command`; unavailable executables are reported
when the run is launched, with the worker name included in the error.

## Testing

Tests should cover preset loading, Antigravity aliases, command override behavior, prompt-mode
execution behavior, and unknown-kind validation. Existing command-string behavior must remain green.
