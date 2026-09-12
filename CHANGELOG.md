# Changelog

Notable changes, per milestone. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

There are no version numbers yet: Graft is unreleased, and both the name and
the versioning scheme are decided in M8.

## [Unreleased]

### M1 — The graph (in progress)

Nothing user-visible yet. What exists so far is the ground the graph is
tested on.

#### Added

- **A scripted fixture corpus** for the fast tier: one small repository per
  history and ref shape, built by `git fast-import` with a fixed identity and
  clock, so the same shape has the same object ids on every machine. Built
  once into `target/fixtures/` and cached under a digest of the generator, so
  changing the generator rebuilds and nothing goes stale.
- **A heavy test tier**, off by default and run only at milestone reviews. It
  builds a **100,000-commit** repository (2 s, 35 MB) — and the same history
  with a commit-graph, as a separate fixture, because ADR-0008 makes
  first-paint order depend on having one — and raises `proptest` from 64 cases
  to 1,000. `cargo test` still runs zero of it: heavy tests are `#[ignore]`d
  *and* behind the `heavy-tests` feature, so a default build does not compile
  them.
- **A peak-memory measurement** (`getrusage`, ADR-0015) for the 300 MB gate
  PROMPT.md §7.1 asks for. The measurement is here and unit-tested; the
  assertion that uses it is still to come.
- **`scripts/clone-git-git.sh`**, which caches `git/git` outside the
  repository for manual testing — 85,649 commits in 42 MB, because
  `--filter=tree:0` leaves the trees on the server. Run by hand; no test ever
  touches it.
- **[`TESTING.md`](TESTING.md)**: the three tiers and their exact commands,
  what each costs, how to build and delete the fixtures, and the manual
  terminal checklist with its pass conditions.

### M0 — Foundations (2026-09-12)

The first thing a user can run. Graft opens a repository, says what it found,
and gets out of the way cleanly. Nothing here writes to a repository.

#### Added

- **`graft [<path>]`** opens the repository at `<path>`, or the current
  directory when no path is given. Discovery works from a subdirectory, and
  handles bare repositories, linked worktrees and submodule working
  directories.
- **A status screen** showing the repository's working directory, where `HEAD`
  points (branch, detached commit, or a branch with no commits yet), the
  rendering tier and cell size, and the theme and colour depth. Each of the
  last two says where its answer came from — detected, reported, assumed or
  defaulted — rather than presenting a guess as a fact.
- **`q` quits**, leaving the terminal as it was: alternate screen left, mouse,
  paste, focus and keyboard modes popped in reverse order, cursor shown.
- **Clean exits on the ways a terminal program usually dies.** `SIGINT`,
  `SIGTERM` and `SIGHUP` restore the terminal first and then die of the signal,
  so `$?` stays 128 + N. A panic restores the terminal, writes a crash report
  under `${XDG_STATE_HOME:-~/.local/state}/graft/`, and prints where it went.
- **Problems get a line instead of being hidden**: a system `git` older than
  2.38 or missing from `PATH`, an unknown key in the config file, and a `HEAD`
  that could not be read.
- **A config file** at `${XDG_CONFIG_HOME:-~/.config}/graft/config.toml`, with
  `ui.theme` (`auto`, `dark`, `light`, `high-contrast`), `graph.renderer`
  (`auto`, `pixel`, `unicode`, `ascii`) and `ui.icons` (`plain`, `nerd`).
  Having no config file is normal and silent. An unknown key is a warning on
  screen; an unknown *value* is a fatal error naming the file, line and column.
  No setting ever silently falls back to a default.
- **`NO_COLOR` is honoured**: no colour is emitted, and the theme line says
  why.
- **`--help` / `-h`** print usage and exit 0. An unrecognised option is
  refused with the usage text rather than ignored.
- **Terminal-capability detection**: the Kitty graphics protocol, iTerm2
  images, Sixel, cell size, background colour, colour depth and tmux status,
  each with a per-query fallback and a 200 ms budget (500 ms over SSH). Its
  full report is runnable on its own with
  `cargo run -p graft --example capability_probe`.
- **Three themes** — dark, light and high-contrast — each mapped down to
  256-colour and 16-colour terminals, plus a fixed 12-hue lane palette chosen
  for colour-vision deficiency.
- **Everything a repository says is sanitised before it is drawn**, so a branch
  name, path or commit message carrying escape sequences, bidi overrides or
  invisible characters renders as inert text.

#### Not in this milestone

No graph, no diffs, no file list, no staging, no writes of any kind. See
[`KNOWN_GAPS.md`](KNOWN_GAPS.md) for the full list, including the parts of M0
that have not yet been watched running in a real terminal emulator.
