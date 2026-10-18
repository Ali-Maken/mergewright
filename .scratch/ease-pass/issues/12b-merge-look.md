# merge-look — `⏎` on the review opens a read-only look: a solved file's result, or what git combined on its own

Status: ready (split from 12, 2026-10-18, after scout `.scratch/ease-pass/scouts/12-finish-review-and-look.md`)
Blocked by: 12 (`ReviewView`, `ReviewRow`, `ReviewBar`, the review's keys by id)
Build with: Sonnet (builder-sonnet) — one widget and two reads whose numbers are checked against `git diff --numstat` by fixture
Needs: 12 `ReviewView`, `ReviewRow::{Clashed, Combined}`, `ReviewBar`; graft-ops `MergeReview` (`by_git` paths); graft-git `line_origins` (`diff/origins.rs:43`), `Origin`; 05b the review's `y` handler (the look's `y` calls the same fn).

## Decided here
- **Look state:** `⏎`/`space` (`finish.toggle_diff`, matched by id since 12) in the merge review → `Showing::Look` inside `Finish`; no new context or id. Bar `ReviewBar::Look` = `esc back to the review · y commit the merge` (03 Handoff). `esc` → review, cursor kept; `y` → the review's `y` fn. `space` pairs with `⏎` (the mock gives `space` no meaning in the review; NOTES).
- **Clashed row:** the result is the **worktree file** (solved files stay unmerged until `y`, 05b; `staged_diff`, `jobs/finish.rs:48`, diffs index vs HEAD — wrong here). Lines numbered, read-only; marks as the merge mock's look shows (copy; none shown → plain).
- **Combined row** (one row for all N files, 12): per file `yours +a −d  theirs +a −d`, **zero parts and unchanged sides left out** (mock :169-175: `yours +6`, `theirs +14`); counts from `diff --numstat` merge-base→`HEAD` and merge-base→`MERGE_HEAD`, limited to the review's combined paths. One sample hunk: the first combined path changed on both sides, else the first path; its lines marked `from yours` / `from theirs` (`line_origins`). `Origin` mapped to a ui-side enum in `view::finish` (graft-ui does not depend on graft-git).
- **States** (New UI pane-state rule): reading → `reading…`; failed → `can't tell: <reason>`.
- **Inline diff retired:** with `⏎` on the look and no non-merge summary (scout), `expanded`, `ExpandedDiff`, `DIFF_ROWS_MAX` and `staged_diff` have no caller → removed with their own tests, each named in NOTES (grep first; a remaining caller → keep and name it).
- Job kind for the look read **appended** at the end of `jobs/kinds.rs`, `update/state.rs`, `update/events.rs` (11b owns them this wave; NOTES names the lines).

## Read first
- `docs/design-notes/ease-pass.md` M17 (:149-150), New UI "Review and look" (:274-275).
- `crates/graft-ui/MAP.md` `finish_summary`, `detail_pane::diff_body`, `graph_screen::hints`. `crates/graft/MAP.md` `jobs::finish` (:132), `update::finish`, `view::finish`. graft-git `MAP.md` `diff` (`line_origins`).
- `12-finish-review-and-look.md` Decided, Signatures. Scout 12 "Read", "Keys".
- Merge mock :163-175 (the look).

## Scope
In: graft-ui new `merge_look.rs` (+ tests), `finish_summary/**` (inline-diff removal), hints `hints/context/review_bar.rs` (`Look`), `hints/lists.rs` (`Review` arm's `Look`), `hints/catalogue/bars.rs`, `hints/tests/bars.rs`; graft `jobs/finish.rs`, `update/finish/**`, `view/finish.rs`, sweep arms (`sweep_screens.rs`, `dead_keys.rs` — free this wave), the three shared files above (append only).
Out: the review frame and its keys (12); the clash screen (11b); graft-ops, graft-git.

## Signatures
graft app `jobs::finish`:
```rust
/// The result of a solved, still-unmerged file: the worktree text.
pub fn look_result(repo: &RepoHandle, path: &Path, stop: &dyn Stop) -> Result<String, RunError>;
pub struct SideCounts { pub path: PathBuf, pub yours: (u32, u32), pub theirs: (u32, u32) }   // (added, deleted)
pub struct Sample { pub path: PathBuf, pub lines: Vec<(u32, String, Origin)> }
/// Two numstat runs (merge-base→HEAD, merge-base→MERGE_HEAD) over `paths`, and one sample.
pub fn combined_look(repo: &RepoHandle, paths: &[PathBuf], stop: &dyn Stop) -> Result<(Vec<SideCounts>, Option<Sample>), RunError>;
```
graft-ui:
```rust
pub enum Origin { Yours, Theirs, Both }   // finish_summary's own
pub enum LookLine { Plain { n: u32, text: SanitizedText }, From { n: u32, text: SanitizedText, origin: Origin } }
pub struct CombinedFile { pub path: SanitizedText, pub yours: (u32, u32), pub theirs: (u32, u32) }
pub enum LookBody { Reading, Failed(SanitizedText), Clashed { lines: Vec<LookLine> },
    Combined { files: Vec<CombinedFile>, sample: Option<(SanitizedText, Vec<LookLine>)> } }
pub struct LookView { pub path: SanitizedText, pub body: LookBody, pub top: usize }
pub struct MergeLook<'a> { pub view: &'a LookView, pub theme: &'a Theme, pub keys: &'a Keymap }   // Widget
pub enum ReviewBar { List, Look }
```

## Touches
`toggle_diff` (`keys.rs` handler, mouse `finish/mouse.rs:74`); `ExpandedDiff`/`staged_diff` callers (grep, NOTES); `HintContext::Review` matches (graft-ui, graft); hint snapshots (existing unedited); `tests/no_literal_keys.rs`; `MAP.md` rows; `over_400.txt` (no row raised).

## Acceptance (each tick names its test)
- [ ] `⏎` on `report.md` → look: numbered lines equal the worktree file; typing changes neither state nor file; `esc` → review, cursor on `report.md`; `space` the same as `⏎` — state + render tests.
- [ ] `⏎` on the combined row → one line per file; numbers equal `git diff --numstat $(git merge-base HEAD MERGE_HEAD) HEAD` / `… MERGE_HEAD` for those paths — it test; a side with no change and zero parts are not drawn — render test; sample has ≥1 `from yours` or `from theirs` line — render test.
- [ ] Look `y` → same state as review `y` — state test.
- [ ] Look bar exactly `esc back to the review · y commit the merge` — hint snapshot (new); sweeps green with `Review(Look)` (`every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `screen_for`/`classified_form`).
- [ ] Read failed → `can't tell: <reason>`; reading → `reading…` — render tests.
- [ ] Inline diff gone: `rg 'ExpandedDiff|staged_diff' crates` empty, or each remaining caller named (NOTES).
- [ ] Captures (colour unverified): `docs/shots/ease-pass/merge-look-clashed-120.txt`, `merge-look-combined-120.txt`.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k.
