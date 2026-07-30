#!/bin/sh
set -eu

if [ "${1:-}" = "--version" ]; then
  printf '%s\n' "codex-cli 0.0.0-canisend-fixture"
  exit 0
fi

prompt="$(/bin/cat)"
case "$prompt" in
  *"local cancellation fixture."*)
    exec /bin/sleep 30
    ;;
esac

printf '%s\n' "$*" >> "$HOME/.canisend-fake-codex-invocations"
case " $* " in
  *" resume "*)
    printf '%s\n' \
      '{"type":"item.completed","item":{"id":"fixture-resume","type":"agent_message","text":"Fixture resumed turn completed."}}' \
      '{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}'
    ;;
  *)
    printf '%s\n' \
      '{"type":"thread.started","thread_id":"fixture-session-1"}' \
      '{"type":"item.completed","item":{"id":"fixture-first","type":"agent_message","text":"Fixture first turn completed."}}' \
      '{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}'
    ;;
esac
