# Graft

A terminal Git client, aimed at the daily work: read history, stage, commit,
fetch, push, merge, stash, undo. The plan is in [`PROMPT.md`](PROMPT.md), the
design in [`DESIGN.md`](DESIGN.md), the milestone order in
[`PROGRESS.md`](PROGRESS.md).

> **"Graft" is a working codename, not the name.** It is taken by another Git
> GUI. The real name is chosen in M8; nothing is published, registered or
> tagged under this one.

**macOS and Linux only** (ADR-0005). No Windows support is planned for v1.

## Where this is

**M0 Foundations is the only finished milestone.** Graft opens a repository,
shows a handful of facts about it on one screen, and quits cleanly. It does
not draw a graph, a diff or a file list, and it never writes to a repository.
M1 builds the graph.

### What M0 actually does

- Opens a repository from a path or the current directory — including from a
  subdirectory, a bare repository, a linked worktree or a submodule's
  working directory.
- Shows one status screen: the repository's working directory, `HEAD` (branch
  name, detached id, or "unborn"), the rendering tier and cell size that
  capability detection settled on, and the theme and colour depth.
- Reports rather than hides problems: a system `git` older than 2.38, an
  unknown key in the config file, and a `HEAD` that could not be read each
  get their own line on that screen.
- Quits on `q` with the terminal exactly as it was — alternate screen left,
  every mode popped, cursor back. Same after `SIGINT`, `SIGTERM` or `SIGHUP`,
  and same after a panic, which also writes a crash report.
- Exits 1 with one plain line when the path is not inside a repository.
- Honours `NO_COLOR`.
- Reads `${XDG_CONFIG_HOME:-~/.config}/graft/config.toml`. A bad value is an
  error with file, line and column — never a silent default.

### What M0 does not do

Everything else — no graph, no diffs, no file list, no staging, no writes.
[`KNOWN_GAPS.md`](KNOWN_GAPS.md) is the full list.

### What has actually been watched, and what has not

Every behaviour above has been driven under a **bare pty** — a pseudo-terminal
with no emulator behind it — and covered by unit, snapshot and property tests.
That proves the bytes Graft writes and the decisions it makes from scripted
replies. It does not prove a pixel.

**No real terminal emulator has ever answered Graft's capability probe, and
nobody has looked at the status screen in one.** Every run so far reports the
silent-terminal fallback, so the tier, cell size, theme and colour depth on
that screen are what Graft assumes when nothing answers — not what your
terminal said. Running the steps below, and the probe example, in a terminal
you actually use is the check that has not been done.

## Build and run

Needs a stable Rust toolchain (edition 2024, MSRV **1.88**) and `git` **2.38**
or newer on `PATH`.

```sh
git clone <this repo> graft && cd graft
cargo build
```

## Demo: two minutes

Run it on any repository you have:

```sh
cargo run -- ~/code/some-repo
```

You should get a screen like this, and nothing else:

```
Graft

Repository  /Users/you/code/some-repo
HEAD        main
Rendering   unicode, detected, cell 9x18 (reported)
Theme       dark, from the terminal background, truecolor

q  quit
```

- `Rendering` and `Theme` say where each answer came from. The cell size and
  the theme carry their real provenance: `reported` means the terminal said
  so, `assumed` or `terminal background unknown` means it did not and Graft
  fell back. In a bare pty — which is the only place Graft has run so far —
  those two read `cell 8x16 (assumed)` and `dark, terminal background
  unknown`.
- Careful with the word `detected` on the `Rendering` line: today it only
  means "not overridden in the config file", and it is printed even when the
  capability probe timed out with no answer at all. That is a known wart, on
  the list in [`KNOWN_GAPS.md`](KNOWN_GAPS.md).
- Press **`q`**. You are back at your shell prompt, colours and cursor
  unchanged, exit status `0`.

Then try the two other paths:

```sh
cargo run -- /tmp        # graft: /tmp is not inside a git repository   (exit 1)
cargo run -- --help      # usage, exit 0
NO_COLOR=1 cargo run -- .   # same screen, no colour; the theme line says so
```

With no path, Graft opens the current directory.

### Asking the terminal what it can do

Capability detection has its own runnable report, outside the alternate
screen:

```sh
cargo run -p graft --example capability_probe
```

It prints every answer the terminal gave — graphics protocols, cell size,
background colour, colour depth, tmux status — and which tier that adds up to.
Running it in your terminals is the most useful thing you can do with M0:
until a real emulator has answered, the tier Graft picks is a guess made from
scripted test fixtures.

## Configuring it

`${XDG_CONFIG_HOME:-~/.config}/graft/config.toml`. No file at all is the normal
case. Every key M0 knows:

```toml
[ui]
theme = "auto"      # auto | dark | light | high-contrast
icons = "plain"     # plain | nerd        (nothing reads this until M1)

[graph]
renderer = "auto"   # auto | pixel | unicode | ascii
```

An unknown key is a warning on the status screen, not an error, so a file
written for a later version still starts today's binary. An unknown *value* is
a fatal error naming the line and column.

## Repository map

| Path | What it is |
|---|---|
| `crates/` | The workspace: `graft-core`, `graft-graph`, `graft-git`, `graft-ops`, `graft-term`, `graft-ui`, and `graft` (the binary) |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crate map and the allowed dependency edges, enforced by a test |
| [`DESIGN.md`](DESIGN.md) | The approved design |
| [`CONTEXT.md`](CONTEXT.md) | The domain glossary; these terms are used everywhere |
| [`docs/adr/`](docs/adr/) | Decision records |
| [`KNOWN_GAPS.md`](KNOWN_GAPS.md) | Everything unfinished, stubbed or deferred |
| [`CHANGELOG.md`](CHANGELOG.md) | What changed, per milestone |
| `prototypes/` | Throwaway spikes, outside the workspace and outside the gates |

## Working on it

Three gates, and code is not done until all three pass:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` is the **fast tier** only — it must stay under three minutes and
must not melt a laptop (PROMPT.md §7.1). The heavy tier, fuzzing and benchmarks
run at milestone reviews, never here. Build parallelism is pinned in
`.cargo/config.toml`.

## Licence

MIT.
