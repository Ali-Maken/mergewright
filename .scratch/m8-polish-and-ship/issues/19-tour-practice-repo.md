# G-tour — a practice repo, eight tasks done by doing, `Esc` back to his repo

Status: ready (amended 2026-10-08 after scout `.scratch/m8-polish-and-ship/scouts/19-tour.md` — **the Amendment at the
end overrides anything above it**)
Build with: **Opus**, as two parts in disjoint crates (see Amendment §A0): **19a** graft-ops only (~35k), then **19b**
graft + graft-ui (~70k). Opus, not Fable: every signature is fixed below; the risks (path guard, the place re-entry)
are judged by tests, not by eye.
Blocked by: 06, 08, 10, 10c, 11, 11b, 18
Needs: `look-outside-repo-screen` (`Exit::Open`, empty state), `look-chrome-header-gutter-bar` (`HeaderView::practice`),
`keymap-spell-from-the-table` (card words), `palette-help-and-actions` (app rows: add `AppAction::Tour` beside
`AppAction::Theme`), `palette-doorway-and-hint-rows` (empty-state bar doorway; same files), `first-run-clone-and-first-launch`
(same empty-state files, sequential).

## Read first
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` §3 and Consequences
- `docs/design/02-keymap.md` §2.18, §2.13 (`t`); `docs/design/01-view-mockups.md` §1.0 (practice header)
- `docs/design-notes/m8-polish-and-ship.md`: Journeys J12, J13; "Where he lands" tour rows; New UI "Tour card"
- `docs/research/first-run.md` — vimtutor, JetBrains learning project, zellij #5064, helix #13483 rows only
- `crates/graft-ops/MAP.md` `## journal`, `## init`, `## clone` (how a non-`Operation` runs git); `crates/graft-core/MAP.md`
  `## journal` (`OperationKind`, `EntryStatus`, `JournalEntry`); `crates/graft/MAP.md` `## event_loop` (lines ~100-105:
  `Exit::Open`, `locate`, `drive`), `lands_on_head_at_open`, `AppState::land_on_head`, `update::map` `seek_to`,
  `update::palette`
- **The scout report** `.scratch/m8-polish-and-ship/scouts/19-tour.md` (file:line for every seam below)

## Decisions (planner) — original; see Amendment for what changed
- **Repo shape** (built by git commands in `<tmp>/tributary-practice-<rand>/`, marker file `.practice` at its root):
  `work/` with `main` (4 commits), branch `feature` (1 commit, merges clean), branch `clash` (edits the same line of
  `menu.txt` as `main`), `notes.txt` with two separate unstaged hunks, an untracked `scratch.txt`; `remote.git` bare,
  `origin` = it, `main` pushed, then one more local commit made in the build so push has work.
- **Tasks and how each ticks** (from the practice repo's own journal, never from what was read on the card):
  1 stage a hunk (a stage entry covering one hunk) · 2 commit · 3 undo that commit (`Undo` reversing task 2's entry) ·
  4 stash and back (a stash then an apply/pop of it) · 5 branch (branch create) · 6 merge `feature` with its preview
  (a merge entry, committed) · 7 resolve a conflict (merge `clash`, finish after the resolver) · 8 push (push entry).
  A task done early ticks early; the card shows the first unticked.
- **Card** per the note's New UI: top of the graph pane, 3 rows, warning border; `practice <n> of 8 · <task>` and
  `esc leave` on the border; the line teaching the key spelled from the live table; ticks row dropped < 100 cols.
  Claims no key but `Esc`, and `Esc` only when nothing else on screen would take it (§2.18).
- **Leave**: `Esc` (as above) or all 8 done → `Exit::Open` back to where he was (his repo + cursor oid, or the empty
  state). Finished adds status `practice done ✓ · ⌃P tour goes again`. The practice dir is removed on leave — only a
  dir carrying `.practice` and under the temp root; anything else refused and reported.
- **Never his repo**: building, ticking and removal touch only the practice dir; his journal, refs, recents and layout
  file are untouched (recents never remember the practice repo).

## Scope
In: `build_practice_repo`, task detection, card widget, `Practice tour` palette row, `t` on the empty state (header
action + bar), `Exit::Open` carrying a cursor and a return target, cleanup, path guard.
Out: tour content beyond the 8 tasks; translations; replay of a single task.

## Signatures — original (superseded by Amendment §A7)
```rust
// graft-ops::tour
pub struct PracticeRepo { pub root: PathBuf, pub work: PathBuf, pub remote: PathBuf }
pub enum TourError { TempDir(std::io::Error), Git { words: String }, NotPractice(PathBuf) }
pub fn build_practice_repo(under: &Path) -> Result<PracticeRepo, TourError>;
pub fn remove_practice_repo(repo: &PracticeRepo) -> Result<(), TourError>;    // refuses without `.practice`
pub enum TourTask { StageHunk, Commit, Undo, StashAndBack, Branch, MergeWithPreview, ResolveConflict, Push }
pub fn tasks_done(entries: &[JournalEntry]) -> [bool; 8];
// graft-ui::tour_card
pub struct TourCardView { pub step: usize, pub of: usize, pub title: SanitizedText, pub line: SanitizedText,
    pub ticks: Vec<(SanitizedText, bool)> }
pub struct TourCard<'a> { pub view: &'a TourCardView, pub theme: &'a Theme }    // impl Widget
pub const TOUR_CARD_ROWS: u16 = 3;
// graft (pub(crate))
pub(crate) struct Open { pub path: Option<PathBuf>, pub cursor: Option<Oid>, pub practice: Option<PracticeRepo>,
    pub back: Option<Box<Open>> }
// Exit::Open(PathBuf) becomes Exit::Open(Open); 5c's callers pass `Open { path: Some(p), ..Default }`
```

## Acceptance — original (each line restated or cut in Amendment §A9)
- [ ] Path guard: `build_practice_repo` under a temp root writes nothing outside it (test lists the temp root's
      parent before/after); `remove_practice_repo` refuses a dir without `.practice` and one outside the root (tests).
- [ ] Each task ticks only by doing (8 tests driving the app in the practice repo); reading the card ticks nothing.
- [ ] J12 `start-tour` = 3 (`⌃P` `tour` `⏎`), `start-tour-empty-state` = 1 (`t`); J13 `leave-tour` = 1 (`Esc` → his
      repo, cursor on the same oid) — walk tests counting keys.
- [ ] `Esc` with a box/prompt/menu/search open closes that, not the tour (test per kind).
- [ ] His repo byte-for-byte untouched by a full tour: refs, journal file, index, worktree (test with `RepoTruth`).
- [ ] Build failure (unwritable temp) → stays in his repo, status `tour: <reason>` (test).
- [ ] Remapped `[keys.graph] merge = "M"` → card says `M` (test).
- [ ] Full tour walked headless, all 8 ticked, finish line shown (test; the acceptance's "tour finished with no shell").
- [ ] Captures: `docs/shots/m8-tour-step1-120x36.txt`, `…-step7-conflict-120x36.txt`, `…-80x24.txt`,
      `…-finished-120x36.txt`. **Colour unverified**.

## Touches — original (extended in Amendment §A10)
- `event_loop::run` re-entry and terminal pairing (5c proved once; now nested return); startup discovery; `Esc` chains
  (keymap §2.2 row; blame/history trail rule); journal reads per frame (ticks read on journal change only); header
  practice form; empty state widget + bar (`t practice tour`); palette app rows; recents; G-rename list
  (`tributary-practice-*` already new-named); MAP.md graft-ops, graft-ui, graft.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Original: one context, ~75k. Amended: two parts, ~35k + ~70k (§A0).

---

## Amendment (planner, 2026-10-08) — rulings on the scout's contradictions and open items

Step counts unchanged: J12 `start-tour` 3, `start-tour-empty-state` 1; J13 `leave-tour` 1. Prior art unchanged
(note rows "Tour form", "Tour exit visible; shows his keys", "Undo as the tour's hook").

### A0. Split (disjoint crates, sequential)
- **19a — graft-ops::tour** (`crates/graft-ops` only): builder, path guard, removal, `tasks_done`, `tour_progress`.
  Unit tests on synthetic entries + real temp dirs. ~35k. Opus (path guard is the one place a wrong approach deletes
  his files).
- **19b — app half** (`crates/graft`, `crates/graft-ui`): `Open`, `Exit` variants, `drive`'s practice slot, landing on a
  commit, card, palette row, empty-state `t`, keymap rows, walks, captures. Needs 19a's §A7 signatures. ~70k. Opus.
- One scout already done (this ticket's); 19b's builder re-reads only the scout and MAP entries, no second scout.

### A1. Scout 1 — journal has no hunk count → **task 1 ticks on any Committed `Stage` entry**
`Touches` has no hunk field (scout, `graft-core/src/journal.rs:161-169`). Ruling: "stage a hunk" ticks on a `Stage`
entry with `EntryStatus::Committed`, whatever it covered. The card still *teaches* one hunk of `notes.txt`.
**Cut:** "a stage entry covering one hunk". Why: a hunk count is a journal-format field (hard to reverse, every line
on disk) for a tutorial tick; parsing `description` text is fragile. Staging a whole file is still staging.

### A2. Scout 5 — branch-create kind: **exists**
`OperationKind::BranchCreate` (graft-core MAP `## journal`, m5/07). Task 5 = a Committed `BranchCreate`. 19b's walk
must show `b` (or whatever §2 binds) writes it; if it writes another kind, stop and report — never tick on text.

### A3. Scout 6 — merge finished after the resolver
`Merge` is one entry across the stop (scout `journal.rs:57-59`); `ResolveConflict` is its own kind (graft-core MAP,
m6-conflicts/03). Ruling, pure on entries (`EntryId` orders by time):
- **Task 6** = any Committed `Merge`.
- **Task 7** = a Committed `Merge` M with at least one `ResolveConflict` entry whose id is after M's and before the
  next `Merge` entry's (if any).
- 19b's task-7 walk asserts the kinds actually written (`Merge` Pending at the stop, `ResolveConflict` from the
  resolver, `Merge` Committed after finish). If the resolver writes no `ResolveConflict`, **stop and report** — that's
  a spec question, not a build decision.
- Other tasks, all `Committed` only: 2 `Commit` (not `Amend`); 3 an `Undo` whose `root_operation` is a `Commit`;
  4 a `StashPush` then a later `StashApply` or `StashPop`; 8 `Push`. Aborted / Pending never tick.

### A4. Scout 2 — every place lands on HEAD → **new state: landing on a commit**
- `Open { path, cursor: Option<Oid> }` (§A7). `cursor: None` = today's behaviour exactly (`lands_on_head_at_open`).
- Generalise `lands_on_head_at_open` to `lands_at_open(state, at: Option<Oid>)`: `None` → HEAD (unchanged);
  `Some(oid)` → the existing `HeadLanding::on` path, seeking as `update::map::seek_to` does (on hand → cursor; else
  `History::seek` + `ShowRows`). `gives_way` as today: a key/click first wins.
- Oid not found by the walk's end → HEAD, sticky status `the commit you were on is gone · on HEAD`. Test.
- Only the tour's return sets `cursor`. Every launch and every other open still lands on HEAD.

### A5. Gap M8/lands-row-launch-in-repo — **19b owns and closes it**
After §A4: launch and every `Exit::Open` from the empty state land on HEAD; the tour return lands on his commit. So
12d's row "Launch inside a repo · graph, cursor on HEAD" stays true. 19b: (1) lists every landing site with file:line
in NOTES (`drive` first place, each `Exit::Open`, `Exit::LeaveTour`, `Place::Outside` → `leave_outside`, and any
`startup.rs` site — grep `locate(` and `lands_` callers; the scout could not confirm startup's); (2) test
`a_launch_lands_on_head_and_a_tour_return_on_its_commit`; (3) closes the gap row naming that test.

### A6. Scout 3 — recents written for every repo → **the practice repo is skipped, decided in `drive`**
`drive` holds the live practice slot (§A7 `Practice`). `remember_opened` (scout `places.rs:116`, `:263`) is skipped
for the place opened from that slot — one check, at the one caller, keyed on the slot, never on a path pattern.
Same gate for any other per-repo state-dir write the tour place would make (layout); 19b finds them by grep for
writes under Graft's state dir and lists them in NOTES. Test: every file under the state dir byte-identical across a
full tour.

### A7. Scout 4 — fixtures/tempfile are dev-only → **builder lives in graft-ops::tour, no new dependency**
Runs git the way `init`/`clone` do (`GitInvocation::in_folder`), files by `std::fs`, temp dir by `std::fs::create_dir`
(not `_all`: fails if it exists — no retry) at `<under>/tributary-practice-<pid>-<unix_ms>`. Not an `Operation`: no
journal entry in any repo, as `init`. Rejected: `tempfile` at runtime (drop-deletes; the dir must outlive the place
switch and be removed by us, guarded).

**The path guard is central and private**: one fn every write and the removal pass through.
- Writes: `fn inside(root: &Path, rel: &Path) -> Result<PathBuf, TourError>` refuses absolute paths, any `..`
  component, and any result not under `root`. Every `std::fs` write and every `in_folder(…)` dir in `tour` comes from it.
- Removal: canonical `root` equals the recorded root; its parent equals canonical `under`; name starts with
  `PRACTICE_PREFIX`; `root` is not a symlink; `<root>/.practice` is a regular file (`symlink_metadata`). Then
  `std::fs::remove_dir_all` (does not follow symlinks). Any miss → `NotPractice` / `Outside`, nothing removed.
- Build config, local to the practice repo (through `inside`): `user.name Practice`, `user.email
  practice@example.invalid`, `commit.gpgsign false`, `tag.gpgsign false`, `core.hooksPath <root>/hooks` (empty dir —
  his global hooks never run there). `HEAD` set to `refs/heads/main` before the first commit. `main` pushed with
  upstream set, then the extra local commit.

```rust
// graft-ops::tour (19a)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeRepo { root: PathBuf, work: PathBuf, remote: PathBuf, under: PathBuf } // private: only build makes one
impl PracticeRepo {
    pub fn root(&self) -> &Path;
    pub fn work(&self) -> &Path;   // what graft opens
}
#[derive(Debug)]
pub enum TourError {
    TempDir(std::io::Error), Git { words: String }, NotPractice(PathBuf),
    Outside { path: PathBuf, root: PathBuf }, Remove(std::io::Error), Journal(JournalError),
}
impl TourError { pub fn explain(&self) -> String; } // the `<reason>` in `tour: <reason>`; not sanitised (caller does)
pub const PRACTICE_MARKER: &str = ".practice";
pub const PRACTICE_PREFIX: &str = "tributary-practice-";
pub fn build_practice_repo(under: &Path) -> Result<PracticeRepo, TourError>;
pub fn remove_practice_repo(repo: &PracticeRepo) -> Result<(), TourError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourTask { StageHunk, Commit, Undo, StashAndBack, Branch, MergeWithPreview, ResolveConflict, Push }
impl TourTask { pub const ALL: [TourTask; 8]; }
pub fn tasks_done(entries: &[JournalEntry]) -> [bool; 8];          // pure, §A1-A3
pub fn tour_progress(work: &Path) -> Result<[bool; 8], TourError>; // reads the practice repo's journal, then tasks_done

// graft::event_loop (19b) — `Exit` is pub, so `Open` is pub with crate-private fields
#[derive(Debug, Clone)]
pub struct Open { pub(crate) path: PathBuf, pub(crate) cursor: Option<Oid> }
impl Open { pub(crate) fn at(path: PathBuf) -> Open } // cursor None: 5c's callers
pub enum Exit { /* … */ Open(Open), Practise { from: Open }, LeaveTour { finished: bool } }
// graft::event_loop::places (19b, private to drive)
struct Practice { repo: PracticeRepo, back: Open }
// graft::update (19b)
pub(crate) struct TourState { pub(crate) done: [bool; 8], pub(crate) asked: Option<JobId> }
// AppState::tour: Option<TourState>, seeded from Arrival::practice: bool
pub(crate) fn lands_at_open(state: &mut AppState, at: Option<Oid>); // replaces lands_on_head_at_open
// Command::TourProgress(JobId) → jobs → Product::TourProgress { id, done: Result<[bool; 8], String> }
// graft-ui::tour_card — as original Signatures
```
`Open.path: PathBuf`, not `Option`: the empty state is always "a folder", which `locate` turns into `Outside`.

### A8. Open items the scout left
- **`drive`** (`places.rs:107-160`): `Exit::Practise { from }` → stop pool → `build_practice_repo(&std::env::temp_dir())`
  → Ok: slot = `Practice { repo, back: from }`, locate `repo.work()`, `Arrival::practice = true`, no recents.
  Err: locate `from` (with its cursor) and `Arrival::notice` = `tour: <explain>`. `Exit::LeaveTour { finished }` → stop
  pool → remove → locate `back` with its cursor; notice `practice done ✓ · ⌃P tour goes again` when finished; a removal
  failure adds `tour: couldn't remove <~path>: <explain>` (sanitised). `Exit::Open` while the slot is held → remove,
  drop `back`, go where asked. **Any ending of a practice place** (Quit, Signal, Fatal, StoppedHalfWay) removes after
  `wind_down`; failure is one stderr line `tour: left <path> behind: <explain>`. Never `let _ =`.
- **Ticks**: `Command::TourProgress` asked at arrival and when any write job ends (`write_job_ended`); stale id dropped;
  an `Err` → status `tour: <why>`, ticks kept. All 8 → `Exit::LeaveTour { finished: true }`. Never per frame.
- **`Esc`**: last link of the graph screen's `Esc` chain, only when `AppState::tour` is `Some` and nothing earlier
  answered. Bound as keymap row `tour.leave` = `Esc` in the existing `Tour` context (scout `keymap.rs:77`); the card's
  `esc leave` is spelled from it.
- **Empty-state `t`** (none today, scout): keymap row `empty_state.tour` = `t` (§2.13); `graft/src/empty_state.rs` →
  `Exit::Practise { from: Open::at(folder) }`. Header actions row gains `t Practice tour`; bar gains `t practice tour`
  after `c clone…` (as `docs/shots/m8-proto-3a-empty.txt:50`), and it is the **first clause shed** when the bar
  overflows (opt-in; the header row still names it). Fix the stale doc comments at `graft-ui/src/empty_state.rs:17,
  :218`.
- **Palette**: `AppAction::Tour` row `Practice tour` in `⌃P` ACTIONS and `outside_source`. Inside the tour it runs
  nothing but status `tour: you're in it · <tour.leave> leaves`.
- **Card words**: one line per `TourTask`, each naming its key spelled from the live table (`op_action(OpKey)` where an
  `OpKey` exists; Branch and ResolveConflict from their own keymap rows — ids cited in NOTES). All 8 lines in NOTES.
- **Stale captures** (grep `o open…` under `docs/shots/`): re-shoot `docs/shots/m8/first-launch-empty-state-120x36.txt`,
  `docs/shots/m8-look-outside-repo-120x36.txt` (its caption's "No tour" line updated),
  `docs/shots/m8-look-outside-repo-no-recent-80x24.txt`. `m8-proto-3a-empty.txt` is a prototype — untouched. Each
  re-shot one goes on the unjudged list (KNOWN_GAPS row, unverified) — the user judges by eye.

### A9. Acceptance (replaces the original list; nothing dropped silently)
19a:
- [ ] Path guard, build: under a temp root nothing appears outside `<root>` (test lists the temp root's parent
      before/after) — **kept**.
- [ ] Path guard, remove: refuses no `.practice`; `.practice` a symlink; root a symlink; root outside `under`; wrong
      prefix — one test each — **kept, widened**.
- [ ] `inside` refuses absolute and `..` (tests) — **new**.
- [ ] `tasks_done` per §A1-A3 on synthetic entries, incl. Aborted/Pending never tick, task 7 needs `ResolveConflict`
      after its `Merge` (tests) — **new**.
- [ ] Built repo shape: branches, two hunks in `notes.txt`, untracked `scratch.txt`, `main` one ahead of `origin/main`,
      `feature` merges clean, `clash` conflicts on `menu.txt` (test with git reads) — **new**.
19b:
- [ ] Each task ticks only by doing (8 pty walks in the practice repo); reading the card ticks nothing — **kept**;
      task 1 by any stage (§A1, "one hunk" **cut**, reason there).
- [ ] J12 `start-tour` = 3, `start-tour-empty-state` = 1; J13 `leave-tour` = 1, his repo, cursor on the same oid —
      key-counting walks — **kept**; cursor now real (§A4).
- [ ] Gone commit → HEAD + status (test) — **new**.
- [ ] `Esc` with a box/prompt/menu/search open closes that, not the tour (test per kind) — **kept**.
- [ ] His repo byte-for-byte untouched by a full tour: refs, journal file, index, worktree (`RepoTruth`) — **kept**.
- [ ] State dir (recents, layout) byte-identical across a full tour (test) — **new**, was in Decisions only.
- [ ] Unwritable temp (`TMPDIR` read-only) → his repo, his cursor, status `tour: <reason>` (test) — **kept**; mechanism
      is re-locate from `from` (§A8), observably the same.
- [ ] Remapped `[keys.graph] merge = "M"` → card says `M` (test) — **kept**.
- [ ] Practice dir gone after `Esc`, after finish, after `q` (three tests) — **new** (was "removed on leave").
- [ ] Full tour walked **in a pty** (18c's clone-walk harness; headless `run` treats `Exit::Open` as a broken promise),
      all 8 ticked, finish line shown — **kept**, "headless" → pty.
- [ ] `a_launch_lands_on_head_and_a_tour_return_on_its_commit`; gap M8/lands-row-launch-in-repo closed naming it (builder names the test in NOTES; the conductor moves the row) — **new**.
- [ ] Captures: `docs/shots/m8-tour-step1-120x36.txt`, `…-step7-conflict-120x36.txt`, `…-80x24.txt`,
      `…-finished-120x36.txt`, plus the 3 re-shot in §A8. **Colour unverified** — **kept**.

**Cut to a gap (filed by the conductor 2026-10-08 — builders don't edit KNOWN_GAPS):** leftover practice dirs from a killed process are not swept — sweeping temp by
name could delete another Graft's live tour; the OS reaps temp (`M8/tour-leftover-after-kill`, unverified).

### A10. Touches (added)
`main.rs:59` (`Exit::Open(_)` match), `places/tests.rs:208`, `exit_wait.rs:161` and `startup.rs:189` docs, every
`Exit::Open(PathBuf)` constructor (→ `Open::at`), `lands_on_head_at_open` callers, `HeadLanding`, graph-pane height
and click geometry (`update::geometry`: the card's 3 rows shift rows), keymap sweep test
`update/tests/keymap_edge/sweep.rs`, `bar_keys` and its callers, palette `listing.rs:254` / `run.rs:46` /
`graft/src/empty_state.rs:398`, header callers (`header.rs:204`), the 3 captures in §A8, KNOWN_GAPS rows, MAP.md
graft-ops (`## tour`), graft (`Open`, `Exit`, `drive`, `lands_at_open`, `TourState`), graft-ui (`tour_card`, empty
state). Each named with how it was checked in the builder's **Touches:** line.
