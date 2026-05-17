# git-graph — Python version

A single-file Python script. Uses [PEP 723](https://peps.python.org/pep-0723/) inline metadata so `uv` knows what Python to run it with — no `pyproject.toml`, no venv, no package install.

## Install

Requires [`uv`](https://docs.astral.sh/uv/getting-started/installation/).

```bash
curl -fsSL https://raw.githubusercontent.com/rafa-rrayes/git-graph/master/python/git-graph \
  -o ~/.local/bin/git-graph
chmod +x ~/.local/bin/git-graph
```

Make sure `~/.local/bin` is on your `PATH`.

## How it works

The shebang:

```python
#!/usr/bin/env -S uv run --script --quiet
```

…tells the OS to execute the file with `uv run --script`. `uv` reads the PEP 723 header at the top of the file:

```python
# /// script
# requires-python = ">=3.10"
# ///
```

…and runs the script in an ephemeral environment with the right Python. If the script needed third-party packages, you'd just add them under a `dependencies = [...]` line in the header and `uv` would resolve, cache, and use them. No extra ceremony.

## Usage

```bash
git-graph              # last 3 commits
git-graph -n 10        # last 10 commits
git graph              # works via git's `git-*` subcommand discovery
```
