# Graft

> **"Graft" is a working codename, not the name.** It is taken by another Git
> GUI. The release name is decided (ADR-0057) and the rename lands before
> anything is published; nothing is published, registered or tagged under
> this one.

A terminal Git client for the daily work: read history as a graph, stage,
commit, branch, merge, rebase, stash, fetch, pull and push, all from the
keyboard. Before a merge or a rebase it shows you the result, and what it
changes in the repository goes into an undo journal, so `u` takes the last
change back. A bar at the bottom always names the keys that work where you
are. macOS and Linux (ADR-0005).

A rebase stopped half-way, in the Unicode tier (from
[`docs/shots/m8/fork-follows-banner-187x44.txt`](docs/shots/m8/fork-follows-banner-187x44.txt),
cut to 110 columns; a character grid, so it carries no colour):

```text
 ◆ api  ~/code/api · detached c9b786e             REBASING  step 2 of 4 · edit c9b786e b1 · feature-b, forked

╭ still to apply · not commits yet ───────────────────────────────────────────────────────────────────────────
│ pick bb8335d feature a: part 3
│ pick 86eea6e feature a: part 2
╰─────────────────────────────────────────────────────────────────────────────────────────────────────────────
    ●        [feature-c] c1
▸   │ ◆      [HEAD] [feature-b] b1  HEAD · stopped here
    │ │ ●    [feature-a] feature a: part 3
    ●─│─╯    feature a: part 2
    ●─╯      feature a: part 1
  ◇─╯        [main] m1
```

## Install

**None of these work yet: they are for after the first release**, which waits
for the rename (ADR-0057). The commands will carry the release name; `<owner>`
is the account the release is published from.

```sh
brew install <owner>/tap/graft                     # Homebrew tap
curl -LsSf https://github.com/<owner>/graft/releases/latest/download/graft-installer.sh | sh
```

From a checkout of this repository, today (needs stable Rust, MSRV 1.88, and
`git` 2.38 or newer on `PATH`):

```sh
cargo install --locked --path crates/graft
```

Then `man ./docs/man/graft.1` reads the manual: every flag, every key in every
place, every config key, the environment and the files Graft writes.

## First run

```sh
graft              # the repository the current directory is in
graft ../api       # or any other
```

- Outside a repository, Graft shows where you can go: a recent repository,
  open, init, clone, or `t` for a practice tour in a throwaway repository.
- `?` shows the keys for the place you are in.
- `⌃P` opens the command palette: every action, found by name.
- The first few launches say where help is on the status row, until you
  first open the palette.

## Command line

`graft --help` prints it:

```text
usage: graft [<path>]

  <path>    the repository to open (default: the current directory)

  -h, --help       print the usage and exit
  -V, --version    print the version and exit
```

## Daily keys

The first key of each; the man page lists them all, and `[keys]` rebinds
any of them by the name in the last column.

| Key | What it does | Name |
|---|---|---|
| `?` | Help for this place | `app.help` |
| `⌃P` | Command palette | `app.palette` |
| `j` | Move down (`k` up) | `app.down` |
| `⏎` | Open / drill in | `app.open` |
| `space` | Stage / unstage, in the changes | `changes.stage` |
| `c` | Commit | `graph.commit` |
| `C` | Switch branch (branch picker) | `graph.switch` |
| `b` | New branch from the selected commit | `graph.branch` |
| `m` | Merge the selected branch into yours | `graph.merge` |
| `r` | Rebase yours onto the selection | `graph.rebase` |
| `S` | Stash | `graph.stash` |
| `f` | Fetch | `graph.fetch` |
| `p` | Pull | `graph.pull` |
| `P` | Push | `graph.push` |
| `u` | Undo | `app.undo` |
| `.` | Context actions for the row | `app.menu` |
| `q` | Quit | `app.quit` |

## Configuration

`$XDG_CONFIG_HOME/graft/config.toml`, or `$HOME/.config/graft/config.toml`.
No file is the normal case, and every key is optional. An unknown key is a
warning, so a file written for a later version still starts this one; a value
a key does not take stops Graft with the line and column. Every key, its
values and its default are in the man page's CONFIGURATION section.

```toml
[ui]
theme = "auto"          # auto | dark | light | high-contrast
icons = "plain"         # plain | nerd (needs a Nerd Font)
overview = false        # start with the overview strip shown

[graph]
renderer = "auto"       # auto | pixel | unicode | ascii
default_branch = "main" # unset: detected
max_lanes = 10          # unset: 8, 10 or 12 by terminal width

[diff]
context = 3
ignore_whitespace = false
word_tint = true

[radar]                 # Mistake Radar, per kind: default | off | info | warning
foxtrot = "warning"

[keys]
suspend_on_ctrl_z = false   # true: ⌃Z suspends, and undo keeps u

[keys.app]                  # rebind by name: a key, a list of keys, or "none"
refresh = ["ctrl+r", "f5"]

[theme.dark]                # one colour over a built-in theme; lanes are fixed
accent = "#7aa2f7"
```

## What it does not do

Not in this version, and not hinted at anywhere in it:

- Visual bisect.
- A worktree switcher (opening a worktree directly works: it is a repository).
- LFS awareness beyond what your own git filters already do.
- Sparse-checkout awareness.
- Several repositories open at once, in tabs.
- Submodule update and sync (submodules are listed and can be opened).
- Image diffs in the pixel tier.
- Mouse use and remote branch rows in the Map view.
- Pruning old undo backups.
- Windows.

## Terminals

- **Used daily by its user:** Zed's integrated terminal (Unicode tier) and
  Ghostty (pixel tier), on macOS.
- **Tested on every change:** a pseudo-terminal harness drives the real
  binary and records the screen as text. It records no colour, so colour is
  checked by eye only.
- **Not yet run by hand:** Kitty, WezTerm and iTerm2 (the matrix in
  [`TESTING.md`](TESTING.md)).

## Working on it

The plan is [`PROMPT.md`](PROMPT.md), the design [`DESIGN.md`](DESIGN.md),
the crate map [`ARCHITECTURE.md`](ARCHITECTURE.md), the test tiers and the
manual checklists [`TESTING.md`](TESTING.md), what is unfinished
[`KNOWN_GAPS.md`](KNOWN_GAPS.md). Code is done when these pass:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo nextest run
cargo test --doc
```

`docs/man/graft.1` is generated; a test fails when it is stale. Regenerate it
with `cargo run -q -p graft -- --generate-man > docs/man/graft.1`.

## Licence

MIT.
