# git-graph — Rust version

A small Rust crate (~150 lines) that compiles to a single static binary with no runtime dependency beyond `git` itself.

## Install

### From crates.io (once published)

```bash
cargo install git-graph
```

### From git

```bash
cargo install --git https://github.com/rafa-rrayes/git-graph git-graph
```

### From a local clone

```bash
git clone https://github.com/rafa-rrayes/git-graph
cd git-graph/rust
cargo install --path .
```

`cargo install` drops the binary in `~/.cargo/bin/` (which should already be on your `PATH` if you installed Rust via `rustup`).

### Pre-built binaries

See the [Releases page](https://github.com/rafa-rrayes/git-graph/releases) for macOS (Intel + ARM) and Linux binaries. Release archives are built by `.github/workflows/release.yml` on every `v*` tag push.

## Build from source

```bash
cd rust
cargo build --release
./target/release/git-graph
```

## Dependencies

- [`clap`](https://crates.io/crates/clap) — argument parsing
- [`owo-colors`](https://crates.io/crates/owo-colors) — ANSI colors

No regex, no async, no extras.

## Usage

```bash
git-graph              # last 3 commits
git-graph -n 10        # last 10 commits
git graph              # works via git's `git-*` subcommand discovery
```
