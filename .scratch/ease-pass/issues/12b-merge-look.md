# merge-look — `⏎` on the review opens a read-only look: a solved file's result, or what git combined on its own

Status: done (7 of 7 boxes; see NOTES). Was: ready (split from 12, 2026-10-18, after scout `.scratch/ease-pass/scouts/12-finish-review-and-look.md`)
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
- [x] `⏎` on a clashed row → look: numbered lines equal the worktree file; typing changes neither state nor file; `esc` → review, cursor kept; `space` the same as `⏎` — `update::finish::tests::look::{enter_on_a_clashed_row_opens_its_look_and_esc_goes_back, space_opens_the_look_as_enter_does, typing_in_the_look_changes_nothing}`, `merge_look::tests::a_clashed_file_draws_numbered_and_read_only`, it `finish::choices::the_look_on_a_clashed_row_is_the_worktree_file` (real merge, `gone.txt`; file bytes compared before/after typing), `jobs::finish::look::tests::a_solved_file_s_result_is_its_worktree_text`. Fixture file is `gone.txt`/`notes.txt`, not `report.md`.
- [x] `⏎` on the combined row → one line per file; numbers equal numstat merge-base→HEAD / →MERGE_HEAD — `jobs::finish::look::tests::the_counts_equal_git_numstat_per_side` (3 files: both / yours-only / theirs-only) and it `finish::captures::summary_captures` (`look_captures`, README.md vs `git diff --numstat`); zero parts and unchanged sides not drawn — `merge_look::tests::the_combined_rows_leave_out_zero_parts_and_unchanged_sides`; sample ≥1 `from …` — `merge_look::tests::the_sample_marks_where_each_line_came_from` + `look_captures` asserts `from theirs` on the real merge; ask from state — `update::finish::tests::look::the_combined_row_looks_at_every_file_git_combined`.
- [x] Look `y` → same state as review `y` — `update::finish::tests::look::y_in_the_look_is_the_review_s_y` (same `Resume`, same `FinishState` but the committing job id).
- [x] Look bar exactly `esc back to the review · y commit the merge` — `graph_screen::hints::tests::bars::the_look_bar_draws_the_mock_words`; `Review(Look)` in `catalogue/bars.rs`, coverage slot 137 (`every_context_reaches_every_shape`); sweep arm `sweep_screens.rs` `Review(ReviewBar::Look)` (look open, read landed) — sweeps green in the workspace run.
- [x] Read failed → `can't tell: <reason>`; reading → `reading…` — `merge_look::tests::reading_and_failed_say_so`; read landing / stale drop — `update::finish::tests::look::the_read_lands_and_a_stale_one_is_dropped`.
- [x] Inline diff gone: grep `ExpandedDiff|DIFF_ROWS_MAX|staged_diff` over `crates/` — only `mergewright-git/src/diff/{read,patch}/tests.rs` local test helpers named `staged_diff` (unrelated, graft-git, out of scope).
- [x] Captures (colour unverified): `docs/shots/ease-pass/merge-look-clashed-120.txt`, `merge-look-combined-120.txt` — checked by it `finish::captures::summary_captures`. Look is the user's to judge (rule 6).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k.

## NOTES (builder, 2026-10-18)
- Scout: used `.scratch/ease-pass/scouts/12-finish-review-and-look.md`; no new scout (12 landed since; read the code directly).
- **Removed** (no caller left): ui `ExpandedDiff`, `DIFF_ROWS_MAX`, `DIFF_INDENT`, `ReviewView::expanded`, `ListLine::{Diff, More}`, `list.rs` `draw_diff`/`indent`, test `a_long_diff_stops_at_12_rows_and_says_enter_hides`; app `FinishState::{expanded, diff_job}`, `keys::{toggle_diff, row_path}`, `answers::diff_landed`, `jobs::finish::staged_diff`/`unchanged` + tests `the_diff_is_the_staged_change`, `a_solved_row_s_diff_is_the_file_on_disk_against_head`; `update::finish::tests::{space_expands_a_clashed_file, enter_on_the_combined_row_does_nothing_yet}`; it `space_on_a_kept_yours_row_warns_nothing` (replaced by the look test), capture `docs/shots/m6/fix-finish-summary-expanded-120x40.txt` (deleted).
- **Job kind renamed in place, not appended**: `Task::ReadFinishDiff`→`ReadFinishLook` (`update/events.rs` ~:247), `Product::FinishDiff`→`FinishLook` (~:473), `Command::ReadFinishDiff`→`ReadFinishLook` (`events/command.rs` ~:428); also `answer.rs`, `answers.rs`, `facts.rs`, `place/write.rs`, `event_loop/label.rs`, `jobs/runner/{dispatch,reads}.rs`. One-line renames; merge with 11b's appends should be clean unless 11b touched those exact lines. No `jobs/kinds.rs` edit needed.
- **Signature deviations**: `look_result`/`combined_look` return `Result<_, String>` not `RunError` (a worktree file read is an io error, not a git run); `look_result` takes no `stop`. `Sample::lines` is `Vec<(u32, String, Option<Origin>)>` — `None` for a context line neither side added (the signature had no plain line). Types `SideCounts`/`Sample`/`LookAsk`/`LookRead`/`Look` live in `update::finish::look` (jobs depend on update, not the reverse).
- **Sample method**: the base→result diff has no `DiffSpec` (index/worktree vs merge-base); used `HEAD`→worktree's first hunk (theirs changed) or base→`HEAD` (yours only), every non-removed line offered to `line_origins` as a result line. Real merge capture shows `+line TWO   from theirs`.
- **Clashed look marks**: plain numbered lines; the mock's per-pick colouring (`clash(l)`) needs the picks' line ranges — not done. Gap.
- **No scrolling in the look**: the mock's bar names only `esc`/`y`; a file longer than the frame is cut at its bottom. `LookView::top` exists, always 0. Gap: needs a key decision (keymap §2.6b).
- Combined header says `both sides changed different lines, so it kept both` (mock words) even when only one side changed a file — wording question for the user.
- `y` in the look closes the look then runs the review's `commit` (so the screen while committing is the review, as from the list).
- KNOWN_GAPS questions: none answered here — ease/12-file-order (mock lists FILES in its own order, no rule), ease/12-bar-120, ease/12-landing-bar / ease/06h-landed-bar (landing bar is outside this ticket's scope) left open.
- Touches: hint catalogue + coverage slot (graph_screen/tests.rs 136→138 slots), sweep `screen_for` (List now also sweeps the combined row), prelude hold (`look_job`), mouse click → opens the look, MAP.md (ui `merge_look`, `finish_summary`; app `jobs::finish`, `update::finish`, `view::finish`), captures (1 deleted, 2 new). `tests/no_literal_keys.rs` passes (keys by id).
- Gates: fmt, clippy, doc green. nextest 6959/6960: the one red is `no_literal_keys::no_user_facing_literal_spells_a_default_key` on `mergewright/src/refs/{perf_ahead,tests_ahead}.rs`. Those files came in with 07a (7ef32a83) and are outside this ticket. Not fixed.
- Also fixed: the look draws only above the review's floor (`fits`). Below it the review's one line and `esc` hold (`bar_keys::every_key_the_bar_draws_is_answered`). Journey `summary-open-git-combined` (m5_walk) now expects the look. Removed `toggle_diff` dead and sweep rows. Updated over_400.txt (one row removed, two lowered; none raised).
- Stale: `docs/shots/m6/m6-contact-sheet-1.txt` still contains the deleted expanded capture; its caption row is gone. Regenerate it with the sheet script (`scripts/m7-accept.sh` family).
