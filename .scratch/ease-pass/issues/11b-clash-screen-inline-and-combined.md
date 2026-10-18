# clash-screen-inline-and-combined — three panes retired, `Combined by git (N)` in the list, `merging <from> into <into>`

Status: ready (split from 11, 2026-10-18, after scout `.scratch/ease-pass/scouts/11-clash-screen-look.md`)
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
- [ ] `rg 'ThreePane|three_panes|THREE_PANE_MIN' crates` empty (NOTES); widths 150, 180, 200 render inline with the 30-col list — render tests.
- [ ] `combined_by_git` on the fixture merge (2 combined, 2 clashing) returns exactly the 2 combined paths, equal to `git diff --cached --name-only` minus `git ls-files -u` paths — graft-ops it test.
- [ ] `finish_summary`'s `by_git` unchanged — existing `finish::summary_*` tests green unedited (name them).
- [ ] List: `Combined by git (2)` + both names; reading and failed states render their words; N = 0 draws nothing; rebase draws no section — render tests.
- [ ] Header `merging sara/fix-toc into wallet-release` on the clash screen; rebase header = its banner words — render tests.
- [ ] Graph banner during the paused merge reads `merging sara/fix-toc into wallet-release …` — `banner_line` snapshot (new).
- [ ] Captures (colour unverified): `docs/shots/ease-pass/merge-clash-200.txt` (new), recaptured `merge-clash-open-120.txt`, `merge-all-solved-120.txt`, `m5-06-stop-lands-in-resolver-160x48.txt`.
- [ ] NOTES: §1.5 and §1.13 edit text for the conductor.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~55k.
