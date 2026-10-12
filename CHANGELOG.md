# Changelog

Notable changes, per milestone. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

There are no released versions yet: Mergewright is unreleased. Entries below
the first name the app by what it was called at the time.

## [Unreleased]

### Unreleased: renamed to Mergewright (command `mwr`)

The app, its command and its package are now Mergewright and `mwr`. On-disk
names move on first launch, nothing lost: the config folder
(`~/.config/mergewright/`), and in each repository `.git/mergewright/` and
`refs/mergewright/` (ADR-0057).

### M5 — History and Ghost Preview

You see the result before you confirm.

#### Added

- **Ghost Preview** for `m` merge, `r` rebase, and the `.` menu's
  cherry-pick, revert, reset (mixed/hard), checkout (branch/detached) and
  delete, plus a drag-and-drop menu on branch badges: ghost rows in the
  graph where the new commits would land, a box naming the ref moves, the
  working-tree effect (`LOSES <file> (backup saved)` for a hard reset), the
  conflict forecast, `undo: u takes this back` and the exact git command.
  `Enter` on `[ Confirm ]` runs it; the cursor starts on `[ Cancel ]`.
- **Refusals in words**: `<ref> moved since the preview · <key> again` when
  a ref moved under an open box; `<verb> needs a clean tree · S stash N
  changed file(s)`; too small a screen says `preview needs 40x12`.
- **Operation banner and CONFLICTS section**: a stopped merge, rebase,
  cherry-pick or revert puts `MERGING`/`REBASING step N of M` in the
  header, lists the files under `CONFLICTS (n)` in the changes pane, and
  the footer offers `c continue`, `a abort`, `M mergetool`, `e edit`. The
  stop survives a relaunch (`still in progress from a previous run`).
- **The time machine**: `Enter` on a journal entry previews `revert to
  before "<entry>" (N steps)`, restoring index and working tree from the
  backups; undoable.
- **Diverged pull** offers `m merge it  r rebase onto it` on the line.
- **Undo after a push** asks first, naming what the remote still has.
- **Credential prompt** uses a zeroizing secret field: a fixed 8 mask
  glyphs, never the typed length.
- **Hint bar redesign** (ADR-0037 §4): the three most relevant keys, more
  with width, never a key that does nothing.

#### Not in M5

- Resolving conflicts inside the app (M6), interactive rebase, pushing a
  rewritten branch. See `KNOWN_GAPS.md`.

### M4 — Remotes

You never leave Graft to fetch, pull or push — not for a password, a
rejection or a hang.

#### Added

- **`f` fetch** (all remotes, prune), **`p` pull** (fast-forward only,
  undoable: `u` puts branch and tree back, the tracking ref stays fetched),
  **`P` push** through a confirm box that names the branch, the commit count,
  `sets upstream` when it will, and the exact `git push` line.
- **Rejections in words, on the footer until `Esc`**: non-fast-forward
  (`push rejected: … has moved · p pull first · P force with lease`), a stale
  lease (`push refused: … (stale lease) · f to fetch, then look`), a diverged
  pull (`pull stopped: … shell: git pull --rebase`), a dirty file
  (`pull refused: would overwrite <file> (uncommitted)`), no upstream,
  detached `HEAD`, authentication failure (`· P to try again`).
- **The lease box** after a rejection: `Force-push main → origin/main?`,
  the remote tip it expects, `--force-with-lease=<branch>:<tip>`; a plain
  `--force` never runs; refused for any branch but `HEAD`'s upstream.
- **Credential prompt in the app**: `graft` is git's askpass helper over a
  private socket; password questions are masked (`•`), username questions
  are not; `Esc` cancels the operation. Nothing typed is stored, logged,
  journaled or captured.
- **`Esc` cancels any running write** (SIGTERM, 2 s grace, entry
  `Aborted("cancelled")`), fetch, pull and push included — the walk cancels
  each of the three against a socket that never answers; `u`/`U` are not
  stoppable and say so.
- **Live progress** from `git --progress` on the footer, `· esc cancels`.
- **Push and fetch are journaled but transparent to undo**: `u` skips them
  and, after a push, says `· origin/main still has it`; the journal rows read
  `not undoable — origin has it · <old> → <new>` / `not undoable`.
- **Real-remote test fixtures** (`graft-fixtures::remote`): a bare over
  `file://`, smart HTTP with Basic auth on loopback, a port that never
  answers.
- **The acceptance walk**: `cargo run -p graft --example m4_walk`, 21
  scenarios judged by running git on both the work clone and the bare
  (`TESTING.md`, "Remotes").

#### Changed

- **Every coverage table is written by one function** (`coverage_matrix`):
  `gallery --matrix`, `m3_walk --matrix` and `m4_walk --matrix` now produce
  the same `docs/shots/35-coverage-matrix.txt` (the gallery used to drop the
  working-tree table).

#### Known gaps

See [`KNOWN_GAPS.md`](KNOWN_GAPS.md)'s M4 rows: clone (M8), add/rename/remove
remotes (M7), pull with merge/rebase (M5), per-remote fetch (M7), a stall
notice or timeout (M7), `q` confirming during an operation (M8), SSH askpass
(env set, unverified), the `notify` watcher (M7), the typed password held in
a plain `String` until the prompt closes, remote footer hints only at 156
columns or more.

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
