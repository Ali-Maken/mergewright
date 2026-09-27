# history-whitespace-and-sizes — `w` in the history place; a binary commit shows its sizes

Status: done
Remainder of: `21-history-place.md` (gaps in its NOTES: "binary sizes not read (no reader)"; "`w` `[` `]` on the file
pane not built (`whole_file` takes no whitespace option)"). 21's Signatures and Rulings stand.
Amended 2026-09-27 (planner, spec-only pass) against scout `.scratch/m7-power-tools/scouts/21c.md` at `f47ce67d`:
`Product::WholeFile`'s real payload, `ReadWholeFile`'s `max_lines`, no whitespace words anywhere, the existing `size()`.
Blocked by: none (21b and 10b merged).
Crates: graft-git, graft, graft-ui. Docs: `docs/design/02-keymap.md` §2.17 (one row).
Build with: **Sonnet** — one flag threaded through one existing reader, one small new reader, one payload widened;
exact signatures below, judged by tests against real repos. Fails Fable test 1 (nothing structural to get wrong).
Scout: done (`scouts/21c.md`). Read it first; its line numbers are the map.
Needs: `graft_git::trace::whole_file` (whole.rs:35), `Command::ReadWholeFile` (events/command.rs:289),
`Product::WholeFile` (events.rs:272), `place::history_answer::whole_file_arrived` (:134),
`graft_ui::history::{FilePane, size}` (history.rs:114, :520).

## Read first
- Scout `scouts/21c.md` (whole). `crates/graft-git/MAP.md` `trace` entry; `crates/graft/MAP.md` "History place" row;
  `crates/graft-ui/MAP.md` `FilePane` row. `docs/design/02-keymap.md` §2.5 (`w`) and §2.17 only.
- Then source at the scout's lines.

## Files (ADR-0051)
- **graft-git:** `src/trace/whole.rs`; new `src/trace/sizes.rs` + its `mod`/`pub use` line in `src/trace/mod.rs`;
  their tests; `MAP.md`.
- **graft:** `src/jobs/detail/history.rs`; `src/jobs/runner.rs` (:284, :341-349, field pass-through only);
  `src/update/events.rs` (`Product::WholeFile`, `WholeFileAnswer`); `src/update/events/command.rs` (`ReadWholeFile`);
  `src/update/mod.rs` (:248-249, :861 — payload type only); `src/event_loop/label.rs` (:119, pattern only if it
  binds the payload); `src/update/place/{history.rs,history_keys.rs,history_answer.rs,write.rs}`;
  the history view mapping in `src/view/` (wherever `FilePane::Binary` is built); `src/update/place/tests/history.rs`;
  `tests/it/history_place.rs`; `MAP.md`.
- **graft-ui:** `src/history.rs` (+ `src/history/snapshots/`, `history/tests.rs`); `src/graph_screen/hints/lists.rs`
  (`history_place_list`), `hints/tests.rs`; `MAP.md`.
- **Docs:** `docs/design/02-keymap.md` §2.17 row; `docs/shots/m7/history-binary-140x40.txt`; `TESTING.md` row.

## Decided
- **`w` in the history place** (§2.5 via §2.17): toggles ignore-whitespace for the whole-file read. Field
  `ignore_whitespace: bool` on `HistoryState`, default off; persists as `j`/`k` step commits; every `ReadWholeFile`
  carries it. Cancels a whole-file read in flight and asks again (as `j`/`k` do).
  - **Focus: either**, copying history's `v` (history_keys.rs:43-47, no focus check). Scout Q4.
  - Git flag: `-w`, the one the detail pane uses (graft-git diff/read.rs:151-152). If `whole_file` runs no `git`
    command that takes `-w`, **stop, report `blocked: whole_file has no -w seam`** — do not invent an in-process
    whitespace compare.
  - `n`/`N` follow the re-read's changed rows. The `┃` range (git's `-L` hunks) is unchanged by `w`.
- **Words while on: none in the pane** (amended). Scout Q2: the detail pane draws no words for it
  (`DiffOptionsView.ignore_whitespace`, graft-ui detail.rs:547, set but never drawn); blame has only its bar entry.
  History matches them: the file pane header is unchanged; the bar offers `w whitespace`. Adding words to history alone
  would make the same mode look different on two screens. The unseen mode is an **app-wide** gap (detail, blame,
  history): the builder records it in this ticket's NOTES as "diff panes don't show ignore-whitespace is on
  (detail/blame/history)"; the conductor files it. Not fixed here.
- **Bar:** `w whitespace` (blame's words, lists.rs:141) goes right after `v`'s entry in each `history_place_list` form
  that offers `v`; if no form offers `v`, in the `on_file` form before `esc back`.
- **`[` / `]`: not built.** Conductor ruling (2026-09-27): a whole file has no context to widen; drop them from §2.17's
  history file-pane row. This builder edits that row only. They stay unbound in history; the bar doesn't offer them.
- **Binary sizes:** when the `FileDiff` `whole_file` returns is binary (the same fact that today makes the place draw
  `FilePane::Binary`), the job also calls `blob_sizes` (first parent for a merge; old path for a rename) and ships the
  answer beside the `DiffView`. Not binary → `sizes: None`, no second read.
  - Pane: `binary · <size(old)> → <size(new)>`. **Formatter: the existing `graft_ui::history::size()`** (history.rs:520:
    `<n> B`, else whole `KB`/`MB`, absent → `none`). No second formatter; the one-decimal `KiB` rule is dropped (amended).
    (The resolver's `size_words`, graft `view/resolver/whole_file.rs`, is a separate formatter — leave it; note it in
    NOTES as a unify candidate.)
  - Size read fails → not swallowed (rule 3): `binary · couldn't read sizes: <Display>`, dim. The diff itself stands.

## Signatures (exact; corrected to the code)
```rust
// graft-git — trace/whole.rs: ONE parameter appended after `max_lines`; every other parameter and the return type
// stay exactly as at whole.rs:35. Its one caller (jobs/detail/history.rs:143) passes the command's flag.
pub fn whole_file(/* existing params …, */ max_lines: usize, ignore_whitespace: bool /* , existing stop … */)
    -> /* unchanged */;

// graft-git — trace/sizes.rs (new), re-exported from trace/mod.rs beside `whole_file`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobSizes { pub old: Option<u64>, pub new: Option<u64> }
/// Sizes of `path` at `commit` and of `old_path.unwrap_or(path)` at `commit^1`. `None`: absent that side
/// (added, deleted, root commit). One `git cat-file --batch-check` for both; `missing` → `None`, anything else
/// unparseable → `Err`.
pub fn blob_sizes(repo: &RepoHandle, commit: Oid, path: &Path, old_path: Option<&Path>, stop: &dyn Stop)
    -> Result<BlobSizes, RunError>;

// graft — update/events/command.rs: one field added
Command::ReadWholeFile { id, commit, path, old_path, max_lines, ignore_whitespace: bool }

// graft — update/events.rs: payload widened (was Result<Arc<DiffView>, String>, events.rs:272)
Product::WholeFile(Result<WholeFileAnswer, String>)
pub struct WholeFileAnswer {
    pub view: Arc<graft_ui::detail::DiffView>,
    /// `Some` only when the diff is binary. `Err`: the size read's `Display`.
    pub sizes: Option<Result<graft_git::trace::BlobSizes, String>>,
}

// graft — place/history_answer.rs:134 (argument type follows the product)
pub fn whole_file_arrived(/* existing params …, */ answer: Result<WholeFileAnswer, String>);

// graft-ui — history.rs:114 (a field added; 21's rule: add, never remove)
FilePane::Binary { header: &'a str, old: Option<u64>, new: Option<u64>, sizes_failed: Option<&'a str> }
```
View mapping: `Some(Ok(s))` → `old: s.old, new: s.new, sizes_failed: None`; `Some(Err(e))` → `None, None, Some(&e)`;
`None` on a binary pane cannot happen (the job always reads sizes for binary) — map to `sizes_failed:
Some("sizes not read")` rather than `none → none`, so a wiring bug shows.

## Acceptance (name the verifying test at tick time; red first where marked)
- [x] **graft-git** (ticked: all six green in `crates/graft-git/tests/it/trace_whole.rs`; the reindent test observed red before `-w` was wired; plus `whole_file_ignoring_whitespace_of_a_whitespace_only_change_is_all_context`) (real throwaway repos): `whole_file_ignoring_whitespace_marks_a_reindent_unchanged` (**red first**:
  rows marked changed without the flag); `blob_sizes_reads_both_sides`; `blob_sizes_of_an_added_file_has_no_old`;
  `blob_sizes_of_a_deleted_file_has_no_new`; `blob_sizes_follows_a_rename`; `blob_sizes_on_a_root_commit`.
- [x] **graft:** (ticked: the six place tests in `update/place/tests/history.rs` + `a_text_commit_reads_no_sizes` in `tests/it/history_place.rs`, plus `a_binary_answer_without_sizes_says_so`. Red observed: compile-red on the old payload for the binary tests; `n_follows_the_changes_after_w` observed failing before the `w` key existed; the other `w` tests were not run red on their own) `w_rereads_ignoring_whitespace_from_either_focus` (**red first**: no command sent; asserts the
  `ReadWholeFile` carries `ignore_whitespace: true`, both focuses); `w_cancels_a_read_in_flight`; `w_holds_as_j_steps`;
  `n_follows_the_changes_after_w`; `a_binary_commit_shows_sizes` (**flipped** from 21's `None/None`; red first: fails
  on today's payload); `a_size_read_failure_is_drawn`; `a_text_commit_reads_no_sizes`.
- [x] **graft-ui:** (ticked: snapshots `history_binary_140x40`, `history_binary_sizes_failed_140x40` accepted; `history_bar_words_in_order` green; full graft-ui suite green) snapshots `history_binary_140x40`, `history_binary_sizes_failed_140x40`; `history_bar_words_in_order`
  updated (`w whitespace` placed per Decided); floor test green. Colour unverified.
- [x] **Capture:** (ticked: `capture_history_binary_140x40`; fixture 600 → 1 500 bytes, pane reads `binary · 600 B → 2 KB`) `docs/shots/m7/history-binary-140x40.txt` (producer `tests/it/history_place.rs`). Fixture: a binary
  file going from under 1 024 bytes to over, so both forms show. **Right:** file pane reads
  `binary · <n> B → <m> KB` with the exact string `size()` gives for the fixture's real byte counts, both counts
  named in the TESTING.md row; no line rows.
- [x] Keymap §2.17 row reads without `[` `]`; MAP.md (all three); TESTING.md row. (ticked: doc edits, no test; checked by reading the diff)

## Touches (check each, report how)
- **`whole_file`'s one caller** (history.rs:143): passes the flag. Check: build + 21's history tests green.
- **`ReadWholeFile` sites:** place/history.rs:197 (builds it), place/write.rs:66 (`is_write`, still false),
  runner :284/:341. **`Product::WholeFile` sites:** update/mod.rs:248-249, :861, event_loop/label.rs:119;
  `Task::WholeFile` (mod.rs:500, label.rs:81) unchanged. Check: `place/tests/history.rs`, `esc_back.rs` green; grep
  `WholeFile` shows no site missed.
- **Detail pane `w`** (detail/keys.rs:379-383): untouched; history keeps its own flag, not the detail's
  `DiffOptions`. Check: detail `w` tests green.
- **Blame place `w`:** different place, own flag; unchanged. Check: blame tests green.
- **History bar:** `w whitespace` added; any 21b capture whose bar shows the form it lands in is re-shot (check all
  four, say which).
- **graft-git MAP** `trace` entry (`whole_file` flag, `blob_sizes`); ADR-0048 needs no change.

## Rules / Gates
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Workspace-wide
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
~45k. One context.

## NOTES (builder, 2026-09-28)
- Scout used: `scouts/21c.md`; its lines held (runner's `ReadWholeFile` arm now :341-360 after 30j).
- No compaction.
- Cause vs symptom: feature work, no fix. Under `-w`, a whitespace-only change gives a header with no hunk; `whole_file` then falls back to the file as all context (git emits no rows, so this is the only way to show the file).
- Gap for the conductor to file: **diff panes don't show ignore-whitespace is on (detail/blame/history)**.
- Unify candidate: graft `view/resolver/whole_file.rs` `size_words` vs `graft_ui::history::size()`.
- `blob_sizes`: a path holding a newline cannot be asked through `--batch-check`; the answer is then `missing` or an Err. Not tested.
- The bar shows `w whitespace` only in the `on_file` form (no form offers `v`), though `w` works from either focus, per Decided.
- Touches: `whole_file`'s caller (jobs/detail/history.rs): builds, history tests green. `ReadWholeFile` sites: place/history.rs builds it with the flag; write.rs `is_write` still false (`history_is_a_read_not_a_write`); the runner passes it through. `Product::WholeFile` sites: update/mod.rs:250 (the type follows), :861 and label.rs `(_)` patterns unchanged. **Outside the file list:** the `update/mod.rs` re-export gained `WholeFileAnswer` (jobs must name it); `update/place/tests/hints.rs` `history_rows_in_the_place_bar_table` gained the `w whitespace` row (it pins the bar). Detail `w` and blame `w` untouched; full suite green. 21b captures: none shows the `on_file` form (all four use the list form) and all are green unchanged.
- Colour unverified (the dim failure line).
