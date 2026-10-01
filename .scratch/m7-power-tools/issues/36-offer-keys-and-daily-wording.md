# 36-offer-keys-and-daily-wording — ticket 15 built, plus `B` in the diff pane always blames

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M2 (15 never built; rows 766/768/769/770 seen) and M3 (`B` in the
diff pane writes a commit-graph while the offer is up). **Supersedes 15**: 15's spec stays in place as the decision
record and is read as this ticket's scope; the conductor marks 15 `superseded by <this number>`.
Build with: **blank — scout first.** 15's scout (`scouts/15.md`) read the code at `9f8dde7b`; `update/preview.rs` has
since split (graft MAP :1064, 30m) and the 30-series touched the remote and commit-editor paths, so every line 15
names may have moved. Suggested after the scout: Sonnet (words and conditions, each decided below).
Scout: **yes, one, ≤1 page**: (a) re-find every site 15's "Decided" names, new path:line beside the old; (b) M3's
sites: `update/detail/keys.rs:77-80`, `place::b_key`, `place::blamable` (graft MAP :684), where the offer's `B`/`X` are
claimed on the graph, and how `AppState::commit_graph_line` builds the offer row (keys a separate part or one string).
Needs: none.
Blocked by: none
Crates / files (disjoint from the other fix groups; wave 1):
- graft-ops: `apply/remote.rs`, `mistake_radar/` (as 15).
- graft: `update/commit_editor*`, `view/commit_editor*`, `update/commit_graph.rs`, the topo-sort job's timing,
  `preview/boxes.rs` (or wherever `closing_box`/`ready_box` now live — **if that is `preview/answer.rs`, report it
  before building: 37-preview-top-row-and-left-behind owns that file**, and the conductor sequences), `view/radar.rs`,
  `update/detail/keys.rs`, `update/place.rs` (`blamable`, `b_key`), the tests 15 lists.
- graft-ui: `name_prompt.rs`, `preview_box.rs` (as 15). Not `graph_screen/hints/*` (40-hint-take-and-amend owns it).
- graft-core: one doc line (as 15).

## Scope
In:
1. **All of 15's "Decided" (exact words, unchanged):** :774 rejected-push words, :776 `subject empty` only after a
   refused commit, :777 no radar finding for a branch without upstream, :778 the commit-graph offer only after a sort
   that took ≥ 300 ms this run, :856 name prompt keys drawn once, :860 single-`[ Close ]` boxes offer only `esc`.
   15's "Rows closed, not built" (:775, :779) stand: the conductor moves them.
2. **M3 (decided here):** `B` in the detail pane (file list or diff) **always** means blame (keymap §2.5:277). The
   offer's `B` / `X` answer **only while the graph has the keyboard** (`AppState::graph_has_focus`). With the pane
   focused:
   - `B` goes to `place::b_key`'s blame path whatever the offer state; `X` is unbound there (not in §2.5) and swallowed
     as any unbound key is today.
   - `blamable` loses its "no commit-graph offer" condition, so the pane bar keeps `B blame`.
   - The offer row stays (it is a status, not an error), drawn **without its keys** while the graph lacks focus:
     `no commit-graph file: history sorts on every open`. Keys come back on `⇥` to the graph. Rule: keys are drawn only
     where they act (H4, §2.1 "the bar always shows what's active").
Out: any other offer change (once-per-repo decline flag, `harness.offer_commit_graph`, `Writing`/`Written` states:
unchanged). Keymap doc edit (below, for the conductor). The hint bar's graph-row ranking (40-hint-take-and-amend).

## For the conductor (spec change, **for the user**)
Keymap §2.1:27-29 says the offer's `B`/`X` "claim the keyboard the same way a modal pane does" — which §2.5's M7 `B`
(blame, ADR-0048) contradicts in the pane. Proposed replacement sentence, apply after the user's OK:
"`B`/`X` on the commit-graph offer are a named, narrow exception: one-shot initials, live **only while the graph has
the keyboard**; in the detail pane `B` is blame (§2.5) and the offer row is drawn without its keys." The code change
does not wait on it: it follows §2.5, the later decision.

## Signatures
```rust
// 15's, unchanged:
pub(crate) fn offer_after_sort(fact: &CommitGraphFact, sort_took: Duration) -> bool; // graft::update::commit_graph
pub(crate) const SLOW_SORT: Duration = Duration::from_millis(300);
// M3 — graft::update::commit_graph: the one predicate the key path and the offer row both read.
pub(crate) fn offer_keys_live(state: &AppState) -> bool; // offer is Offered && state.graph_has_focus()
// graft::update::place — signature unchanged, condition narrowed:
pub(crate) fn blamable(state: &AppState) -> bool; // pane shown and focused, a diff cursor (no offer clause)
```
graft-ops: the no-upstream `Risk` variant removed (as 15). graft-ui: `preview_box`'s `back_only` removed (as 15).

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] Every acceptance box of 15 (`15-daily-wording.md` "Acceptance"), its tests and captures as named there, with the
      scout's new line numbers for the old-word tests.
- [ ] `b_in_the_diff_pane_blames_while_the_offer_is_up` (red first): offer `Offered` via `harness.offer_commit_graph`,
      `⏎` on a commit, `⇥` to the diff, `B` → a blame place opens; `Command::WriteCommitGraph` not emitted.
- [ ] `the_offer_keys_answer_only_with_the_graph_focused`: graph focused, `B` → `WriteCommitGraph`, `X` →
      `DeclineCommitGraph`; pane focused, neither is emitted.
- [ ] `the_pane_bar_keeps_b_blame_while_the_offer_is_up` (red first).
- [ ] `the_offer_row_drops_its_keys_off_the_graph`: the drawn row with pane focus has no `B`/`X`; with graph focus,
      today's words byte-identical.
- [ ] Hint guards green: `every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here`.
- [ ] **Capture** `docs/shots/m7/diff-pane-b-with-offer-187x44.txt` — **right:** the blame place open on the file and
      line the diff cursor was on (title names the file; the who/when column on the cursor line), and the offer row
      under the graph **without** `B`/`X`; no `commit-graph written` anywhere. Colour unverified.
- [ ] MAP.md for graft-ops, graft, graft-ui (`blamable`, `CommitGraphOffer`'s keys, `offer_keys_live`).
- [ ] Gates.

## Touches (check each; report how)
- Everything in 15's "Touches", re-checked at the scout's lines.
- **`blamable` readers:** the pane bar's `B blame`, `place::b_key`. Grep `blamable(`; list each.
- **The offer row:** `commit_graph_line`, `graph_rows` subtracting its row (row count unchanged — only the keys drop),
  `view::rows::modal_blanked` (graft MAP :875). Check: the m1-54 offer tests green; grep `CommitGraphOffer::Offered`.
- **Graph-focused `B`/`X`:** unchanged; m1-54 tests green.
- **History place's `B`** (keymap §2.17:476) and the resolver's `B` (§2.6:293): not reached by this change (both are
  claimed before the pane). Check: their tests green.
- **Captures:** any fast-tier capture showing the offer row with the pane focused is re-taken by its test, listed.
- **Keymap doc:** the §2.1 sentence above — conductor, not builder. **Journal/undo:** none.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~65k (15's ~50k + M3 ~15k). One context. If the scout finds 15's sites widely moved, the conductor may split :856/:860
(graft-ui) into a second ticket; say so in the scout.
