# clash-screen-inline-and-combined — three panes retired, `Combined by git (N)` in the list, `merging <from> into <into>`

Status: done 2026-10-18 (split from 11, 2026-10-18, after scout `.scratch/ease-pass/scouts/11-clash-screen-look.md`)
Blocked by: 11 (list column, `ListTail`, `INLINE_FROM`, files under `conflict_resolver/files/`)
Build with: Sonnet (builder-sonnet) — deletions plus one read checked by fixture (`git diff --cached --name-only`); nothing structural
Needs: 11 `INLINE_FROM`, `FileListView`, `ListTail`; graft-ops `ConflictSet`, `FinishSummary`/`ByGit` (the by-git walk `finish_summary` uses); graft-ui `BannerView`, `banner_line` (`graph_screen/banner.rs:124`).

## Decided here
- **Inline at every width ≥100** (§1.5 edit approved 2026-10-17): `Arrangement::ThreePane`, `three_panes` (`panes.rs:93`, `panes/band.rs`, `panes/draw.rs`), `THREE_PANE_MIN` (`conflict_resolver.rs:73`) removed; `Arrangement` itself removed if only `Card` is left (NOTES). Tests that pin three panes are deleted with the code they pin, each named in NOTES: graft-ui `tests/resolver_layout.rs:133-206`, `conflict_resolver/tests/three_panes.rs`; graft `tests/it/resolver/three_panes.rs`, `update/resolver/tests/three_panes.rs`, the three-pane case in `frame_budget.rs`. Card tests passing `Arrangement::Card` (`tests.rs:137,185,228,243`, `card_colours.rs`, `tests/finish_row.rs:70,116`) re-pointed, assertions unedited.
- **`Combined by git` read:** new graft-ops fn, the index's stage-0 entries that differ from `HEAD`, unmerged paths left out — the same walk that builds `FinishSummary::by_git`, extracted and shared, not copied. Merge only (Q12 a: other operations draw no section). Asked in the job that reads the stop's `ConflictSet` (name it in NOTES); a new job kind only if that job can't carry it — then at the end of `jobs/kinds.rs` (**11b owns `jobs/kinds.rs`, `update/state.rs`, `update/events.rs` this wave**; 12b appends at the end and names its lines).
- Section states (New UI pane-state rule, design note :271-272): reading → `Combined by git · reading…` muted; failed → `Combined by git · can't tell: <reason>`; read → `Combined by git (N)` + names; N = 0 → section not drawn (status rule). Rows are information: not selectable, a click does nothing.
- **Header (M5, Q12 a):** top row `merging <from> into <into>` for a merge, names from `SideNames` (no new read); other operations: the words their graph banner uses. The file title (11) sits under it. Replaces `resolver_title`'s `name · N conflicts · M open` (`view/resolver.rs:194`).
- **Banner (§1.13 edit):** `banner_line` for a merge reads `merging <from> into <into>` + today's step part; `BannerView` gains the names. Built where `BannerView` is built today — **not** in `update/banner/resume/**` (12's).

## Read first
- `docs/design-notes/ease-pass.md` M5, M6 (:142-144), New UI (:268-276). `docs/design/01-view-mockups.md` §1.5, §1.13. ADR-0043 §1.
- `crates/graft-ui/MAP.md` `conflict_resolver` (:268-307), `graph_screen` banner rows. `crates/graft-ops/MAP.md` `finish` (:97-103), `ConflictSet` (:86). `crates/graft/MAP.md` `view::resolver`.
- `.scratch/ease-pass/scouts/11-clash-screen-look.md` "Arrangement", "Header, banner, list". `11-clash-screen-look.md` Decided.
- The merge mock's clash-screen text (header and list section words only).

## Scope
In: graft-ui `conflict_resolver/**` (panes, layout, list section), `graph_screen/banner.rs`; graft-ops `finish/**` (new `finish/combined.rs`, the by-git walk extracted from where `finish_summary` builds it; `finish/review.rs` untouched); graft `view/resolver*`, the stop's read job, the `BannerView` construction site; tests and captures below.
Out: hints files; `update/finish/**`, `jobs/finish.rs` (12, 12b); key behaviour.

## Signatures
graft-ops `finish`:
```rust
/// Files the stopped merge changed without a clash: stage-0 index entries differing from HEAD.
pub fn combined_by_git(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<PathBuf>, SummaryError>;
```
graft-ui `conflict_resolver`:
```rust
pub enum CombinedList { NotMerge, Reading, Failed(SanitizedText), Files(Vec<SanitizedText>) }
pub struct FileListView { /* 11's */ pub combined: CombinedList }
```
graft-ui `graph_screen`: `pub struct BannerView { /* existing */ pub names: Option<(SanitizedText, SanitizedText)> }   // (from, into), merge only`

## Touches
`layout()` dispatch (`layout.rs:108`), `arrangement()` callers (`view/resolver.rs:135,145`); `FileListView` literals (grep); `BannerView` literals and `banner_line` snapshots; `FinishSummary::by_git` (behaviour unchanged — `finish::summary_*` tests green unedited); captures `docs/shots/m5-06-stop-lands-in-resolver-160x48.txt` (recaptured), 11's `merge-clash-open-120.txt` and `merge-all-solved-120.txt` (recaptured with the section and header); graph banner captures (grep `merging` under `docs/shots/`); `MAP.md` rows (graft-ui, graft-ops, graft); `over_400.txt` (no row raised).

## Acceptance (each tick names its test)
- [x] `rg 'ThreePane|three_panes|THREE_PANE_MIN' crates` empty (only MAP history gone; grep run 2026-10-18); 150/180/200 inline beside the 30-col list — `mergewright-ui` `resolver_layout::wide_frames_render_inline_beside_the_30_column_list`, `conflict_resolver::tests::arrangement_is_inline_from_100_at_every_width`.
- [x] `combined_by_git` on the fixture (2 combined, 2 clashing) = `git diff --cached --name-only` minus `git ls-files -u` — ops `finish::combined::combined_by_git_is_staged_minus_unmerged` (+ `combined_by_git_is_merge_only`).
- [x] `by_git` unchanged — `finish::summary_splits_yours_from_gits`, `finish::unrecorded_puts_every_change_under_git`, `finish::yours_and_deleted_are_read_from_the_index` green unedited (46/46 `finish` ops tests).
- [x] List: `Combined by git (2)` + names; reading/failed words; N=0 nothing; rebase none — ui `files::tests::combined::*` (6), app `view::resolver::tests::clash_header::{the_list_shows_what_git_combined, the_section_says_reading_failed_or_nothing, a_rebase_draws_no_section}`.
- [x] Header `merging sara/fix-toc into wallet-release`; rebase = its banner words — `clash_header::{a_merge_header_says_merging_from_into, a_rebase_header_is_its_banner_words}`.
- [x] Graph banner `merging sara/fix-toc into wallet-release` — ui `graph_screen::banner::tests::a_merge_banner_says_merging_from_into` (exact string).
- [x] Captures (colour unverified): `docs/shots/ease-pass/merge-clash-200.txt` new; `merge-clash-open-120.txt`, `merge-all-solved-120.txt` recaptured (`clash_shots::capture_the_clash_screen`). `m5-06-stop-lands-in-resolver-160x48.txt` **not** recaptured — no test writes it (same open box as 11; acceptance walk's job).
- [x] NOTES: §1.5 and §1.13 edit text below.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~55k.

## NOTES (builder, 2026-10-18)
- Scout used: `.scratch/ease-pass/scouts/11-clash-screen-look.md`. **Compacted once** mid-ticket (during capture regeneration); nothing remains. Also re-pointed `m5_walk` two-files@200 and `resolver::takes` wide asserts to inline.
- **Header = banner.** The clash screen's top row *is* the operation banner (`view/frame/conflicts.rs::draw_resolver_banner` → `banner_line`), so M5's header and §1.13's banner are one change: `BannerView.names` set in `update/banner.rs::banner_view` from `side_names` (role words stripped, no new read); `None` → `MERGING` kept. **Title row left as `▸ f · N conflicts · M open`** (Q1 open): the ticket's "replaces resolver_title" was ambiguous with "file title sits under it"; mock's `N of M clashes open · K left in the merge` needs counts for unread files — not built.
- Read carried by the conflict-set job (`jobs/runner/reads.rs::read_conflict_set` → `jobs/conflict.rs::combined_by_git`); `Product::ConflictSet` now carries `update::resolver::SetRead {set, combined}`. No new job kind; `jobs/kinds.rs` untouched.
- `ResolverState.combined: Loading<Arc<Vec<PathBuf>>>`; names drawn by last path segment (mock), column only (strip <100 draws no section). Failed reason goes on its own row under `Combined by git · can't tell:` (30 cols cut it to nothing otherwise) — deviation from one-line wording.
- Deleted with the panes: `panes.rs`, `panes/{band,draw}.rs`, ui `tests/three_panes{.rs,/band.rs}`, `tests/shots/resolver-three-200.txt`, `resolver_layout_is_pinned_three_200`; app `tests/it/resolver/three_panes.rs`, `update/resolver/tests/three_panes.rs`; `ScreenRow::{PaneTitles,Aligned}`, `Cell`, `LineRef`, `WholeFile.{ours,theirs}_absent` + `absent_on`, dead `rule/cursor_mark/take_keys`, `drawn_for_tests`. `arrangement(placed)` lost its `state` arg. **Deviation:** `frame_budget.rs` kept, re-pointed to inline at 187/200 (heavy tier; not run here).
- Old captures `docs/shots/m6/fix-three-panes-*.txt` left in place (historical; nothing writes them now) — conductor: delete or keep.
- 11's questions: Q1 open (above). Q2–Q8 not answered by this ticket or mock section read — left.
- Touches: banner word change re-pointed `MERGING` asserts in it tests (conflict_interim, landing_shots, m5_walk, shell_begun, quit_waits, tour_walk, tour_tasks) to `merging `; ~30 m5/m6/m7/m8 captures regenerated (header row only, plus list section in merge resolver shots); `no_literal_keys` allow-list (panes rows out, ops `finish/combined.rs` parse error in); `over_400.txt` two rows lowered; MAP.md ui/ops/app.
- §1.5 edit text: "From 100 columns the clash screen is inline at every width: the 30-column list, then the file, open clashes side by side. The three-pane arrangement is retired. In a merge the list ends with `Combined by git (N)` and the names, muted, not selectable." §1.13 edit text: "A merge's banner reads `merging <from> into <into>`; other operations unchanged."
