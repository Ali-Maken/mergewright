# 21b-7 — 21b-5's remainder: update/tests parent, consequence.rs, 5+1

Status: 21b-7a done (builder, Opus, 2026-10-09); 21b-7b open. Was: draft (planner spec-only pass 2026-10-09; from 21b-5 NOTES "Open, not decided" / "Not decided" and box 5+1)
Build with: per part, below. No Fable: no public signature another ticket builds on; every item is judged by a test,
`nextest list` or `wc -l`.
Needs: 21b-5 merged (`472e433e`). Runs **before 21b-6** (see Order).

Builders never read KNOWN_GAPS.md. Line numbers below are from the 21b-5 scout (`scouts/21b-5.md`, at `a51f9cc3`) or
21b-5 NOTES; **re-locate each by name at HEAD** and say the line found in NOTES. The move-only check is 21b's
(`21b-hygiene-round-2.md` "Move-only check", steps 1–6), with step 5's target restated per item below.

## Rulings (planner)

1. **Purity scan: exempt test modules by module tree, guarded.** Not a `fixtures.rs` name exemption: a skip by file
   name would also skip a production file that took that name, and the guard would not notice. The scan skips a file
   iff, relative to `src/update/`, it is named `tests.rs` **or** lies under a directory named `tests`. The guarantee
   is kept by a second check: every `mod tests` declared in a scanned file is directly under `#[cfg(test)]`. So every
   skipped file is reachable only through a `cfg(test)` module. This also closes a hole the old name-only rule had.
   Then the parent's fixtures move to `update/tests/fixtures.rs`. The two lines 21b-5 hoisted to dodge the scan go
   back where they were.
2. **Over-400 files 21b-5 did not name.** `undo/consequence.rs` 419 is production under `update/undo/`, which item 1
   capped ("production files ≤ 400"). **Split it now.** `undo/tests/set_aside.rs` 436 and `update/tests/keymap_rules.rs`
   435 are test files that predate 21b-5 and no row names them. **Don't split them.** The conductor names both in
   `M8/files-over-400-unrowed` (→ 25), as review m8-bars-hygiene Minor 6 did for its two. Why: about 80 test files
   are over 400 (`view/tests.rs` 5261). Trimming two by about 35 lines each buys nothing until 25 ranks the class.
3. **5+1 restated.** The review's seams (m8-bars-hygiene Major 1) cannot bring `layout.rs` (828, production to :608)
   or `view/rows.rs` (1215, production to :1103) under 400. The target for those two: the named seam plus their
   inline tests moved out (step 5's "inline tests first"). They stay over, and the conductor names them in
   `M8/files-over-400-unrowed`. `footer.rs` and `catalogue.rs` must reach ≤ 400. A file still over after its one seam
   is a **stop** for that file: give the count and the seam you'd need. Don't add a second seam.

## Parts, crate sets, order

| Part | Crate set (disjoint) | Build with | Size |
|---|---|---|---|
| **21b-7a** graft | graft: `update/tests.rs`, `update/tests/{purity,fixtures}.rs` (+ the children that take back the hoisted lines and `shift_click…`), `update/undo/consequence.rs` (+ child), `view/rows.rs` (+ `view/rows/` children), MAP.md graft | **Opus**: it edits the purity guard; a wrong predicate weakens `update`'s guarantee silently, and the red checks need judgment | ~60k |
| **21b-7b** graft-ui | graft-ui: `graph_screen/layout.rs`, `graph_screen/footer.rs`, `graph_screen/hints/catalogue.rs` (+ children), MAP.md graft-ui | Sonnet: move-only, judged by `nextest list` + line-set check | ~40k |

7a ‖ 7b may run as one wave (two helpers, disjoint crates). **Scout:** brownfield (ADR-0035), one ≤1-page scout per
part. 7a: `update_sources`' file filter as written, the parent's sections with `wc -l`, `consequence.rs`'s fn list,
`view/rows.rs`'s five fns and their private helpers. 7b: `layout.rs` header-tail items, `catalogue.rs` fn list.

**Order with 21b-6: 21b-7 precedes it.** Not a correctness dependency, since the lint gate catches either order. But
21b-6's blocker is "all of 21b-1…5 merged", and 21b-5 is not done until this lands. Both edit the same files: 7a's and
7b's new files carry moved `#![allow(…)]` preludes. Sweeping after the moves means 21b-6 counts and fixes the final
layout once, and its "diff is only allow lines" check stays clean. They never run together (21b-6 runs alone).

---

## 21b-7a — graft

**Read first:** this file; 21b-5 NOTES (`21b-hygiene-round-2.md:574–585`); `crates/graft/MAP.md:363` (update::tests
layout), `:461` (`consequence`), `:589`, `:955` (purity guard), `:223` (`cursor_is_stash`/`cursor_row_context`),
`:784`, `:951` (`back_to_place`); the scout. List `fn`/`mod`/`impl` lines; don't read files whole.

**Items**
1. **Purity scan** (ruling 1). Move `update_cannot_reach_the_world` and `update_sources` from `update/tests.rs` to
   `update/tests/purity.rs` (the child exists). Change the filter to `is_test_module` below. Add the `cfg(test)` check.
   **Order:** first write `the_scan_skips_test_modules_only` and `every_tests_module_is_cfg_test` with the old filter
   in place. The predicate table must fail on `tests/purity.rs → true`. Then change the filter.
2. **Parent ≤ 400.** The 17 shared fixtures (MAP `:363`: `key` … `resolver_card_on_screen`) go to
   `update/tests/fixtures.rs`. If that file would pass 400, split it by kind (`fixtures/screens.rs` etc.), each
   ≤ 400. The parent re-exports them by explicit list (`pub(crate) use fixtures::{…}`), so `crate::update::tests::key`
   etc. compile unedited. `shift_click_on_a_second_row_opens_the_compare` goes to `copy_and_compare.rs`, or to
   `pointer.rs` if that would put `copy_and_compare.rs` over 400. `use std::time::Duration` goes back to
   `commit_graph_offer.rs`, and the fn-local `use ratatui::layout::Rect` back into `the_mirror_agrees_at`, both as
   before 21b-5. Re-imports of helpers that children share stay in the parent only if a child names them through
   `super::`. The parent keeps `mod` lines, the use-block prelude and the re-exports.
3. **`undo/consequence.rs` 419 → ≤ 400** (ruling 2). First, if it has inline tests, move them to
   `undo/consequence/tests.rs` (11f B1). If it is still over 400, split on MAP `:461`'s two concerns: the landing after
   a forced run goes to `undo/consequence/landing.rs`, and the drift-box words with their fit rule stay.
4. **`view/rows.rs`** (5+1, ruling 3): inline `mod tests` (:1104) → `view/rows/tests.rs`. The hint-context
   classification goes to `view/rows/context.rs`: `cursor_row_context` (:850), `cursor_is_stash` (:884),
   `search_wrapped` (:371), `live_wrapped` (:584), `back_to_place` (:684, `pub(crate)`), plus any private helper only
   they call. `back_to_place` is re-exported so `crate::view::rows::back_to_place` holds.

**Signatures**
```rust
// graft src/update/tests/purity.rs
/// A file the scan skips: `tests.rs`, or anything under a `tests/` directory below `src/update/`.
/// Sound only with `every_tests_module_is_cfg_test`: it can't see a `#[path]` mod pointing into a `tests/` dir.
fn is_test_module(relative_to_update: &Path) -> bool;
#[test] fn update_cannot_reach_the_world();           // moved, body unchanged but for the filter call
#[test] fn the_scan_skips_test_modules_only();        // new, table
#[test] fn every_tests_module_is_cfg_test();          // new
// graft src/update/tests.rs
pub(crate) use fixtures::{key, merge_on_screen, asked_view, core, /* …the 17, MAP :363 */};
// graft src/view/rows.rs
pub(crate) use context::back_to_place;                // path unchanged for callers
```
Every other item: no signature change. `pub(super)` widening only where a sibling needs it.

**Acceptance**
- [x] 1a `update::tests::purity::the_scan_skips_test_modules_only` is a table. False: `undo.rs`,
      `undo/consequence.rs`, `undo/box_answers.rs`, `mod.rs`. True: `tests.rs`, `undo/tests.rs`, `tests/purity.rs`,
      `undo/tests/set_aside.rs`, `tests/keymap_words/stops.rs`. **Seen red** on the old filter (NOTES: the failing
      row).
- [x] 1b `update::tests::purity::every_tests_module_is_cfg_test`: in every scanned file, each `mod tests` (`;` or `{`)
      has `#[cfg(test)]` as its previous attribute line. **Seen red** once: delete `#[cfg(test)]` above `undo.rs`'s
      `mod tests` and run. NOTES gives the message, which names file:line. Restore it.
- [x] 1c The guarantee holds: `update::tests::purity::update_cannot_reach_the_world` is green. **Seen red** once: put
      `eprintln!("x");` in a fn in `undo/consequence.rs`, run, NOTES the message, remove it. The scanned-file count
      before and after goes in NOTES. The two counts differ by exactly the files under `update/**/tests/` not named
      `tests.rs`: `find crates/graft/src/update -path '*/tests/*' -name '*.rs' ! -name tests.rs | wc -l`, quoted.
- [x] 2 `wc -l` on the committed files: `update/tests.rs` ≤ 400, `fixtures*.rs` each ≤ 400, receiving children
      ≤ 400 (quote each). `rg -n 'Duration|ratatui' crates/graft/src/update/tests.rs` empty. Code naming
      `crate::update::tests::` compiles unedited: the `git diff --stat` of item 2's commit lists only `update/tests*`.
- [x] 3 `wc -l` committed: `consequence.rs` and its children each ≤ 400. Undo tests green unedited:
      `cargo nextest run -E 'test(/update::undo/)'`, count in NOTES.
- [x] 4 `wc -l` committed: `view/rows/context.rs`, `view/rows/tests.rs` ≤ 400; `view/rows.rs` before → after
      (stays over, per ruling 3). `rg -n 'rows::back_to_place' crates/graft/src` callers compile unedited.
- [x] 5 Move-only check per moved file (steps 1–6). Run `nextest list --workspace | sort` before and after: the
      count rises by exactly 2 (1a, 1b), and test-fn names are otherwise identical. Purity tests, the
      `shift_click…` test, rows tests and consequence tests change path by design: list each. The line-set
      exceptions are listed, including the two hoisted lines returning. `git diff --stat <base> -- docs/shots` is
      empty, with no `.snap.new`.

**Touches** (each checked, say how in NOTES):
- Fixture path users: `rg -n 'update::tests::' crates/graft/src` (compile is the check).
- Purity guard docs: `rg -n 'update_cannot_reach_the_world' --glob '*.md'`. Update the living docs (graft MAP
  `:589`, `:955`, `:363`; ARCHITECTURE.md if it hits). Leave records alone (reviews, log, `.scratch`,
  `TESTING.md:3166`).
- `tests/no_literal_keys.rs` allow-list entries naming `view/rows.rs` or `consequence.rs`: re-point them if a literal
  moved (21b-1 precedent).
- Undo/journal behaviour: none (moves). Keymap: none. MAP graft layout lines for every new file.
- For the conductor: rows to name in `M8/files-over-400-unrowed` are `undo/tests/set_aside.rs` 436,
  `update/tests/keymap_rules.rs` 435 and `view/rows.rs` (after). `M8/doorway-files-over-400` closes on green.


**Ticked (21b-7a):**
- 1a `update::tests::purity::the_scan_skips_test_modules_only` — table of the 9 rows. Seen red on the old filter
  (`file_name == "tests.rs"`): `the scan's filter is wrong on: ["tests/purity.rs → want true",
  "undo/tests/set_aside.rs → want true", "tests/keymap_words/stops.rs → want true"]`. Green after.
- 1b `update::tests::purity::every_tests_module_is_cfg_test`. Seen red: **deleting** `#[cfg(test)]` above
  `undo.rs`'s `mod tests` does not compile (the lib build then pulls the test module in: 11 errors, e.g. `cannot find
  function notice_line`), so the red run replaced it with `#[cfg(all(test))]` — still test-only, not the literal:
  "undo.rs:273 declares `mod tests` without `#[cfg(test)]` directly above it: the purity scan skips test modules,
  so each must compile only into tests" (left `Some("#[cfg(all(test))]")`). Restored; green.
- 1c `update::tests::purity::update_cannot_reach_the_world` green. Seen red: `eprintln!("x");` in
  `undo/consequence.rs` `cut_end` → `` `update` must stay pure, but its module mentions `println!` `` (substring of
  `eprintln!`). Removed. Scanned-file count, same tree (commit 258203f1), via a temporary `>= 100_000` floor: old
  filter **261**, new filter **159**; difference 102 = `find crates/graft/src/update -path '*/tests/*' -name '*.rs'
  ! -name tests.rs | wc -l` → **102**. (At base 610ba5ee, before `purity.rs` existed, the old filter found 260.)
- 2 `wc -l` committed (868dc4f5): `update/tests.rs` 687 → **128**; `tests/fixtures.rs` **212**,
  `tests/fixtures/screens.rs` **205** (split by kind: one file would be ~412); `copy_and_compare.rs` 196 → **238**,
  `commit_graph_offer.rs` 381 → **382**, `tab_strip_geometry.rs` 233 → **235**. `rg -n 'Duration|ratatui'
  crates/graft/src/update/tests.rs` → empty. `git show --stat` of item 2's commit: only `update/tests.rs` and
  `update/tests/*` (6 files). `crate::update::tests::{key, core, …}` users compile unedited (gates green).
- 3 `undo/consequence.rs` 419 → **289**; `undo/consequence/tests.rs` **129**. Inline tests moved only; the
  `landing` split was not needed. `cargo nextest run -E 'test(/update::undo/)'`: 56 run, 56 passed (56 before).
- 4 `view/rows.rs` 1215 → **893** (stays over, ruling 3); `view/rows/context.rs` **222**, `view/rows/tests.rs`
  **109**. `rg -n 'rows::back_to_place' crates/graft/src` → no hits (no caller outside `rows.rs`); the
  `pub(crate) use context::back_to_place` keeps the path. `view::rows` tests 5 run, 5 passed (5 before).
- 5 `nextest list --workspace | sort`: 6203 → **6205** (+2: 1a, 1b). Path changes, all by design:
  `update::tests::update_cannot_reach_the_world` → `update::tests::purity::…`;
  `update::tests::shift_click_on_a_second_row_opens_the_compare` → `update::tests::copy_and_compare::…`. The rows
  and consequence tests keep their paths (an inline `mod tests {}` moved to `tests.rs` is the same module).
  `git diff --stat 610ba5ee -- docs/shots` empty; no `.snap.new`; no insta snapshots in the moved tests.

## NOTES — 21b-7a (builder, Opus, 2026-10-09)
No compaction. Scout: `scouts/21b-5.md` (the conductor's pointer; no 7a scout file exists) plus my own greps of the
named fns. Base 610ba5ee. Commits: 258203f1 (purity), 868dc4f5 (item 2), e5ad035d (consequence), 82969fab (rows), then
MAP + ticket. Gates on 82969fab (worktree, workspace-wide): fmt ok, clippy `-D warnings` ok, nextest 6205/6205 pass
(14 skipped), `cargo test --doc` ok.
**Re-located at HEAD:** `update_sources`/`update_cannot_reach_the_world` parent `:483/:527`; fixtures `:38–238`
and `:267–466`; `shift_click…` `:474`; `consequence.rs` `mod tests` `:289`; rows `search_wrapped` `:371`,
`live_wrapped` `:584`, `back_to_place` `:684`, `cursor_row_context` `:850`, `cursor_is_stash` `:884`, plus
`row_verbs` `:899` and `rewrite_offered` `:1013` (called only by `cursor_row_context`, so they moved too; `live_c`
stays, `overlay_context`/`hint_context` call it). Inline tests `:1104`.
**Ticket said "the child exists" for `tests/purity.rs`:** it did not (MAP listed it; 21b-5 had tried it and backed
out). Created.
**Line-set check** (non-blank new lines absent from the old file): `use`/`mod`/`//!`; visibility `pub(super)` →
`pub(crate)` on the 13 fixtures in `fixtures.rs` and 3 in `screens.rs` (reach unchanged: `update::tests` is private
to `update`), `facts` private → `pub(super)` (`opening.rs` names it), rows' `search_wrapped`/`live_wrapped`/
`cursor_row_context` → `pub(super)`; the two hoisted lines back where they were before 21b-5
(`use std::time::Duration;` in `commit_graph_offer.rs`'s use block, `use ratatui::layout::Rect;` first line of
`the_mirror_agrees_at`); dedent of the two inline test bodies. Purity: `update_sources` body now maps
`scanned_files` (the walk, shared with 1b, filter `is_test_module(relative)`, results sorted); its loop body is the
old one; the old `if name == "tests.rs"` is the filter call. Dropped: two stale comment lines in the parent (`//
Search (m1-the-graph/26).`, an orphan divider; `// Kept here, not in copy_and_compare…`, now false).
**Touches:** fixture path users — `rg 'update::tests::' crates/graft/src` (12 files) compile unedited; purity docs
— MAP `:589` path updated, `:363` layout line rewritten, new `update::tests::purity` row, `consequence` row, new
`view::rows` files row; ARCHITECTURE.md: no hit. `no_literal_keys` allow-lists (`crates/graft-ui/tests/
no_literal_keys*`): no entry names `view/rows.rs` or `consequence.rs`. Undo/journal, keymap: none (moves; 56 undo
tests green). Captures: shots diff empty.
**Found, not done (for the conductor):**
- `shift_click_on_a_second_row_opens_the_compare` still prints `eprintln!("DBG …")` in a loop — a debug leftover,
  moved verbatim (move-only). It is why 21b-5 kept it in the parent. Should go.
- Comments naming the old path `update::tests::update_cannot_reach_the_world`: `config/tests.rs:28`,
  `view/resolver/tests.rs:6`, `update/preview/rect.rs:34`; and `update/resolver/tests.rs:667` "Kept here, not in
  `file_list.rs`: `update_cannot_reach_the_world` skips …" may now be stale (the scan skips `tests/` dirs too). Left
  (outside this part's file list).
- Rows-to-name for `M8/files-over-400-unrowed`: `undo/tests/set_aside.rs` 436, `update/tests/keymap_rules.rs` 435,
  `view/rows.rs` 893. `M8/doorway-files-over-400`: closes on green per the ticket.
- Moved doc comments in `view/rows/context.rs` link `[`hint_context`]` (in `rows.rs`), unresolved from the child
  under `cargo doc --document-private-items`; not a gate, left verbatim.

---

## 21b-7b — graft-ui (5+1 rest)

**Read first:** this file; `docs/reviews/m8-bars-hygiene.md` Major 1 (:92–114); `crates/graft-ui/MAP.md:22` (hints
files, `catalogue.rs`), `:57`, `:199` (`searching_form`, `pane_focused_form`); the scout.

**Items**
1. **`graph_screen/layout.rs`** (828): the header tail goes to `graph_screen/header_tail.rs`: `TAIL_SEPARATOR`
   (:238), `enum Drop` (:245), `struct TailItem` (:253), `fitted_tail` (:263–~303), as the review states it.
   `search_item`/`tail_items` (:152–) stay. Inline tests (production ends :608) go to `layout/tests.rs`.
   `layout.rs` stays over (ruling 3).
2. **`graph_screen/footer.rs`** (430): inline `mod tests` (:284) → `footer/tests.rs`. `footer.rs` ≤ 400.
3. **`graph_screen/hints/catalogue.rs`** (635, no tests): the row-form wrappers go to `catalogue/forms.rs`. That
   means `searching_form`, `operation_live_form`, `pane_focused_form`, `over_rewritable` and the helpers only they
   call. `every_context`/`every_hint` stay. Both files ≤ 400, else stop (ruling 3).

**Signatures:** none change. Public paths re-exported from their old module.

**Acceptance**
- [ ] 1 `wc -l` committed: `header_tail.rs`, `layout/tests.rs` ≤ 400; `layout.rs` before → after. The
      `graph_screen::layout` tests pass unedited but for `use` lines (count in NOTES).
- [ ] 2 `wc -l` committed: `footer.rs`, `footer/tests.rs` ≤ 400. The `graph_screen::footer` tests pass, same count.
- [ ] 3 `wc -l` committed: `catalogue.rs`, `catalogue/forms.rs` ≤ 400. `every_hint`/`every_context` users pass
      unedited: graft `update::tests::hint_coverage::*` and graft-ui `hints::tests` (counts in NOTES).
- [ ] 4 Move-only check per file (steps 1–6). `nextest list --workspace | sort`: count unchanged and names
      identical. Only the moved inline tests change path: list them. `git diff --stat <base> -- docs/shots` is
      empty, with no `.snap.new`. Insta snapshots move beside their tests.

**Touches:** header captures (any `docs/shots/m8/*` with a search tail; the check is byte-identical through the
shots diff); `tests/no_literal_keys.rs` allow-list paths for `catalogue.rs`, `layout.rs`, `footer.rs`; MAP graft-ui
`:22` and the layout lines; keymap: none. For the conductor: `layout.rs` (after) → `M8/files-over-400-unrowed`.

---

## Gates (both parts)
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, run
workspace-wide. One commit per file split; the purity change (7a item 1) is its own commit, before item 2.
