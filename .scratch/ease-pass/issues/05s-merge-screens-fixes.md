# 05s — merge screens fixes (review ease-pass-05-10: Minors 1–6, 12, 13; Q2's stale comment; M2 for three rows)

Status: ready (planner, spec-only pass, 2026-10-18, from `docs/reviews/ease-pass-05-10.md`)
Blocked by: none (05b, 05c, 10 merged). **Disjoint by file from 05t** (the rest of M2, Minors 9, 10) and from 08u — either order. Shares only registries, row by row: `crates/graft/tests/it/over_400.txt` (05s lowers 3 rows; 05t and 08u others), `crates/*/MAP.md` (one row each) — the conductor merges rows by hand.
Build with: Opus — three crates and one routing change (the review's door: when it opens, when it refuses) whose cases a state test pins; no seam another ticket builds on, so not Fable.
Scout: none — the review names every line; recheck each against main before editing.
Needs: existing — graft-ops `FileState`, `file_state`, `merge_review`, `finish_preview`, `SummaryError`, `ConflictSet::solved`, `MergeLook`; graft-ui `MergeParts`, `PreviewBox::with_merge`; app `still_open_words`, `ReviewFailed::StillOpen`, `Picks::resolver_seen`, `open_finish`. Keys: no new ids; `n` spelled from `resolver.next_block`, `y` from `confirm.yes`.

## Read first
- `docs/reviews/ease-pass-05-10.md`: Minors 1–6, 12, 13; Q2.
- `.scratch/ease-pass/issues/05b-merge-review-at-y.md`: Decided, Scope (`y`/`c` bullets), NOTES "Gotchas". `10-merge-preview.md`: Decisions, Scope M2 box, Signatures, NOTES (fast-forward, spawn cost).
- `docs/design/02-keymap.md` §2.3b (:304); `docs/design/01-view-mockups.md` §1.7 (:298); `docs/design-notes/m6-conflicts.md` `journey-counts` fence (:490-529).
- MAP.md: graft-ops `## finish`, `## preview`, `FileState`/`ConflictSet` (:76, :86); graft-ui `## preview_box`; graft `update::finish`, `update::resolver`, `update::banner`, `jobs::finish`.

## Decided here
- **Minor 1 — a fast-forward gets its own words** (mock silent: no fast-forward in it; these are ours). Git makes no merge commit and combines nothing, so the box says so and keeps the four-row shape (same layout, different words):
  ```
  Merge sara/fix-toc into wallet-release
  Nothing has changed yet.

  Comes in      3 commits from sara/fix-toc (<who>)
  Changes       <C> files
  Git combines  nothing: no merge commit is made
  You decide    nothing: no file clashes

  After y wallet-release moves up to sara/fix-toc.
  ```
  Names as the title spells them (full ref names). `y` spelled from `confirm.yes`. Footer unchanged (`y merge  esc cancel  ⏎ see the N commits`). A fast-forward cannot clash, so its words win over a `not_checkable` line. Unrelated histories never fast-forward (no case). The landing after `y` is not this ticket's (12's knock-on, written there).
- **Minors 2 + 3 — one door to the review, one unit in its words.**
  - **Clash count rule (graft-ops):** `FileState::open_clashes`: `Open { open }` → `open.max(1)` (a whole-file card is one clash), `Solved`/`Done` → 0. The resolver's count and the read's count both use it.
  - **The read names clashes:** `merge_review` refuses with a new `SummaryError::ClashesOpen { files, clashes }`; `StillUnmerged { count }` stays `finish_summary`'s only (files git lists — a different thing, a different variant). App `ReviewFailed::StillOpen` carries `{ files, clashes }`.
  - **The door** (resolver `y`/`c`, banner `c`, merge only): **refuse at once** — no job, no review — when the resolver has read at least one file with a clash open **and** every file it hasn't read is one `ConflictSet::solved` names (so the count is whole); else **ask the read**. Banner `c` uses the same door when the resolver has been opened this stop (`Picks::resolver_seen`), else asks the read.
  - **The review opens only when the read lands:** `Ready` → the review (Files focus, as today); `Failed` → the review opens showing the failure (today's words and place); `StillOpen` → the notice only, the review never drawn. Nothing is drawn between key and answer. One read in flight: `y`/`c` again while it runs do nothing. The marker is not `finish` (that stays `None` until `Ready`/`Failed`); it clears on every answer and when the stop ends.
  - **Words:** resolver origin, at once or from the read: `N clashes still open · n goes to the next` (`1 clash still open · …`). Banner origin: today's file words, counted from `files`, unchanged.
- **Minor 4:** keep the read's error when both fail; the removal's error is dropped **with a comment saying why it is safe** (inert scratch under a unique name, as `snapshot/temp_index.rs:140-145` argues), and the `:133-134` comment's promise is reworded to what holds ("said when the read succeeded; when the read failed, its error is the one returned").
- **Minor 5:** `finish_preview` becomes `pub(crate)`; `merge_review` is the one public door. Every caller outside `graft-ops/src` (grep) is re-pointed at `merge_review` (same facts, judged from disk) or, if it needs a solved list `merge_review` can't build, becomes a unit test beside `finish/review.rs`.
- **M2, this ticket's three rows** (the files its tests live in): `crates/graft/src/update/finish/tests.rs` — 05b's review tests move to `update/finish/tests/review.rs` (row ≤823, the pre-05b value); `crates/graft/src/update/banner/tests.rs` — the banner-`c`-in-a-merge tests move with the new ones to `update/banner/tests/review_door.rs` (row ≤1965); graft-ops `tests/it/finish.rs` — row ≤443 (re-pointing for Minor 5 or moving a block). Pre-05b values: `git show 4ed45107^:crates/graft/tests/it/over_400.txt`. Moves keep fn names; no line-joining.

## Scope
In:
- graft-ops: `preview/kinds.rs` (the fast-forward path sets the flag, :81), `preview/merge_look.rs` (field + tests), `conflict/state.rs` (`open_clashes` + table test), the file defining `SummaryError` (new variant), `finish/review.rs` (Minors 3, 4, 5), `tests/it/finish.rs`, the it files holding `merge_look::` and `merge_review::` tests, `MAP.md` (rows: `SummaryError`, `finish_preview`, `merge_review`, `MergeLook`, `FileState`).
- graft-ui: `preview_box/merge.rs` and its tests; `MAP.md` `MergeParts` row.
- graft: `update/preview/merge_view.rs` (maps the flag); `update/resolver/review.rs` (the door, `still_open_words`); `update/finish/answers.rs`; the `open_finish` site; `update/banner/pressed.rs`; `update/finish/tests.rs` → `tests/review.rs`; `update/banner/tests.rs` → `tests/review_door.rs`; `update/resolver/tests/review_at_y.rs`; `tests/it/merge_preview.rs` (+ its `shots`); `tests/it/merge_one_file.rs`; `MAP.md` `update::finish` row.
- Docs: §2.3b :304 and §1.7 :298 — add (Minor 13) "Over unrelated histories the box says `Can't merge: <from> and <into> share no history.` and withholds `y` (`m m` and the list's `y` too); `Esc` and `Enter` still work." and (Minor 1) "A fast-forward says `Git combines  nothing: no merge commit is made` and `After y <into> moves up to <from>.`" `m6-conflicts.md` `last-block-to-finished` comment (Q2, below).
- **Minor 12:** `rg -n '\.conflicts\b' crates/` — list every reader in NOTES (file:line, what it does with a merge plan's value, and that the value is unchanged in kind: `Clean` / `Likely { paths }` / `NotChecked(r)`, now from `conflicts_from`). Reading only; a wrong reader → stop and report it (rule 7, with its line).
- **Q2 comment:** the journey counts 05b edited are confirmed (planner). One stale comment: `last-block-to-finished` (3) still lists `o ⏎ y`; in a merge it is `o y y` (count unchanged). Read the walk's scenario for that id (`tests/it/m5_walk/`, read only): if it is a merge and sends `o y y`, fix the comment; if it sends anything else, or isn't a merge, say what it sends in NOTES and stop on that line (the walk is 05t's file).
Out: every file in 05t's list and 08u's (`update/commit_editor/`, `view/frame`, graft-ui `commit_editor`, `graph_screen/{layout,mod}.rs`); the review's look (12); the clash list's look (11); KNOWN_GAPS (conductor's). **Do not push `update/resolver.rs` (398), `update/finish/keys.rs` (390), `update/preview/answer.rs` (391) or graft-ops `preview/effects.rs` (395) past 400** — new code goes in the files named above. Need one of 05t's files → commit at the seam, `blocked: needs <file>`.

## Signatures
graft-ops:
```rust
impl FileState {
    /// Open clashes this file counts for; a whole-file card (`Open { open: 0 }`) is one.
    pub fn open_clashes(self) -> usize;
}
pub enum SummaryError { /* existing */ ClashesOpen { files: usize, clashes: usize } }  // merge_review only
pub(crate) fn finish_preview(repo: &RepoHandle, solved: &[PathBuf], stop: &dyn Stop) -> Result<FinishSummary, SummaryError>;
pub struct MergeLook { pub check: MergeCheck, pub incoming: Range, /// The merge only moves `into` up: no merge commit.
    pub fast_forward: bool }
```
graft-ui: `MergeParts` gains `pub fast_forward: bool` (words above; wins over `not_checkable`).
graft app:
```rust
// update::resolver::review
pub(crate) enum Door { Refuse { files: usize, clashes: usize }, Read }
pub(crate) fn review_door(/* the resolver's read files + the stop's ConflictSet — fit to where they live, name it in NOTES */) -> Door;
// update::finish
pub(crate) enum ReviewFailed { StillOpen { files: usize, clashes: usize }, /* existing */ }
```

## Touches (check each, report how)
- `SummaryError` matches (grep): app `answers.rs`, `jobs::finish`, any `Display`/`explain`; `StillUnmerged` still reached from `finish_summary` (test stays green).
- `finish_preview` callers (grep; Minor 5). `MergeLook` constructions (grep; `..Default::default()` sites per 10 NOTES).
- `finish = Some(.. Pending)`: every reader (answer routing, view, sweeps `update/tests/sweep_screens.rs`, `dead_keys.rs`) — the review is no longer `Pending` for a merge; say which readers still see `Pending` (non-merge summary).
- Existing tests whose words or routing move: `c_with_a_file_still_open_is_refused_by_the_review_read`, `y_in_the_review_stages_the_files_the_read_found_solved`, `review_at_y::*`, it `conflict_interim::continue_refuses_while_unmerged_then_finishes` (05t's file — must pass unedited; if it can't, `blocked`).
- Merge preview captures (`docs/shots/ease-pass/merge-*-120.txt`): unchanged except the new one (`git diff --stat`).
- Keymap, journal/undo: none (no key, no write changes) — say the grep.

## Acceptance
Each tick names its test inline. New tests seen red first.
- [ ] **Minor 1, words:** graft-ui `preview_box::merge::tests::a_fast_forward_says_nothing_is_combined_and_into_moves_up` (the seven lines above, exact) and `…::a_fast_forward_with_no_check_still_says_it_only_moves_up`.
- [ ] **Minor 1, plan:** graft-ops it `merge_look::a_fast_forward_plan_says_so_and_a_true_merge_does_not` (`fast_forward` true / false).
- [ ] **Minor 1, app:** it `merge_preview::a_fast_forward_says_it_only_moves_up_and_m_y_moves_it` (fixture: `wallet-release` behind `sara/fix-toc`, no commits of its own; box lines exact; `m y` → HEAD = `sara/fix-toc`'s tip, HEAD has one parent, one journal entry). Capture `docs/shots/ease-pass/merge-fast-forward-120.txt` from `merge_preview::shots::capture_the_merge_preview`; colour unverified, look unjudged.
- [ ] **Clash count:** `conflict::state::tests::open_clashes_counts_a_whole_file_card_as_one` (`Open{3}`→3, `Open{0}`→1, `Solved`→0, `Done`→0).
- [ ] **Read names clashes:** graft-ops it `merge_review::a_file_with_two_open_clashes_is_refused_with_its_clash_count` (`ClashesOpen { files: 1, clashes: 2 }`); `finish_summary`'s `StillUnmerged` test unchanged and green (name it).
- [ ] **At once:** `update::resolver::tests::review_at_y::y_with_a_clash_open_and_every_file_known_refuses_at_once` (no review-read command out; `finish` is `None`; notice `1 clash still open · n goes to the next`).
- [ ] **No flash:** `…::y_with_an_unread_file_asks_the_read_and_draws_no_review_until_it_lands` (after `y`: one read command, `finish` `None`; `y` again: no second command; answer `StillOpen { files: 1, clashes: 2 }` → notice `2 clashes still open · n goes to the next`, `finish` `None`).
- [ ] **Ready / Failed:** `…::the_review_opens_when_its_read_lands_ready` (Files focus); `…::a_failed_review_read_opens_the_review_showing_it`.
- [ ] **Banner `c`:** `update::banner::tests::review_door::banner_c_in_a_merge_with_a_clash_open_opens_no_review` (today's banner words, from `files`); `…::banner_c_with_every_file_solved_opens_the_review_when_the_read_lands`.
- [ ] **Minor 6:** it `merge_one_file::u_in_the_review_lands_on_the_undone_clash` (review, `u`, pump → resolver open, `cursor_block` on the undone clash, `open == 1`).
- [ ] **Minor 4:** by reading — quote the new comment lines in NOTES.
- [ ] **Minor 5:** `rg -n 'finish_preview' crates --glob '!crates/graft-ops/src/**'` empty (NOTES); `merge_review::*` it tests green.
- [ ] **Minor 12:** the reader list in NOTES, with the grep command.
- [ ] **Minor 13 + docs:** by reading — quote the §2.3b and §1.7 sentences in NOTES.
- [ ] **Q2 comment:** by reading — quote the walk's keys and the new comment in NOTES.
- [ ] **Rows:** `update/finish/tests.rs` ≤823, `update/banner/tests.rs` ≤1965, graft-ops `tests/it/finish.rs` ≤443; no row raised, no new file over 400 — graft's ratchet test (name it); test count before/after in NOTES (`cargo nextest list`), moved fns' old → new paths where a ticket tick names them (grep `.scratch/ease-pass/issues/`).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~75k. Split point if it overruns: Minors 1, 4, 5, 12, 13 + Q2 here; the door (2, 3, 6) and its three rows as a follow-on.
