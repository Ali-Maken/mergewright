# Changelog

Notable changes, per milestone. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

There are no version numbers yet: Graft is unreleased, and both the name and
the versioning scheme are decided in M8.

## [Unreleased]

### M3 — Working tree, undo and stash

You cannot lose work: every write is journaled with a backup and comes back
byte for byte on `u`.

#### Added

- **The changes pane** on the `Uncommitted changes` row (`k` from the top,
  `Enter`): `STAGED` / `UNSTAGED` / `UNTRACKED` sections. `Space`/`s` stage or
  unstage the file under the cursor; in the diff, `Space` stages a hunk or a
  single line; `Ctrl+A` stages all; `d` discards after a confirm box that
  opens on the safe answer.
- **Commit editor** (`c`, `a` to amend): subject, body, `amend` / `sign-off`
  / `no-verify` / `allow empty`, `[ Commit ]` reached by `Tab` (ADR-0034;
  `Ctrl+Enter` where the terminal reports it). Refusals on the title row.
- **Stash overlay** (`S`): `s` save with message and scope, `Enter` apply,
  `p` pop, `d` drop (confirmed).
- **Undo journal**: every write records refs, `HEAD`, index tree and worktree
  tree before and after behind a `refs/graft/backup/<worktree>/<id>` ref.
  `u` undoes, `U` redoes; a drift box asks first when the repository changed
  since. `J` browses the journal (`← u` / `← U` marks, `Space` shows the
  git commands). An interrupted operation is announced on the next launch and
  `u` restores it. Linked worktrees keep their own journal.
- **Refresh on focus**: status re-read when the terminal regains focus.
- **The acceptance walk**: `cargo run -p graft --example m3_walk`, 22
  scenarios judged by running git (`TESTING.md`, "Working tree, undo and
  stash").

#### Changed

- **The harness prelude waits for M3's writes and reads** (`update::prelude`):
  a key is fed only once no operation, journal/stash read or editor prefill
  is in flight, and once the working-tree status has landed.
- **Reads never write `.git/index`** (`GIT_OPTIONAL_LOCKS=0`; scenario
  `no-index-write-on-read`).

#### Known gaps

See [`KNOWN_GAPS.md`](KNOWN_GAPS.md)'s M3 rows: hunks in untracked files,
`i`, message history, selected-files stash, branch-from-stash, the `notify`
watcher, clearing backups, commit during an in-progress merge/rebase
(refused), the signing prompt (unverified), `Ctrl+Enter` outside the kitty
keyboard protocol, a forced undo not pinned to the entry the drift box named.

### M2 — Details and diffs

What a commit changed is readable without leaving the graph, and a bad merge
is flagged and explained.

#### Added

- **The detail pane**: `Enter` on a graph row opens a file tree and a unified
  diff. `v` side-by-side, `W` word tint, `w` ignore whitespace, `[`/`]`
  context (0–20), `Tab`/`Shift+Tab` cycle focus Graph → Files → Diff, `Esc`
  closes it. Divider drag and `<`/`>` resize the pane; the size persists per
  layout orientation.
- **Compare (`x`)**: marks a row as the compare base; the pane then follows
  the cursor showing the selected commit against that base.
- **Context actions (`.`, right-click)**: a menu with `Explain` (marked rows),
  `Copy SHA`, `Compare with this`, `Open details`.
- **Map view (`M`)**: one bar per local branch bucketed by date, with tag and
  Mistake Radar marks, `↑↓` to select and `Enter` to jump the graph to a
  branch tip.
- **Overview strip (`O`)**: a column along the graph's edge marking merges,
  Mistake Radar findings, search hits and the current viewport.
- **Mistake Radar**: backward merges, foxtrot merges, accidental pulls, evil
  merges, duplicate commits, force-pushed remotes and risky repository state
  are flagged with `⚠`/`ⓘ` in a marker column; `.` → `Explain` shows why,
  with the relevant commits highlighted and the rest of the graph faded.
  Configurable per kind under `[radar]`; diff options under `[diff]`.

#### Changed

- **`Enter` no longer peeks a truncated subject in the footer (ADR-0023)** —
  it opens the detail pane instead; the footer peek is gone.
- **A marker column** appears beside the graph, badges when the repository
  has any Mistake Radar finding.
- **`Ctrl+R` reopens the walk on the commit-graph path** after `B` writes a
  commit-graph file, re-anchored on the current selection (closes
  KNOWN_GAPS row 54).

#### Known gaps

See [`KNOWN_GAPS.md`](KNOWN_GAPS.md)'s M2 sections — including image diffs in
the pixel tier, Map view's mouse and remote-branch handling, the overview
strip's default, and the compare header's missing file/line counts, all
owned by M8 polish unless noted otherwise.

### M1 — The graph

Graft draws its commit graph. Nothing here has been confirmed by eye in a
real terminal yet — see README's "What has actually been watched" and
`TESTING.md`'s manual checklist, still **not run**.

#### Added

- **The commit graph itself**: lanes, merge curves, refs and relative dates
  for the current branch's ancestry, across pixel (Kitty), Unicode and ASCII
  rendering tiers chosen from M0's capability probe (or forced by
  `graph.renderer`). `j`/`k` scroll, `1` toggles first-parent-only view.
- **Folding beyond the lane cap**: the cap follows terminal width
  (8/10/12 lanes) and is a priority-ordered budget, not a set of vetoes
  (ADR-0022, ADR-0025); lanes over the cap fold behind a `⋮` rail, with the
  exact count in the header (`+N lanes`).
- **No hover tooltip** (ADR-0023): `Enter` on a truncated row reveals its
  full subject in the footer; `y` copies the selected commit's SHA. Full
  message, parents and containing branches move to a detail pane in M2.
- **Pixel-tier ancestry emphasis is alpha-only** (ADR-0024): ancestry
  highlighting never changes stroke width, so lane geometry is stable under
  focus.
- **A scripted fixture corpus** for the fast tier: one small repository per
  history and ref shape, built by `git fast-import` with a fixed identity and
  clock, so the same shape has the same object ids on every machine. Built
  once into `target/fixtures/` and cached under a digest of the generator, so
  changing the generator rebuilds and nothing goes stale.
- **A heavy test tier**, off by default and run only at milestone reviews. It
  builds a **100,000-** and **1,000,000-commit** repository — and the same
  history with a commit-graph, as a separate fixture, because ADR-0008 makes
  first-paint order depend on having one — and raises `proptest` from 64 cases
  to 1,000. `cargo test` still runs zero of it: heavy tests are `#[ignore]`d
  *and* behind the `heavy-tests` feature, so a default build does not compile
  them. Frame and memory budgets pass at 1M commits (KNOWN_GAPS.md 56d/64);
  first paint is measured warm, with one cold check left for the user
  (ADR-0027).
- **A peak-memory measurement** (`getrusage`, ADR-0015) for the 300 MB gate
  PROMPT.md §7.1 asks for, now gating the heavy tier.
- **`scripts/clone-git-git.sh`**, which caches `git/git` outside the
  repository for manual testing — 85,649 commits in 42 MB, because
  `--filter=tree:0` leaves the trees on the server. Run by hand; no test ever
  touches it.
- **The Graph Gallery** (`cargo run -p graft --example gallery`): walks 24
  named fixture shapes with a banner describing what each shows or differs.
- **`scripts/m1-checkpoint.sh`**: the milestone's manual checkpoint. Opens one
  named scenario (`git-git`, `criss-cross`, `foxtrot`, `octopus`) or lists
  them, building the fixture corpus or cloning `git/git` as needed; exits
  non-zero with a clear message on any missing prerequisite (no network,
  a fixture that won't build) rather than skipping silently.
- **[`TESTING.md`](TESTING.md)**: the three tiers and their exact commands,
  what each costs, how to build and delete the fixtures, the manual terminal
  checklist (pixel and Unicode tiers, tmux, SSH, two verdicts per terminal —
  a pass condition and a "reads well" judgement left blank for the user), and
  the ADR-0027 cold-first-paint check.

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
