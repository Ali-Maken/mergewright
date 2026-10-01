# 36-offer-keys-and-daily-wording — ticket 15 built, plus `B` in the diff pane always blames

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M2 (15 never built; rows 766/768/769/770 seen) and M3 (`B` in the
diff pane writes a commit-graph while the offer is up). **Supersedes 15**: 15's spec stays in place as the decision
record and is read as this ticket's scope; the conductor marks 15 `superseded by <this number>`.
Build with: **Sonnet** — words and conditions, every site found by scout 36 at `9133428c`, each decision made below.
Scout: done (`scouts/36.md`). No second scout.
Needs: none.
Blocked by: none. **Wave 1, concurrent with 37 and 38** (file sets disjoint, see "File set").
**Seam with 40-hint-take-and-amend:** 36 owns `graft-ui/src/graph_screen/footer.rs` and `graph_screen/mod.rs` in wave 1.
40 is already `Blocked by: 36`; if 40 needs either file it takes it after 36 merges. No concurrent overlap.

## File set (exactly these; nothing else without a report)
- graft-ops: `src/apply/remote.rs`, `src/apply/remote/tests.rs`, `src/mistake_radar/{rules,mod,explain}.rs`,
  `tests/it/mistake_radar.rs`, `MAP.md` (`apply` remote and `mistake_radar` sections only).
- graft-core: `src/mistake_radar.rs` (one doc line).
- graft src: `update/commit_editor.rs`, `update/commit_editor/tests.rs`, `view/commit_editor.rs`,
  `update/commit_graph.rs` (+ its test module: inline, or a new `update/commit_graph/tests.rs`), `update/state.rs`,
  `update/history.rs`, `update/events.rs`, `update/mod.rs`, `jobs/runner.rs`, `update/preview/boxes.rs`,
  `view/radar.rs`, `update/place.rs`, `update/place/tests/hints.rs`, `update/detail/keys.rs` (comment only),
  `update/remote/tests.rs`, `update/remote/tests/lease.rs`, `view/tests/bottom_row.rs`.
- graft tests/it: `main.rs` (one `mod` line — 36 is the only wave-1 ticket adding one), new `offer_and_blame.rs`,
  `remote_by_key.rs`, `m4_walk/push.rs`, `box_owns_the_bar.rs`, `m6_stash_put_back.rs`, `gallery.rs`,
  `status_under_a_box.rs`, `radar_bookkeeping.rs`.
- graft `MAP.md` (`update::commit_graph`, `CommitGraphOffer`, `update::place`, `update::commit_editor`,
  `update::preview::boxes` entries only).
- graft-ui: `src/name_prompt.rs`, `src/preview_box.rs`, `src/graph_screen/footer.rs`, `src/graph_screen/mod.rs`,
  `MAP.md`.
- `docs/shots/m7/`: the three new captures below + fast-tier captures this change moves (listed in NOTES).
- **Never** (held by 37 concurrently): `graft/src/view/tests.rs`, `view/preview.rs`, `view/rewrite_editor/*`,
  `update/preview/{answer,line_rows,plan_lines}.rs`, `update/preview/tests.rs`, `update/preview/slice/tests.rs`,
  `update/picker/tests.rs`, `update/stash/tests.rs`, `update/amend_into/tests.rs`, `tests/it/m7_walk/*`, graft-ops
  `preview*`, `rewrite/*`. (By 38:) graft-ops `journal_list*`, graft `tests/it/journal_screen.rs`.
  A test there that this change turns red: don't edit it. Commit at the seam, report `blocked: needs <file>` with the
  exact edit; the conductor applies it after 37/38 merge.
- Any other graft test file not listed and not in the "Never" list (e.g. `update/tests.rs` building
  `Product::TopologicalOrder`): allowed, each named in NOTES (40, its other holder, waits on 36).

## Read first
- `scouts/36.md` (all) — the lines below are its.
- `15-daily-wording.md` "Decided", "Acceptance", "Touches".
- `docs/design/02-keymap.md` §2.1:27-29, §2.5:277.
- graft `MAP.md` `CommitGraphOffer`, `AppState::commit_graph`, `update::place`; graft-ui `MAP.md` `graph_screen`
  (footer), `preview_box`, `name_prompt`.

## Scope
In:
1. **All of 15's "Decided" (exact words, unchanged), at the scout's sites:**
   - :774 rejected push: the line the user saw is `push rejected: origin/main has moved · p pull first · P force with
     lease` (`docs/reviews/m7-usability.md:84`) = `graft-ops/src/apply/remote.rs:301`. That line becomes 15's words.
     `:262` (`rejected: the remote has commits you don't`, NonFastForward) is **not** this row: unchanged; NOTES says
     what reaches it.
   - :776 `subject empty` at `update/commit_editor.rs:186` (`refusal` :175-187), drawn `view/commit_editor.rs:21-24,51`.
   - :777 rule at `mistake_radar/rules.rs:324-327`, variant `mod.rs:231`, `explain.rs:270,359`, test `rules.rs:683`,
     drawn `view/radar.rs:244`; graft-core doc line (grep `upstream` in `graft-core/src/mistake_radar.rs`).
   - :778 timing: `jobs/runner.rs:532-544` (`promote_history`, `topological_order` :538) times the sort; the
     `Duration` rides `Product::TopologicalOrder` (`update/events.rs:252`), lands `update/mod.rs:281-282` →
     `history::sorted`; the offer is decided in `update/commit_graph.rs:13-36` (called `update/mod.rs:710`) by
     `offer_after_sort`.
   - :856 `graft-ui/src/name_prompt.rs:194`.
   - :860 `update/preview/boxes.rs:40-48` (`closing_box`, `back_only` :46), `:53-75` (`ready_box`, nothing-to-do
     :62-63); graft-ui `preview_box.rs:42` FOOTER, `:44-53` back_only footer, field `:135`, default `:155`. The
     callers in `update/preview/answer.rs:330-348` don't set `back_only`: **answer.rs is not edited** (37 holds it).
   15's "Rows closed, not built" (:775, :779) stand: the conductor moves them.
2. **M3 (decided):** `B` in the detail pane (file list or diff) **always** means blame (keymap §2.5:277). The offer's
   `B`/`X` answer **only while the graph has the keyboard** (`AppState::graph_has_focus`, `state.rs:618`).
   - Key path today: `update/keys.rs:261-268` tries `pane_and_overlay_key` first, then `commit_graph_key` (:264).
     Pane `B` → `detail/keys.rs:77-84` → `place::b_key` (`place.rs:233`), guarded by `claims_b` (`place.rs:297-301`:
     `shown && focus != Graph && !commit_graph_actionable()`). `commit_graph_key` (`commit_graph.rs:40-56`) has **no
     focus check**: pane-focused `X` reaches it and declines today.
   - Change: drop `!commit_graph_actionable()` from `claims_b`; `blamable` (:305-307, `claims_b && diff_cursor`)
     follows, so the pane bar keeps `B blame`. `commit_graph_key` answers only when `offer_keys_live`; otherwise it
     claims nothing and the key is swallowed as any unbound key is. `update/keys.rs` order unchanged.
   - The offer row stays (a status, not an error). `commit_graph_line` (`state.rs:504-522`) returns
     `CommitGraphLine` (graft-ui `graph_screen/mod.rs:481`); its offered variant gains `keys: bool`, set from
     `offer_keys_live`. `footer.rs:176-192` draws the `key("B")`/`key("X")` spans and their verb words only when
     `keys`. Without: `no commit-graph file: history sorts on every open`. Keys return on `⇥` to the graph. Row count
     unchanged (readers `state.rs:481`, `picker.rs:260`, `banner.rs:310`, `update/mod.rs:143`: read, not edited).
Out: any other offer change (once-per-repo decline flag, `harness.offer_commit_graph`, `Writing`/`Written`: unchanged).
Keymap doc edit (below). The hint bar's graph-row ranking (40). `update/keys.rs` (untouched).

## For the conductor (spec change, **for the user**)
Keymap §2.1:27-29 says the offer's `B`/`X` "claim the keyboard the same way a modal pane does" — which §2.5's M7 `B`
(blame, ADR-0048) contradicts in the pane. Proposed replacement sentence, apply after the user's OK:
"`B`/`X` on the commit-graph offer are a named, narrow exception: one-shot initials, live **only while the graph has
the keyboard**; in the detail pane `B` is blame (§2.5) and the offer row is drawn without its keys." The code change
does not wait on it: it follows §2.5, the later decision.

## Signatures
```rust
// graft::update::commit_graph — 15's, unchanged:
pub(crate) fn offer_after_sort(fact: &CommitGraphFact, sort_took: Duration) -> bool;
pub(crate) const SLOW_SORT: Duration = Duration::from_millis(300);
// M3: the one predicate the key path and the offer row both read.
pub(crate) fn offer_keys_live(state: &AppState) -> bool; // commit_graph_actionable() && state.graph_has_focus()
// graft::update::place — signatures unchanged; `claims_b` loses its offer clause, `blamable` follows.
pub(crate) fn blamable(state: &AppState) -> bool;
// graft-ui::graph_screen (mod.rs): the offered variant of `CommitGraphLine` gains `keys: bool` (field name fixed;
// keep the variant's other fields as they are).
```
graft-ops: the no-upstream `Risk` variant removed (named in NOTES). graft-ui: `preview_box`'s `back_only` removed;
footer from choice count. `Product::TopologicalOrder` carries the sort's `Duration`.

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] Every acceptance box of 15 ("Acceptance"), its captures as named there. Old-word tests: find them by grepping the
      old strings (`has moved`, `p pull first`, `subject empty`, the radar words), not 15's stale lines; list each in
      NOTES. `rejected_push_names_pull_then_m_or_r` in `graft-ops/src/apply/remote/tests.rs`.
      `a_single_close_box_offers_only_esc` (four callers) in `tests/it/offer_and_blame.rs`;
      `the_stash_put_back_box_is_unchanged` in `tests/it/m6_stash_put_back.rs`.
- [ ] `b_in_the_diff_pane_blames_while_the_offer_is_up` (red first, `tests/it/offer_and_blame.rs`): offer `Offered`
      via `harness.offer_commit_graph`, `⏎` on a commit, `⇥` to the diff, `B` → a blame place opens;
      `Command::WriteCommitGraph` not emitted.
- [ ] `the_offer_keys_answer_only_with_the_graph_focused` (`update/place/tests/hints.rs` or commit_graph's tests):
      graph focused, `B` → `WriteCommitGraph`, `X` → `DeclineCommitGraph`; pane focused, neither emitted.
- [ ] `the_pane_bar_keeps_b_blame_while_the_offer_is_up` (red first, `update/place/tests/hints.rs`).
- [ ] `the_offer_row_drops_its_keys_off_the_graph`: graft half (`commit_graph_line` → `keys: false` with pane
      focus) in commit_graph's tests; graft-ui half (`footer.rs` tests: `keys: false` draws no `B`/`X`; `keys: true`
      byte-identical to today's words).
- [ ] Hint guards green: `every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here`.
- [ ] **Capture** `docs/shots/m7/diff-pane-b-with-offer-187x44.txt` (written by `tests/it/offer_and_blame.rs`) —
      **right:** the blame place open on the file and line the diff cursor was on (title names the file; the who/when
      column on the cursor line), and the offer row under the graph reading `no commit-graph file: history sorts on
      every open` with no `B`/`X`; no `commit-graph written` anywhere. Colour unverified.
- [ ] MAP.md for graft-ops, graft, graft-ui (`blamable`, `CommitGraphLine`'s `keys`, `offer_keys_live`).
- [ ] Gates.

## Touches (check each; report how)
- Everything in 15's "Touches", re-checked at the scout's lines.
- **`blamable`/`claims_b` readers:** `view/rows.rs:797` (bar), `place/history/open.rs:141`, `place::b_key`,
  `place/tests/hints.rs:32`. Grep `blamable(` and `claims_b(`; list each. Read, not edited, outside the file set.
- **The offer row:** `graph_rows` subtracting its row (count unchanged), `view::rows::modal_blanked` (graft MAP :875).
  Check: m1-54 offer tests green; grep `CommitGraphOffer::Offered` and every `CommitGraphLine` match; list.
- **Graph-focused `B`/`X`:** unchanged; m1-54 tests green.
- **History place's `B`** (§2.17:476) and the resolver's `B` (§2.6:293): claimed before the pane; their tests green.
- **Captures:** fast-tier captures the change moves (offer gone after a fast sort; keys gone with pane focus) are
  re-taken by their tests and listed. A capture also moved by 37: the conductor re-runs its test after both merge;
  never hand-merged.
- **Keymap doc:** the §2.1 sentence above — conductor, not builder. **Journal/undo:** none.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~65k (15's ~50k + M3 ~15k). One context. Scout: no split needed.
