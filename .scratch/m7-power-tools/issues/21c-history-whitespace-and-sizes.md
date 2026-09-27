# history-whitespace-and-sizes — `w` on history's file pane; a binary commit shows its sizes

Status: draft
Remainder of: `21-history-place.md` (gaps in its NOTES: "binary sizes not read (no reader)"; "`w` `[` `]` on the file
pane not built (`whole_file` takes no whitespace option)"). 21's Signatures and Rulings stand.
Blocked by: `21b-history-place-shown` (shares `hints/lists.rs`, `graft-ui/src/history.rs`, `place/history*.rs`).
Crates: graft-git, graft, graft-ui.
Build with: **Sonnet** (planner recommends; conductor sets after the scout). One option threaded through an existing
reader, one small new reader, exact signatures below; judged by tests against real repos. Fails Fable test 1.
Scout: **needed** (≤ ½ page, brownfield), questions below.
Needs: `graft_git::trace::whole_file`, `graft_git::diff::FileDiff`/`DiffOptions`, `Command::ReadWholeFile`,
`Product::WholeFile`, `place::history_answer::whole_file_arrived`, `graft_ui::history::FilePane` (all from 21).

## Scout (≤ ½ page, before building)
1. Every caller of `whole_file` (expect one: `jobs/detail/history.rs::read_whole_file`). Line.
2. How the detail pane's `w` reaches git (`DiffOptions::ignore_whitespace` → which flag, which line), and the words its
   header shows while on. Line.
3. Where `Product::WholeFile` is matched (routing `update/mod.rs`? `history_answer.rs`?). Lines. Does any file 10b
   holds name it?
4. History's `v` arm (`place/history_keys.rs`): which focus answers it. `w` copies that rule.
5. An existing byte formatter in graft-ui (grep `KiB`). Line or none.

## Files (ADR-0051)
- **graft-git:** `src/trace/whole.rs`; new `src/trace/sizes.rs` + its `mod`/`pub use` line in `src/trace/mod.rs`;
  their tests; `MAP.md`.
- **graft:** `src/jobs/detail/history.rs`; `src/update/events/command.rs`, `src/update/events.rs` (the two variants);
  the `Product::WholeFile` routing line (scout Q3; if it is in `update/mod.rs`, a type-name edit only);
  `src/update/place/{history.rs,history_keys.rs,history_answer.rs}`; `src/update/place/tests/history.rs`;
  `tests/it/history_place.rs`; `MAP.md`.
- **graft-ui:** `src/history.rs` (+ `src/history/snapshots/`, `history/tests.rs`); `src/graph_screen/hints/lists.rs`
  (`history_place_list`), `hints/tests.rs`; `MAP.md`.
- **Docs:** `docs/shots/m7/history-binary-140x40.txt`; `TESTING.md` row.

## Decided (planner, 2026-09-27)
- **`w` on the file pane** (§2.5 via §2.17): toggles ignore-whitespace for the whole-file read. It lives on
  `HistoryState` and persists as `j`/`k` step commits; every `ReadWholeFile` carries it. Cancels a read in flight and
  asks again (as `j`/`k` do). Same focus rule as history's `v` (scout Q4). The file pane header gains the detail pane's
  own words for it while on (scout Q2); no new wording.
  - `n`/`N` follow the re-read's changed rows. The `┃` range (git's `-L` hunks) is unchanged by `w`.
  - The bar's `on_file` form gains `w whitespace` before `esc back` (the blame place's words).
- **`[` / `]`: not built here.** The file pane is the whole file: there is no context to widen or narrow. See Proposed
  change. Until answered, they stay unbound in history and the bar doesn't offer them.
- **Binary sizes:** when the whole-file read is binary, the job also reads both blob sizes (first parent for a merge;
  old path for a rename). The file pane: `binary · <old> → <new>`.
  - Absent side (added, deleted, root): `none`.
  - Format: under 1 024 → `<n> B`; else one decimal `KiB` / `MiB`. Use graft-ui's existing formatter if scout Q5 finds
    one; else this rule, in `history.rs`.
  - Size read fails → not swallowed (rule 3): `binary · couldn't read sizes: <Display>`, dim. The diff itself stands.

## Signatures (exact)
```rust
// graft-git — trace/whole.rs (one parameter added; its caller updated)
pub fn whole_file(repo: &RepoHandle, commit: Oid, path: &Path, old_path: Option<&Path>, max_lines: usize,
                  ignore_whitespace: bool, stop: &dyn Stop) -> Result<FileDiff, DiffError>;
// graft-git — trace/sizes.rs
pub struct BlobSizes { pub old: Option<u64>, pub new: Option<u64> }
/// Sizes of `path` at `commit` and of `old_path.unwrap_or(path)` at its first parent. `None`: absent that side.
pub fn blob_sizes(repo: &RepoHandle, commit: Oid, path: &Path, old_path: Option<&Path>, stop: &dyn Stop)
    -> Result<BlobSizes, RunError>;

// graft — commands and products
// Command::ReadWholeFile { id: JobId, commit: Oid, path: PathBuf, old_path: Option<PathBuf>, ignore_whitespace: bool }
//                                                   → Product::WholeFile(Result<WholeFileAnswer, DiffError>)
pub struct WholeFileAnswer { pub diff: FileDiff,
                             /// `Some` only when `diff` is binary. `Err`: the size read's words.
                             pub sizes: Option<Result<BlobSizes, String>> }

// graft-ui — history.rs (a field added; 21's rule: add, never remove)
// FilePane::Binary { header: &'a str, old: Option<u64>, new: Option<u64>, sizes_failed: Option<&'a str> }
```

## Acceptance (name the verifying test at tick time; red first where marked)
- [ ] **graft-git:** `whole_file_ignoring_whitespace_marks_a_reindent_unchanged` (**red first**: rows marked changed
  without the flag); `blob_sizes_reads_both_sides`; `blob_sizes_of_an_added_file_has_no_old`;
  `blob_sizes_of_a_deleted_file_has_no_new`; `blob_sizes_follows_a_rename`; `blob_sizes_on_a_root_commit`. Real
  throwaway repos.
- [ ] **graft:** `w_on_the_file_pane_rereads_ignoring_whitespace` (**red first**: no command); `w_holds_as_j_steps`;
  `n_follows_the_changes_after_w`; `a_binary_commit_shows_sizes` (**flipped** from 21's `None/None`; red first:
  `binary · sizes not read`); `a_size_read_failure_is_drawn`.
- [ ] **graft-ui:** snapshots `history_binary_140x40`, `history_binary_sizes_failed_140x40`; `history_bar_words_in_order`
  updated (`w whitespace`); floor test green. Colour unverified.
- [ ] **Capture:** `docs/shots/m7/history-binary-140x40.txt` (producer `tests/it/history_place.rs`). **Right:** file
  pane reads `binary · 1.2 KiB → 3.4 KiB` (the fixture's real sizes, named in TESTING.md); no line rows.
- [ ] MAP.md (all three), TESTING.md row.

## Touches (check each, report how)
- **`whole_file`'s callers** (scout Q1): each passes the flag. Check: the build + 21's history tests green.
- **Other `ReadWholeFile` / `Product::WholeFile` sites:** `is_write` (still false), `trail_back`'s cancel, the runner's
  dispatch. Check: `place/tests/history.rs`, `esc_back.rs` green.
- **History bar:** `w whitespace` added; 21b's captures whose bar has the file focus re-shot (none of 21b's four has
  it: say so after checking).
- **graft-git MAP** `trace` entry; ADR-0048 needs no change.

## Proposed change (not decided; conductor's call — a keymap-doc correction, not a scope cut)
- Keymap §2.17 lists `[` `]` on history's file pane. They change context lines; a whole file has none to change.
  Proposal: drop `[` `]` from §2.17's `w v [ ] L` row. Alternative (invent): `[`/`]` switch the pane between the whole
  file and hunks-with-context — new behaviour nobody asked for (field notes :71 asked for the whole file). Planner
  recommends dropping. If the user wants them, that is new scope for a later ticket.

## Rules / Gates
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Workspace-wide
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
~45k. One context.

## Conductor note (2026-09-27)
`[`/`]` do nothing on a whole file: drop them from keymap §2.17 for the history place (doc fix, conductor ruling on planner recommendation; not a scope cut). This builder edits `docs/design/02-keymap.md` §2.17 for that row only.
