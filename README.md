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

**M0 Foundations and M1 The graph are built.** Graft opens a repository and
draws its commit graph — lanes, merges, refs, relative dates — across three
rendering tiers, with a fixture corpus and a heavy tier proving it holds up
at 1M commits. It still does not show a diff or a file list, and it never
writes to a repository. **M1's manual checkpoint (`TESTING.md`, `scripts/
m1-checkpoint.sh`) has not been run by the user yet** — see "What has
actually been watched" below before trusting any of this by eye.

### What M1 actually does

- Draws the commit graph for the current branch and its ancestry: lanes,
  merge curves, refs (branches, tags, `HEAD`), relative commit dates, and the
  commit's first line — across three rendering tiers (pixel/Kitty, Unicode,
  ASCII), auto-selected from M0's capability probe or forced by
  `graph.renderer`.
- Lane assignment keeps a branch in one column end to end; the lane cap
  follows terminal width (8/10/12 lanes under 100/100–159/160+ columns) and
  is a priority-ordered budget, not a hard veto (ADR-0022, ADR-0025) — lanes
  beyond the cap fold behind a `⋮` rail, with the exact count in the header
  (`+N lanes`).
- `j`/`k` scroll, `1` toggles first-parent-only view, `Enter` reveals a
  truncated row's full subject in the footer (no tooltip — ADR-0023), `y`
  copies the selected commit's SHA.
- Pixel tier: ancestry-path emphasis under focus is alpha only, never a
  stroke-width change (ADR-0024), so lane geometry never shifts as you move
  the cursor.
- Holds a 1M-commit fixture inside PROMPT.md §7.1's frame and memory budgets
  (`docs/reviews/`, `KNOWN_GAPS.md`'s 56d/64 entries have the numbers); first
  paint is measured warm, with one cold check (`sudo purge`) left for the
  user (ADR-0027, TESTING.md).
- `cargo run -p graft --example gallery` walks 24 named fixture shapes with a
  banner per stop; `scripts/m1-checkpoint.sh <scenario>` opens one scenario
  (`git-git`, `criss-cross`, `foxtrot`, `octopus`, or `list`) directly,
  cloning `git/git` or building the fixture corpus as needed and exiting
  non-zero with a clear message if it can't.

### What M2 actually does

- `Enter` on a graph row opens the detail pane: a file tree and a unified
  diff. `v` switches to side-by-side, `W` tints changed words, `w` ignores
  whitespace, `[`/`]` shrink/grow diff context (0–20), `Tab`/`Shift+Tab`
  cycle focus Graph → Files → Diff, `Esc` closes the pane.
- `x` on a row marks it as a compare base; the pane then follows the cursor
  showing that commit against the base instead of the commit alone.
- `.` (or right-click) opens a context-actions menu on the row: `Explain`
  (marked rows only), `Copy SHA`, `Compare with this`, `Open details`.
- `M` opens the Map view — one bar per local branch, bucketed by date, with
  tag and Mistake Radar marks. `O` toggles an overview strip along the
  graph's edge showing merges, radar findings, search hits and the viewport.
- Mistake Radar flags a marked row with `⚠` (warning) or `ⓘ` (info) in a
  dedicated column; `.` → `Explain` on a marked row shows why, with the
  relevant commits highlighted and the rest of the graph faded.
- `[radar]` config: `foxtrot`, `backward`, `accidental-pull`, `evil`,
  `duplicate`, `force-pushed`, `risky-state`, each left at its PROMPT.md
  §1.4 default or forced to `"off"`/`"info"`/`"warning"`. `[diff]` config:
  `context` (integer), `ignore_whitespace`, `word_tint` (booleans) — the
  same options `w`/`[`/`]`/`W` change for the running session.

### What M3 actually does

M3 promises that **you cannot lose work**: every write is journaled with a
backup and `u` takes it back byte for byte.

- **The changes pane**: `k` from the top row selects `Uncommitted changes`,
  `Enter` opens it — `STAGED`, `UNSTAGED`, `UNTRACKED` sections with the
  files under each. `Space` (or `s`) stages or unstages the file under the
  cursor (its section decides which); `Enter` moves into the diff, where
  `Space` on a hunk heading stages that hunk and on a `+`/`-` line that one
  line; `Ctrl+A` stages everything; `d` discards a file, hunk or line after a
  confirm box that opens on the safe answer.
- **Commit and amend**: `c` opens the editor, `a` opens it prefilled from
  `HEAD`. `Tab` moves Subject → Body → options → `[ Commit ]`; `Enter` on the
  button commits (`Ctrl+Enter` too, where the terminal can tell it apart).
  Refusals (`nothing staged`, `subject empty`) show on the title row.
- **Stash**: `S` lists the stashes; `s` saves (message, `Tab` toggles
  `all incl. untracked` / `staged only`), `Enter` applies, `p` pops, `d`
  drops after a confirm.
- **Undo and redo**: `u` undoes the last write, `U` redoes it — for stage,
  unstage, discard, commit, amend, stash save/apply/pop/drop alike. If the
  repository changed since (an edit, a stash from the shell), a box says
  what drifted and asks before undoing. `J` opens the journal: every write,
  its backup, `← u` on the entry `u` would take back. An operation a killed
  process left half done is announced in the footer on the next launch, and
  `u` restores it.
- **Refresh on focus**: the working-tree status is re-read when the
  terminal regains focus. No file watcher yet.
- **Not in M3**: hunk staging inside untracked files, `i` (.gitignore),
  message history, stashing selected files, branch-from-stash, a file
  watcher, clearing old backups, committing during a merge/rebase (refused,
  names the command). All in `KNOWN_GAPS.md`.

**From today, use it and keep score.** `TESTING.md`'s "Times I left" takes
one line per reach for the shell; `docs/field-notes.md` takes what real use
turned up. Those two lists, not the test suite, measure the promise.

### What M4 actually does

M4 promises that **you never leave Graft to fetch, pull or push — not for a
password, a rejection or a hang**.

- **Fetch**: `f` fetches every remote with prune. Progress from git's own
  `--progress` output runs in the footer (`fetch all remotes · receiving
  objects 45% · esc cancels`); `fetched` when it lands. Fetches are
  journaled but `u` passes over them — nothing of yours moved.
- **Pull**: `p` fast-forwards the current branch from its upstream. Undoable:
  `u` puts the branch and the working tree back, and leaves the tracking ref
  where the fetch put it; `U` pulls again. A diverged branch is refused in
  words that name the shell command (`pull stopped: main and origin/main
  diverged — merge or rebase (M5); shell: git pull --rebase`); an
  uncommitted edit to a file the pull would change is refused before
  anything moves (`pull refused: would overwrite <file> (uncommitted)`). No
  upstream: `main has no upstream — P pushes and sets one`.
- **Push**: `P` opens a box first — `Push main → origin/main?`, the commit
  count, `sets upstream` when it will, `can't be undone from here — origin
  keeps what you send`, and the exact `git push` line. `Enter` on `[ Push ]`
  runs it. A push is journaled but not undoable: `u` afterwards undoes your
  commit **locally** and says `· origin/main still has it`; the remote is
  only ever moved by `P`.
- **A rejected push** stays on the footer until `Esc`: `push rejected:
  origin/main has commits you don't · p to pull first, or P to force with
  lease`. The next `P` offers the lease box — `Force-push main →
  origin/main?`, the tip it expects on the remote, `rewrites history for
  everyone who has …`, `--force-with-lease=…` in the command. If the remote
  moved again since your fetch it is refused: `push refused: origin/main
  moved since your last fetch (stale lease) · f to fetch, then look`. A plain
  `--force` never runs; a lease push to any branch but the upstream of `HEAD`
  is refused.
- **Credentials are asked in the app**: git's password question opens a
  masked field (`•`), a username question an unmasked one; `Enter` answers,
  `Esc` cancels the whole operation. Graft never stores what you type and
  never puts it in a log, the journal or a capture; your own credential
  helpers keep working. A wrong password is a line, not a hang:
  `authentication failed for http://… · P to try again`.
- **`Esc` cancels** a running fetch, pull or push (`… · cancelling`, then
  `… cancelled`, the journal entry says so). It does **not** cancel `u`/`U`:
  `undo cannot be stopped part-way — finishing…`.
- **Not in M4**: clone, adding/renaming/removing remotes, pull with merge or
  rebase, fetching one remote, a stall notice, SSH (the askpass wiring is
  set but unverified). All in `KNOWN_GAPS.md`.

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

**The same is true of the graph.** Everything above about lanes, merges,
folding and tier behaviour is proven by unit, snapshot and bare-pty tests
against scripted replies — never by a human looking at a real Ghostty, Kitty,
WezTerm, iTerm2, Terminal.app, Alacritty, Zed or VS Code terminal, with or
without tmux or SSH in the way. `TESTING.md`'s manual matrix and
`scripts/m1-checkpoint.sh` exist to make that check quick; neither has been
run yet. Every "reads well" and by-eye result in `TESTING.md` reads **not
run** until you run it.

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
cargo run -p graft -- ~/code/some-repo
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
cargo run -p graft -- /tmp        # graft: /tmp is not inside a git repository   (exit 1)
cargo run -p graft -- --help      # usage, exit 0
NO_COLOR=1 cargo run -p graft -- .   # same screen, no colour; the theme line says so
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
# max_lanes = 10     # unset (default): the cap follows the terminal's width —
                      # 8 lanes under 100 columns, 10 at 100–159, 12 at 160+.
                      # A number larger than the screen can hold is reduced to
                      # what fits; 1 or 0 is refused, there being no room left
                      # for the `+N` marker.
```

An unknown key is a warning on the status screen, not an error, so a file
written for a later version still starts today's binary. An unknown *value* is
a fatal error naming the line and column.

## Repository map

| Path | What it is |
|---|---|
| `crates/` | The workspace: `graft-core`, `graft-graph`, `graft-git`, `graft-ops`, `graft-term`, `graft-ui`, `graft` (the binary), and `graft-fixtures` (test-only) |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crate map and the allowed dependency edges, enforced by a test |
| [`TESTING.md`](TESTING.md) | The three test tiers, the fixtures, and the manual terminal checklist |
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
must not melt a laptop (PROMPT.md §7.1). It is 307 tests and about 4 seconds
warm. Build parallelism is pinned in `.cargo/config.toml`.

The **heavy tier** exists and runs only at milestone reviews. It builds a
100,000-commit repository (2 s, 35 MB in `target/fixtures/`) and raises
`proptest` from 64 cases to 1,000. Two commands, because `--ignored` runs only
the ignored tests:

```sh
cargo test --features heavy-tests -- --ignored --test-threads=2
cargo test --features heavy-tests -- --test-threads=2
```

For a real, merge-heavy history to look at, `scripts/clone-git-git.sh` caches
`git/git` outside the repository — 85,649 commits for 42 MB, because
`--filter=tree:0` leaves the trees on the server. It is run by hand; no test
ever touches it.

[`TESTING.md`](TESTING.md) has all three tiers, what each one costs, how to
delete the fixtures, and the manual terminal checklist.

## Licence

MIT.
