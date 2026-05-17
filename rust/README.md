# git-graph — Rust version

A small Rust crate that compiles to a single static binary with no runtime dependency beyond `git` itself.

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
- [`terminal_size`](https://crates.io/crates/terminal_size) — detect terminal width

No regex, no async, no extras. ANSI colors are emitted as raw escape codes.

## Usage

See the [top-level README](../README.md#usage) — output is byte-identical to the Python implementation.
