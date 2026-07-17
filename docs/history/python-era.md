# Python-Era Archive

## Final active baseline

The final active Python implementation of CanISend is preserved by the annotated Git tag:

```text
archive/python-v0.6.0b1-final
```

The tag points to:

```text
18ab40815fa7976e2ce4ab9ef91e3f4826689af3
```

That commit contains the Stage 5 workflow implementation and the final Python 3.12 fast/full test architecture. The
earlier public beta tag `v0.6.0b1` points to an older commit and is not the complete pre-rebuild source baseline.

## Historical source locations

At the archival tag, the Python product is located in:

- `src/canisend/`
- `tests/`
- `scripts/`
- `pyproject.toml`
- `uv.lock`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

The historical product and stage roadmaps remain under `docs/superpowers/plans/`. Accepted Python-era architecture
decisions remain under `docs/architecture/decisions/` at the archival tag.

## Support boundary

The Rust-native implementation is a greenfield replacement. It does not promise to:

- Import a Python-era workspace.
- Preserve Python-era CLI arguments or output bytes.
- Implement `canisend.agent/v1`.
- Run the Python package or its Pytest suite.
- Publish future releases through PyPI.

Users who need to inspect or run the historical implementation must explicitly check out the archive tag. New work
belongs on `rewrite/rust-native` and follows the Rust-native roadmap.

## Recovery commands

Read the historical tree without changing the current branch:

```text
git show archive/python-v0.6.0b1-final:README.md
git ls-tree -r --name-only archive/python-v0.6.0b1-final
```

Create a separate historical worktree when execution is genuinely required:

```text
git worktree add ../CanISend-python-archive archive/python-v0.6.0b1-final
```

The Rust branch must not use the archived implementation as a runtime dependency.
