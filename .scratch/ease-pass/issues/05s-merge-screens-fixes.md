# 05s — merge screens fixes (review ease-pass-05-10: Minors 1–6, 12, 13; Q2's stale comment; M2 for three rows)

Status: partial — the door (Minor 2, the "at once" half of 3) blocked, see Handoff (builder 2026-10-18; was: ready, planner, spec-only pass, 2026-10-18, from `docs/reviews/ease-pass-05-10.md`)
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
- [x] **Minor 1, words:** graft-ui `preview_box::merge::tests::a_fast_forward_says_nothing_is_combined_and_into_moves_up` (the seven lines above, exact) and `…::a_fast_forward_with_no_check_still_says_it_only_moves_up`. — `preview_box::merge::tests::a_fast_forward_says_nothing_is_combined_and_into_moves_up`, `…::a_fast_forward_with_no_check_still_says_it_only_moves_up` (seen compile-red only: the field did not exist).
- [x] **Minor 1, plan:** graft-ops it `merge_look::a_fast_forward_plan_says_so_and_a_true_merge_does_not` (`fast_forward` true / false). — graft-ops it `merge_look::a_fast_forward_plan_says_so_and_a_true_merge_does_not`.
- [x] **Minor 1, app:** it `merge_preview::a_fast_forward_says_it_only_moves_up_and_m_y_moves_it` (fixture: `wallet-release` behind `sara/fix-toc`, no commits of its own; box lines exact; `m y` → HEAD = `sara/fix-toc`'s tip, HEAD has one parent, one journal entry). Capture `docs/shots/ease-pass/merge-fast-forward-120.txt` from `merge_preview::shots::capture_the_merge_preview`; colour unverified, look unjudged. — it `merge_preview::fast_forward::a_fast_forward_says_it_only_moves_up_and_m_y_moves_it` (seen red on the words, then green); capture `docs/shots/ease-pass/merge-fast-forward-120.txt` from `merge_preview::shots::capture_the_merge_preview` — colour unverified, look unjudged. Other merge captures unchanged.
- [x] **Clash count:** `conflict::state::tests::open_clashes_counts_a_whole_file_card_as_one` (`Open{3}`→3, `Open{0}`→1, `Solved`→0, `Done`→0). — `conflict::state::tests::open_clashes_counts_a_whole_file_card_as_one` (seen red).
- [x] **Read names clashes:** graft-ops it `merge_review::a_file_with_two_open_clashes_is_refused_with_its_clash_count` (`ClashesOpen { files: 1, clashes: 2 }`); `finish_summary`'s `StillUnmerged` test unchanged and green (name it). — graft-ops it `merge_review::a_file_with_two_open_clashes_is_refused_with_its_clash_count`; `finish_summary`'s `StillUnmerged` test graft-ops it `finish::summary_refuses_with_anything_unmerged` unchanged and green; the app maps `ClashesOpen` → `StillOpen{files,clashes}` (`jobs::finish::tests`). Resolver words now in clashes from the read: `update::resolver::tests::review_at_y` (`2 clashes still open · n goes to the next`).
- [ ] **At once:** `update::resolver::tests::review_at_y::y_with_a_clash_open_and_every_file_known_refuses_at_once` (no review-read command out; `finish` is `None`; notice `1 clash still open · n goes to the next`). — **open: the door is blocked — see Handoff**
- [ ] **No flash:** `…::y_with_an_unread_file_asks_the_read_and_draws_no_review_until_it_lands` (after `y`: one read command, `finish` `None`; `y` again: no second command; answer `StillOpen { files: 1, clashes: 2 }` → notice `2 clashes still open · n goes to the next`, `finish` `None`). — **open: blocked — see Handoff**
- [ ] **Ready / Failed:** `…::the_review_opens_when_its_read_lands_ready` (Files focus); `…::a_failed_review_read_opens_the_review_showing_it`. — **open: blocked — see Handoff**
- [ ] **Banner `c`:** `update::banner::tests::review_door::banner_c_in_a_merge_with_a_clash_open_opens_no_review` (today's banner words, from `files`); `…::banner_c_with_every_file_solved_opens_the_review_when_the_read_lands`. — **open: blocked — see Handoff**
- [x] **Minor 6:** it `merge_one_file::u_in_the_review_lands_on_the_undone_clash` (review, `u`, pump → resolver open, `cursor_block` on the undone clash, `open == 1`). — it `merge_one_file::u_in_the_review_lands_on_the_undone_clash` (green first run: the behaviour already held; never seen red).
- [x] **Minor 4:** by reading — quote the new comment lines in NOTES. — by reading, quoted in NOTES.
- [x] **Minor 5:** `rg -n 'finish_preview' crates --glob '!crates/graft-ops/src/**'` empty (NOTES); `merge_review::*` it tests green. — grep empty (NOTES); graft-ops it `merge_review::*` green; the one caller needing a hand-built solved list became unit test `finish::review::tests::the_review_counts_only_the_unmerged_files_not_named_solved`.
- [x] **Minor 12:** the reader list in NOTES, with the grep command. — list in NOTES.
- [x] **Minor 13 + docs:** by reading — quote the §2.3b and §1.7 sentences in NOTES. — quoted in NOTES.
- [x] **Q2 comment:** by reading — quote the walk's keys and the new comment in NOTES. — quoted in NOTES.
- [x] **Rows:** `update/finish/tests.rs` ≤823, `update/banner/tests.rs` ≤1965, graft-ops `tests/it/finish.rs` ≤443; no row raised, no new file over 400 — graft's ratchet test (name it); test count before/after in NOTES (`cargo nextest list`), moved fns' old → new paths where a ticket tick names them (grep `.scratch/ease-pass/issues/`). — `update/finish/tests.rs` 802, `update/banner/tests.rs` 1945, graft-ops `tests/it/finish.rs` 421; it `file_size::every_file_over_400_lines_is_listed_with_its_count` green. Count: 6847 tests in the full run (moves keep names). Moves in NOTES.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~75k. Split point if it overruns: Minors 1, 4, 5, 12, 13 + Q2 here; the door (2, 3, 6) and its three rows as a follow-on.

## NOTES (builder, 2026-10-18)
- **Compacted once** mid-ticket. No scout: the ticket says none.
- **Minor 3 as built:** `FileState::open_clashes`; `merge_review` → `SummaryError::ClashesOpen { files, clashes }`; `jobs::finish` maps it to `ReviewFailed::StillOpen { files, clashes }`; resolver words `N clash(es) still open · n goes to the next` via `update::resolver::clashes_still_open` (also used by the resolver's own count; `row_open` counts through `FileState::open_clashes`). Banner words unchanged, counted from `files`.
- **Minor 4 comment** (`graft-ops/src/finish/review.rs`): "Removed before the summary is returned. A copy that would not go is said when the read succeeded; when the read failed, its error is the one returned." and "Safe to drop the removal's error when the read failed: the copy is inert scratch under a name no other run takes (as `snapshot/temp_index.rs` argues for its own), and the read's failure is the one he can act on."
- **Minor 5:** `grep -rn finish_preview crates --include='*.rs' | grep -v '^crates/graft-ops/src/'` → empty. Two it callers re-pointed to `merge_review`; one (hand-built solved list) is now `graft-ops/src/finish/review/tests.rs::the_review_counts_only_the_unmerged_files_not_named_solved`. MAP row for `finish_preview` dropped (crate-only; `map_coverage` lists public items only).
- **Minor 12** (`grep -rnE '\.conflicts\b' crates/`): graft-ops `preview.rs:152` feeds `put_back::forecast`; app `update/preview/line_rows.rs:252` `forecast_rungs` matches only `StopsAt` (merge → none); `plan_lines.rs:67` `conflict_line` on the ordinary box; `boxes.rs:123` stash only; `view/rewrite_editor/right.rs:56,272` rewrite plans only; writer `kinds.rs:115` `conflicts_from`. Kinds unchanged (`Clean`/`Likely`/`NotChecked`). No wrong reader found.
- **Minor 13 + 1 docs** (§2.3b and §1.7, same sentence): "Over unrelated histories the box says `Can't merge: <from> and <into> share no history.` and withholds `y` (`m m` and the list's `y` too); `Esc` and `Enter` still work. A fast-forward says `Git combines  nothing: no merge commit is made` and `After y <into> moves up to <from>.` (ease-pass/05s)."
- **Q2:** walk `conflict_journeys.rs:1180-1200` `last_block_to_finished` is a merge (`m_m`) sending `o`, `y`, `y` (`review_then_commit`). Comment now "P11+P12 → 3 (`o ⏎ y`; in a merge since ease-pass/05b `o y y`: `y` opens the review, its `y` commits)".
- **Moves (old → new, names kept):** `update/finish/tests.rs::{c_on_a_resolved_merge_opens_the_summary_not_the_commit, c_with_a_file_still_open_is_refused_by_the_review_read, y_in_the_review_stages_the_files_the_read_found_solved}` → `update/finish/tests/review.rs` (05b tick :72 names two); `update/banner/tests.rs::c_on_a_shell_begun_merge_opens_the_summary` → `update/banner/tests/review_door.rs` (m6 ticket 20 :190); graft-ops `tests/it/finish.rs::gits_comment_lines_never_reach_the_summary` → `tests/it/finish/comment_lines.rs` (m6 ticket 22 :54); it `merge_review.rs::the_review_counts_only_the_unmerged_files_not_named_solved` → unit test above.
- **Touches:** `SummaryError` matches — `jobs/finish.rs` only (grep); `StillUnmerged` still from `finish_summary` (`finish::summary_refuses_with_anything_unmerged` green). `MergeLook {` built only in `merge_look.rs`; `MergeParts {` only `merge_view.rs` + ui tests. `finish = Some(Pending)` readers unchanged (door not built): keys.rs, view/finish.rs. Captures: only `merge-fast-forward-120.txt` new. Keymap/journal: no key ids or writes touched (no edits under keymap or journal).
- **Unverified:** colour of the fast-forward box; look of `docs/shots/ease-pass/merge-fast-forward-120.txt` for the user's eye.

## Handoff — the door (Minor 2; boxes At once, No flash, Ready/Failed, Banner `c`)
- **Blocked: needs 05t's** `tests/it/conflict_interim.rs:477`, `tests/it/finish/choices.rs`, `update/resolver/tests.rs` (`all_resolved_from_the_pane_offers_c_where_c_continues`), `update/resolver/tests/opens_itself.rs` (`click_on_the_finish_row_is_c`): they assert `finish.is_some()` right after the key — contradicting "nothing drawn between key and answer". Also outside both lists: it `finish.rs:74` (`summary_open`), it `finish/captures.rs`, it `resolver/{modal,shell_begun,opens_summary,editor_words}.rs`, `update/keys/live_c_tests.rs`, `update/resolver/tests/finish_here.rs` — ~34 failures with the door in.
- **WIP:** branch `ease-pass-05s-door-wip` at 33f226cb (red): `Picks.asked`, `finish::ask_review`, `resolver::review::{Door, review_door(rows: &[ListedFile]), clashes_still_open}`, `banner/pressed.rs::merge_door`, its tests. Next step: after 05t lands, rebase it onto this commit, re-point those tests to pump the read before asserting `finish`, run nextest.
- **Question (no line yet proving it wrong):** after the resolver closes, `resolver/leave.rs:51` takes `state.resolver`, so per-file states are gone; banner `c` with `resolver_seen` can then only ask the read, never refuse at once. Spec's banner rule may need "when the resolver is open".
