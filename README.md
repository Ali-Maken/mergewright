# Mergewright

**A Git client that lives in your terminal.** See your history as a picture,
make commits, switch branches, merge, rebase and push, all with a few keys.
Nothing to click, nothing to memorise: the bottom bar always shows which keys
work where you are.

The command is `mwr`.

> **Status:** not released yet. You can build and run it from source today
> (see [Install](#install)). Prebuilt downloads and a Homebrew package come
> with the first release.

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

*A rebase paused half-way. Branches are the lines, commits are the dots, and
the top bar says where you are. (A text capture, so it shows no colour.)*

## Why you might like it

- **See before you do.** Before a merge or a rebase, Mergewright shows you what
  the result will look like. You confirm after you have seen it.
- **Undo.** Press `u` to take back the last change. Everything it changes in
  your repository is written to an undo journal first.
- **Always know the next key.** A bar at the bottom lists the keys that work
  right here. `?` explains the screen you are on.
- **Find anything.** `⌃P` opens a palette: type part of what you want to do and
  pick it.
- **It is just Git.** It works on your normal repositories. There is nothing
  to set up in them first.

## Install

You need `git` 2.38 or newer. macOS and Linux are supported.

**Build it from a checkout (works today).** You also need a stable Rust
toolchain, version 1.88 or newer. This installs the `mwr` command:

```sh
git clone https://github.com/Ali-Maken/mergewright
cd mergewright
cargo install --locked --path crates/mergewright
```

**After the first release** you will also be able to use the lines below.
**None of them work yet.** `<owner>` is the account the release is published
from.

```sh
brew install <owner>/tap/mergewright               # Homebrew tap
curl -LsSf https://github.com/<owner>/mergewright/releases/latest/download/mergewright-installer.sh | sh
```

Then `man ./docs/man/mwr.1` reads the manual: every flag, every key in every
place, every config key, the environment and the files Mergewright writes.

## Your first five minutes

```sh
mwr                # open the repository you are standing in
mwr ../other-repo  # or any other one
```

Not inside a repository? Run `mwr` anyway. It shows recent repositories and lets
you open, create or clone one. Press **`t`** there for a practice tour in a
throwaway repository where nothing you do can hurt.

Once a repository is open, press `?` for the keys that work where you are,
or `⌃P` to search every action by name. The most-used keys are listed under
[Daily keys](#daily-keys) below.

## Good to know

- **Terminals.** The author uses it daily in Zed's terminal and in Ghostty on
  macOS. Kitty, WezTerm and iTerm2 have not been tried by hand yet.
- **Colour.** The automatic tests record the screen as plain text, so colours
  were only checked by eye, and not on every terminal.
- **What it does not do yet:** visual bisect, several repositories in tabs,
  Windows, image diffs, and a few other things. The honest list is in
  [`KNOWN_GAPS.md`](KNOWN_GAPS.md).
- **Its own files.** Inside each repository it keeps `.git/mergewright/` (the
  undo journal lives there) and its own refs under `refs/mergewright/`. In the
  config file it writes one setting, and only if you pick a graph mode from the
  `?` screen.

## Command line

`mwr --help` prints it:

```text
usage: mwr [<path>]

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

`$XDG_CONFIG_HOME/mergewright/config.toml`, or `$HOME/.config/mergewright/config.toml`.
No file is the normal case, and every key is optional. An unknown key is a
warning, so a file written for a later version still starts this one; a value
a key does not take stops Mergewright with the line and column — except
`graph.merged_branches`, whose bad value is a config problem (`?` lists it)
and opens folded. That one key is also the only thing Mergewright writes into
the file: picking a mode under `GRAPH MODE` in `?` on the graph keeps it there
(every other line and comment left as it was). Every key, its
values and its default are in the man page's CONFIGURATION section.

In each repository it opens, Mergewright keeps its files in
`.git/mergewright/` (the undo journal among them) and its own refs under
`refs/mergewright/`.

```toml
[ui]
theme = "auto"          # auto | dark | light | high-contrast
icons = "plain"         # plain | nerd (needs a Nerd Font)
overview = false        # start with the overview strip shown

[graph]
renderer = "auto"       # auto | pixel | unicode | ascii
default_branch = "main" # unset: detected
max_lanes = 10          # unset: 8, 10 or 12 by terminal width
merged_branches = "folded"  # folded | shown (every branch on its own lane)

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

## Contributing

Plans and design live in [`PROMPT.md`](PROMPT.md), [`DESIGN.md`](DESIGN.md) and
[`ARCHITECTURE.md`](ARCHITECTURE.md). Tests and manual checklists are in
[`TESTING.md`](TESTING.md). A change is done when these pass:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo nextest run
cargo test --doc
```

The manual `docs/man/mwr.1` is generated, and a test fails if it is stale.
Regenerate it with:

```sh
cargo run -q -p mergewright -- --generate-man > docs/man/mwr.1
```

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.
