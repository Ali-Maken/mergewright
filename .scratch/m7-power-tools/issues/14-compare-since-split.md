# compare-since-split — `x` opens the compare at once, the diff is since the split, and the header says so in words

Status: done
Blocked by: 14a (`compare_span`)
Crates: graft (`update::detail` `toggle_compare`, `update::selection::follow_selection`, `jobs::detail` the compare
read, `view::detail::compare_header`, the base row's badges), graft-ui (`detail::Subject`, `detail_pane::header`,
`graph_screen::badges`). **Not** graft-ops (scout 14 §4: nothing there).
Source: note Decision 2; "Pass 2a" R6 (Compare); journey `pr-contents`; `docs/research/branch-compare-submodules.md`
takeaways 1, 4, 5, 6. Respecified from scout `.scratch/m7-power-tools/scouts/14.md`.
Build with: Sonnet. The behaviour is fixed below line by line; one read already exists (14a).
Scout: done (14). No second scout.
Needs: `graft_git::history_reads::compare_span`, `CompareSpan` (14a).

## Today (scout 14, with lines)
- First `x` pins `CompareBase` and **opens nothing** (`update/detail/state.rs:137-139`).
- Motion follows the cursor **only if the pane is shown** (`update/selection.rs:146`); on the base row it returns no
  command and the pane keeps the old compare (`:155`).
- Second `x` on another row opens `Compare{base, head}` and keeps the base (`state.rs:145`); same row clears (`:141`).
- Diff is **two-dot**: `DetailSpec::Compare → DiffSpec::Range{base, head}` (`state.rs:102`) → `diff-tree base head`.
- Header `COMPARE <base> → <head>` (`graft-ui/src/detail_pane/header.rs:20`, from `graft/src/view/detail.rs:156`).
- No `x base` badge; the base row has only the `▏` rule (`graft-ui` `render.rs:473-480`).
- Keymap §2.3 `x` already reads "diffs since they split" (`docs/design/02-keymap.md:212`). No keymap edit.

## Decided
- **First `x` opens the pane.** No base → pin it **and** open `DetailSpec::Compare { base, head: base }` (shows the
  base line below). This is what makes journey `pr-contents` = `x`, motion from a closed pane. *Behaviour change:*
  today's first `x` is pin-only.
- **Live.** With a base pinned and the pane shown, every cursor move opens `Compare { base, head: cursor }`
  (unchanged). **On the base row** it opens `Compare { base, head: base }` too (today: no command).
- **Second `x`:** same row → clear (unchanged); another row → today's meaning, opens that compare (now a re-read of
  what is shown). `Esc` clears and closes (unchanged, `update/detail/keys.rs:34`). Shift/Ctrl+click (`mouse.rs:133,
  223`) and the context menu (`context_menu.rs:222`) go through `toggle_compare`, so they change with it.
- **Diff range.** The compare read runs `compare_span(base, head)` first, in the same job, same `Stop`:
  - `Since { split, .. }` → files and each file's diff from `DiffSpec::Range { base: split, head }` (three-dot).
  - `NoSharedHistory` → `DiffSpec::Range { base, head }` (two-dot), said in words.
  - The **per-file diff** uses the split the file list used, never a fresh `base` (the file list and a file's hunks
    must be the same range). Hold the span beside the compare's file list; a file diff before it lands waits.
- **Header words** (detail pane top, first line). Names: C's = the first local-branch badge on the head row, else
  its first remote badge, else sha7; B's likewise for the base row.
  - `head == base` → `this is the compare base · x clears it` (no files).
  - `Since`, `ahead > 0` → `changes on <C> since it left <B> · N commits · F files +a −d` (`1 commit` singular;
    `F/+a/−d` summed from the file list, binary files count to `F` only).
  - `Since`, `ahead == 0` → `<C> is already in <B> · nothing of its own` (no files).
  - `NoSharedHistory` → `<C> and <B> share no history · showing <B>..<C>`.
  - Span not landed → `comparing <C> with <B> …`. Read failed → `couldn't compare: <error>` (rule 3).
  - Narrow pane ladder: drop `· F files +a −d`, then `· N commits`, then cut `<C>` with `…`. Never cut a number.
- **Badge.** The base row gets a badge `x base` (new `BadgeKind::CompareBase`), first in its badge list; the `▏`
  rule stays. Distinction: word + position. Colour unverified.
- **Footer** `compare base <sha7> — move, x again clears` (`view/rows.rs:237`): unchanged.
- No two-dot toggle, no new screen.

## Read first
- Note Decision 2, "Pass 2a" R6, "Journey counts" `pr-contents`. `docs/design/02-keymap.md` §2.3 `x`.
- Scout 14 (all).
- `crates/graft/MAP.md`: `AppState::compare_base`, `toggle_compare`, `follow_selection`, `DetailSpec`, `jobs::detail`.
  `crates/graft-ui/MAP.md`: `Subject`, `DetailPane`, `graph_screen::badges`. `crates/graft-git/MAP.md`
  `history_reads` (14a's lines).

## Signatures
```rust
// graft-ui::detail — replaces `Compare { base, head }` (one variant, not two).
pub enum Subject { None, Commit { sha: String }, Compare(CompareHeader), WorkingTree { /* unchanged */ } }
pub struct CompareHeader { pub base: String, pub head: String, pub state: CompareState }
pub enum CompareState {
    Reading,
    IsBase,
    Since { commits: usize, tally: Option<FileTally> }, // tally None until the file list lands
    AlreadyIn,
    NoSharedHistory,
    Failed(String),
}
pub struct FileTally { pub files: usize, pub added: usize, pub removed: usize }
// graft-ui::graph_screen::badges
pub enum BadgeKind { /* existing */ CompareBase }
// graft: calls graft_git::history_reads::compare_span(repo, base, head, stop)
```
Existing field types on `Subject::Commit`/`WorkingTree` are whatever they are today; keep them.

## Acceptance (each box names its test at tick time)
- [x] `update::detail::tests::the_first_x_opens_the_compare_on_the_base` — red first: `panicked at update/detail/tests.rs:256` (commands empty).
- [x] `update::tests::the_diff_follows_the_cursor_after_x` — pane closed, `x`, `j` → `Compare { base, head }` shown.
- [x] `update::selection::tests::the_cursor_on_the_base_says_x_clears` — red first: `panicked at update/selection.rs:541` (no command).
- [x] `jobs::detail::compare_tests::the_compare_diff_is_since_the_split` — red first (two-dot): `left: ["a.txt", "b.txt", "m.txt", "shared.txt"]` — feature-a 3 commits off main, main moved
      on by 2 → only feature-a's files listed.
- [x] `jobs::detail::compare_tests::a_file_diff_in_a_compare_uses_the_split` — red first (two-dot): hunk held `main-only` removed — the hunks of a file are the split..head hunks.
- [x] `graft-ui` `detail_pane::tests::the_header_says_changes_since_it_left`, `an_ancestor_says_nothing_of_its_own`,
      `no_shared_history_says_two_dot_in_words`, `a_failed_compare_says_why`,
      `the_header_drops_the_tally_first_when_narrow` (graft-ui header tests).
- [x] `the_base_row_carries_x_base` — graft-ui `graph_screen::tests::the_base_row_carries_x_base` (text) + app `m7_walk::pr_contents::the_base_row_carries_x_base` (first on main's row, `▏` kept, drag from `main]` picks `main`, `x base` picks nothing) — red first: `no x base on main's row: ◆   ▏  [◆ main] Readme …`.
- [x] Existing compare tests updated, each named in NOTES: `update/detail/tests.rs:256-346`,
      `update/selection.rs:444-545`, `update/tests.rs:599-719`, `detail/landing/tests.rs:328-377`,
      `detail/cursor/tests.rs:240`, `place/tests/pinned_base.rs` (blame trail sets aside and restores the base,
      `place.rs:215,229`, `place/trail.rs:39,145`: still green, unchanged meaning).
- [x] **Walk** `m7_walk::pr_contents::pr_contents`: fixture with feature-a's tip one row from main; cursor on main, pane closed; `x`, `j`;
      key count ≤ `noted_keys("pr-contents")` (2). Header reads `changes on feature-a since it left main · 3 commits`.
- [x] **Captures** (`m7_walk::pr_contents::capture_compare_since_split`, re-taken with `x base`): `docs/shots/m7/compare-since-split-187x50.txt` and `-80x24.txt`. **Right:** the header words
      above (at 80x24 the ladder's cut, numbers whole); `x base` on main's row; the file list holds only feature-a's
      files. Colour unverified. `gallery.rs:419-423` (`x j j x`, expects `COMPARE 0683985 → e30e870`) re-taken to the
      new header; `docs/shots/m2-compare.txt`, `m2-05-feature-branches-x-j-j-x-160x40-unicode.txt` re-taken if a test
      writes them, else listed in NOTES as stale (not hand-edited).
- [x] Hint-bar tests green (full nextest 4244/4244). MAP.md for graft and graft-ui. TESTING.md row for `pr-contents`.

## Touches (check each, report how)
- **Blame trail** (`Origin.base`, set aside and re-pinned): the re-pin now opens the pane? It must not — re-pinning
  restores, it does not press `x`. Check: `place/tests/pinned_base.rs` + one new assertion that leaving the trail
  lands on the same compare header.
- **Diff pane keys (`w W v [ ] < > L`, `B` blame from a compare):** unchanged. Check: diff-pane tests; `B` on a
  compare file blames `head` (grep `Compare` in `update/place*`, report the hit).
- **Mouse and context menu:** through `toggle_compare`. Check: the mouse compare test (`mouse.rs:133` path).
- **Hint bar / footer:** footer line unchanged; bar under a compare unchanged. Check: hint tests.
- **Ghost Preview rows** carry no badge: `debug_assert` in `render` forbids `compare_marked` on a ghost row; the
  `x base` badge is built from the same `compare_base` check, so ghost rows can't get it. Check: grep the badge's
  builder.
- **Keymap, journal/undo, other crates:** none (read only; §2.3 already worded).

## Rules / Gates
As rewrite-engine.

## Size
Medium (~45k).

## NOTES (builder, 2026-09-27)
- Scout 14 used. Own worktree `.claude/worktrees/m7-14`, branch `m7-14-compare` (the harness's worktree was removed mid-task).
- **Deviation, accepted by conductor:** the span rides on `FileListView::compare` (`CompareSummary::Since { split, commits }` / `NoSharedHistory`), not in a separate state. `read_file_diff` re-runs `compare_span`: fixed oids → same merge-base; cost one `git merge-base` spawn per file diff.
- `CompareHeader.base/head` are `SanitizedText`, not `String` (graft-ui rule: every string arrives sanitised).
- `x` second press on another row when the pane already follows it: `open` of the shown spec asks for nothing (not a forced re-read).
- `DetailState::compare_followed` added: without it `settle`'s follow undid a Shift/Ctrl+click compare while the cursor sat on the base row (the new base-row rule). An unmoved cursor leaves a compare-against-this-base alone. A `show_line` landing that opened the base as a plain commit keeps it.
- First `x` keeps focus where it was so `j` moves the graph (else focus went to Files).
- **Resumed (grant of `view/entries.rs`, `update/geometry.rs`, `update/place/from.rs`):** badge pushed first in `view/entries.rs::row_entry` when `compare_marked`; `update/geometry.rs::badge_at` subtracts 1 on the row the drawing marks (`base.row == history row`, same test as `view/entries.rs:137`). `B` on a compare's old side blames `split` (`place/from.rs`, test `update::place::from::compare_tests::an_old_side_line_of_a_compare_blames_the_split`, red first: `left: …0101… right: …0909…`); `blame_from` gains `split: Option<Oid>`, so its one caller `update/place.rs:200` changed (one line, outside the grant — needed for the signature).
- **Was blocked — `x base` badge placement:** push `Badge { kind: CompareBase, .. }` first in `view/entries.rs::row_entry` when `compare_marked`; **and** `update/geometry.rs::badge_at` must subtract 1 from the index on that row (it indexes `refs.on(at)` with the drawn index — a first extra badge would make dragging a branch badge on the base row pick the wrong ref). Neither file in my set.
- **Fixed since (see above) — `B` on an old-side line of a compare:** `update/place/from.rs:86` blames `base` with the line number, but the old side is now `split`. Fix: read `split` from `detail.files` (`CompareSummary::Since { split, .. }`) there. Not in my set.
- Tests changed for the behaviour change: `update/detail/tests.rs` (renamed `toggle_compare_with_no_base_pins_one_and_emits_nothing` → `the_first_x_opens_the_compare_on_the_base`); `update/selection.rs` (`follow_selection_back_onto_the_base_row_does_nothing` → `the_cursor_on_the_base_says_x_clears`); `update/tests.rs` (`x_on_a_row_with_no_base_pins_it_and_sends_nothing` → `..._and_opens_the_pane`; `x_on_a_different_row…`, `shift_click…`, `ctrl_click…` — size 160x20 and no "asks for its diff", the compare is already shown); outside the plan's list: `update/context_menu/tests.rs:347`, `update/place/tests/esc_back.rs:~290` (pin now opens the pane on the base), `view/tests.rs:1159` (header words), `update/operate.rs` test literal and `update/detail/cursor/tests.rs` (`compare: None`), `docs/capabilities/m2-details-and-diffs.md` M2-08 (dead citation, registry test). `detail/landing/tests.rs`, `detail/cursor/tests.rs:240`, `place/tests/pinned_base.rs` unchanged in meaning and green; `pinned_base.rs::esc_from_blame_restores_the_compare_base` gains the same-header assertion.
- Captures: new `docs/shots/m7/compare-since-split-187x50.txt`, `-80x24.txt` (80x24 needs `Tab` to show the pane tab, not counted; the whole header fits in 80 so the ladder's cut shows only in `the_header_drops_the_tally_first_when_narrow`). **Stale, not written by any test:** `docs/shots/m2-compare.txt`, `docs/shots/m2-05-feature-branches-x-j-j-x-160x40-unicode.txt`, `-80x24-unicode.txt`. Gallery caption (`gallery.rs`) re-worded, not re-taken.
- **Compacted** once mid-ticket (context re-read after). Nothing remains.
- 05-held files touched, minimal: `update/operate.rs` one line `compare: None,` in the test `FileListView` literal (~:1355); `view/tests.rs:1131,1159-1160` (doc comment + assertion now `comparing`).
