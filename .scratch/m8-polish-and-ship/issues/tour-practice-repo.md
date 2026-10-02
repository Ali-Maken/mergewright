# G-tour — a practice repo, eight tasks done by doing, `Esc` back to his repo

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the risk is the path guard (never write outside
the temp dir) and the in-process switch back; both tested. Not Fable: approach decided here, judged by tests.
Needs: `look-outside-repo-screen` (`Exit::Open`, empty state), `look-chrome-header-gutter-bar` (`HeaderView::practice`),
`keymap-spell-from-the-table` (card words), `palette-help-and-actions` (app rows), `first-run-clone-and-first-launch`
(same empty-state files, sequential).

## Read first
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` §3 and Consequences
- `docs/design/02-keymap.md` §2.18, §2.13 (`t`); `docs/design/01-view-mockups.md` §1.0 (practice header)
- `docs/design-notes/m8-polish-and-ship.md`: Journeys J12, J13; "Where he lands" tour rows; New UI "Tour card"
- `docs/research/first-run.md` — vimtutor, JetBrains learning project, zellij #5064, helix #13483 rows only
- `crates/graft-ops/MAP.md` `## journal` (entry kinds); `crates/graft-fixtures/MAP.md` (how fixtures script git);
  `crates/graft/MAP.md` `event_loop` (`Exit`, `run` re-entry), startup

## Decisions (planner)
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

## Signatures
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
`TourTask` is `pub` here (the note had `pub(crate)`): graft reads it to word the card.

## Acceptance
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

## Touches
- `event_loop::run` re-entry and terminal pairing (5c proved once; now nested return); startup discovery; `Esc` chains
  (keymap §2.2 row; blame/history trail rule); journal reads per frame (ticks read on journal change only); header
  practice form; empty state widget + bar (`t practice tour`); palette app rows; recents; G-rename list
  (`tributary-practice-*` already new-named); MAP.md graft-ops, graft-ui, graft.

## Scout questions
- How `Exit::Open` was built in 5c (file:line); what state survives a re-run (keymap, theme, config problems)?
- Which journal entry kinds and fields identify "a stage of one hunk", "a stash then its apply/pop", "a merge finished
  after the resolver" (file:line in graft-ops journal)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k tokens. If over, card widget + palette row split to a second ticket after the ops half lands.
