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

You need a Rust toolchain (stable, version 1.88 or newer) and `git` 2.38 or
newer. macOS and Linux are supported.

```sh
git clone https://github.com/Ali-Maken/mergewright
cd mergewright
cargo install --locked --path crates/mergewright
```

That installs the `mwr` command. After the first release you will also be able
to install with Homebrew or a one-line installer; those do not work yet.

## Your first five minutes

```sh
mwr                # open the repository you are standing in
mwr ../other-repo  # or any other one
```

Not inside a repository? Run `mwr` anyway. It shows recent repositories and lets
you open, create or clone one. Press **`t`** there for a practice tour in a
throwaway repository where nothing you do can hurt.

Once a repository is open:

| Key | What it does |
|---|---|
| `?` | Help for the screen you are on |
| `⌃P` | Search every action by name |
| `j` / `k` | Move down / up |
| `⏎` | Open the selected thing |
| `space` | Stage or unstage a change |
| `c` | Commit |
| `C` | Switch branch |
| `b` | New branch from the selected commit |
| `m` | Merge the selected branch into yours |
| `r` | Rebase yours onto the selection |
| `f` / `p` / `P` | Fetch / pull / push |
| `S` | Stash |
| `u` | Undo the last change |
| `.` | More actions for the selected row |
| `q` | Quit |

Every key can be changed in the config file, and `?` always shows the keys that
are active.

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

## Settings

Everything is optional. The file is `~/.config/mergewright/config.toml` (or
`$XDG_CONFIG_HOME/mergewright/config.toml`). A small example:

```toml
[ui]
theme = "auto"          # auto | dark | light | high-contrast

[graph]
renderer = "auto"       # auto | pixel | unicode | ascii
default_branch = "main" # unset: detected

[keys.app]
refresh = ["ctrl+r", "f5"]   # rebind any key by its name
```

Every setting and every key is listed in the manual:

```sh
man ./docs/man/mwr.1
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
