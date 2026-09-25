# tab-words-name-where-tab-goes: every `⇥` hint and refusal names where `⇥` really lands

Status: draft
From: `docs/reviews/m6-usability-final.md` N6 (:178-191). Folds in **n2** (:223-227, `c continue` vs `c finish` on a
merge's banner; same hint file) and **n1** (:212-221, an untracked file reads `bin`; the changes pane) — n1's body half
only after its find.
Build with: (conductor, after the scout)
Scout: `.scratch/m6-conflicts/scouts/tab-words-name-where-tab-goes.md` — brief below.
Needs: none. Runs after group B if both are queued (both regenerate bar captures; `every_context` counts move here).
Crates: graft-ui (`graph_screen/hints/lists.rs`, `hints/catalogue.rs`, `HintContext`, `detail_pane/files.rs`,
`graph_screen/tests.rs`), graft (`update/banner.rs` focus cycle, `update/detail/ops.rs`, `update/commit_editor.rs`
refusal words, `view/rows.rs`, `view/detail.rs`, tests, walks).
Group: **C** (the changes pane and the banner's words).

## Read first
- `docs/reviews/m6-usability-final.md` N6 (:178-191), n1 (:212-221), n2 (:223-227); the task row "Rebase stop, from the
  changes pane to abort" (:74).
- `crates/graft/MAP.md` :244 (`OperationRefusal`), :303 (`take_focus`: the cycle Graph → Files → Diff → banner),
  :317 (`one_tab_to_banner`), :671 (`footer_context`, `live_wrapped`), :674 (`hint_context`).
- `crates/graft-ui/MAP.md` :137 (`HintContext::ChangesPane { unmerged, live }`), the `Banner { skippable, conflicted }`
  entry.
- `docs/design-notes/m6-usability-fixes.md` "Decided here" item 2 (the pane's refusal words, one value two forms).
- `docs/design/02-keymap.md` §2.1 ("the hint bar always shows what's active"), §2.8 banner rows (:301-302).

## The cause (rule 7, lines at `1b1995cc`; the scout re-reads them on main)
- The pane's bar always ends `⇥ graph` (`graft-ui/src/graph_screen/hints/lists.rs:261`, `:270`): a literal, not read
  from the cycle.
- The cycle is Graph → Files → Diff → banner → Graph while an operation is live (`update/detail/ops.rs:403-408`;
  `update/banner.rs:256-275`). From Files, `⇥` goes to Diff.
- `a` in the pane refuses with `rebase in progress · ⇥ banner, then c continue  a abort`
  (`update/commit_editor.rs:161-163`): one `⇥` named, two needed from Files.
- n2: the banner's bar says `c continue` for a merge (`lists.rs:197`); the graph and pane say `c finish` (`:239`); both
  open the finish summary.
- n1: `bin` is drawn whenever the line counts are absent (`graft-ui/src/detail_pane/files.rs:58-61`); an untracked file
  has none. The body reads `select a file` when the diff is `Idle` (`graft/src/view/detail.rs:236`), and `show_cursor`
  leaves it `Idle` when the row has no diff entry (`update/detail/ops.rs:276-285`). Why an untracked row has none: not
  traced.

## Decided here
- **The words follow the cycle; the cycle does not change.** Every `⇥` the pane draws names its target (`⇥ diff`,
  `⇥ banner`, `⇥ graph`) from one function that reads the cycle. The refusal names the real count
  (`⇥ ⇥ banner, then c continue  a abort` from Files; `⇥ banner, …` from Diff). Rejected: making Files' `⇥` jump to the
  banner during a stop (3 keys, not 4) — `⇥` would mean two things in one pane by state (H4), and `c` in the pane
  already continues (P14), so only `a` takes the longer road.
- **Journey** "rebase stop, from the changes pane, abort": `⇥ ⇥ a y` **4**, unchanged; every word on the way now true.
- **n2:** a merge's banner says `c finish`, as the graph and pane do. Word only; `c` does the same thing.
- **n1:** an untracked file's count column reads `new`, not `bin`; a real binary still reads `bin`. The cause is that the
  view infers binary from absent counts; the row must carry why its counts are absent. The body: see find 3.

## Scout brief (≤1 page, code not intent)
1. `lists.rs:190-280` on main: which `HintContext` arms :197, :239, :261, :270 belong to; every other literal `⇥ …` in the
   hint tables.
2. The real cycle, both layouts: `detail/ops.rs:395-415`, `banner.rs:250-280` (`take_focus`), and the sub-100-column
   Tab layout. For each focus (Graph, Files, Diff, banner) × {operation live, not} × {pane open, hidden} × {≥100, <100
   columns}: where one `⇥` lands. A table.
3. n1's body: why an untracked row has no diff entry. The detail job's worktree diff read (`jobs/detail.rs`): does it
   read untracked files at all, by design (M3 deferred hunk staging in untracked files, `m3-…md:131`) or by accident?
   What `files.rs` gets for an untracked row (a flag, a status letter).
4. `OperationRefusal` construction for the pane's `a` (`commit_editor.rs:150-170`): every arm, and whether the keys
   string is built in one place.
5. Captures with `⇥ graph` or `c continue` on a merge banner: `grep -rln "⇥ graph\|c continue" docs/shots`.

## Scope
**In:** one cycle-reading function; the pane's `⇥` word; the refusal's count; the merge banner's `c finish`; n1's `new`
word; n1's body if find 3 shows the read exists and the row only drops it (a graft-only fix).
**Out:** the cycle itself; `a` aborting from the pane (it refuses, by P14); n1's body if find 3 shows no read of
untracked content exists — then it is a new read, not a fix: NOTES says so with the line, and the conductor files it.

## Signatures
```rust
// graft, update/banner.rs (beside `one_tab_to_banner`, which becomes `tabs_to_banner(state) == Some(1)`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabTarget { Graph, Files, Diff, Banner }
/// Where one `Tab` goes from the focus the keyboard has now: the one reading of the cycle
/// (`detail::ops` and `banner::take_focus`), both layouts. The bar's `⇥` word and the refusal's count read it.
#[must_use] pub(crate) fn tab_target(state: &AppState) -> TabTarget;
/// How many `Tab`s from here reach the banner; `None` when no banner is drawn.
#[must_use] pub(crate) fn tabs_to_banner(state: &AppState) -> Option<u8>;

// graft-ui, hints: the pane contexts carry the word instead of the literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabWord { Graph, Diff, Banner }   // `word()` -> "graph" | "diff" | "banner"
// HintContext::ChangesPane { unmerged, live, tab: TabWord }   // + `tab` on the :270 context the scout names

// graft-ui, detail_pane/files.rs — the file row says why it has no counts.
pub enum Counts { Lines { added: u32, removed: u32 }, Binary, Untracked }   // replaces the Option the scout finds
```

## The failing tests first
- graft-ui `graph_screen` tests: `the_files_list_bar_names_where_tab_goes` — `ChangesPane { tab: Diff, .. }` ends
  `⇥ diff`. Red today (the literal).
- graft `update` tests: `the_amend_refusal_counts_the_tabs_it_names` — a rebase stop, focus on Files, `a`: the keys read
  `⇥ ⇥ banner, then c continue  a abort`; pressing `⇥` that many times puts the keyboard on the banner. Red today.
- graft-ui: `an_untracked_file_reads_new_not_bin`. Red today.

## Touches (each entry, and how the builder checks it)
- **`one_tab_to_banner`** and the graph's `⇥ banner` hint (`OperationLive { tab_to_banner }`): re-expressed through
  `tabs_to_banner`; same output. Check: its existing tests green, byte-identical captures.
- **`tab_target` against the real cycle:** `tab_target_is_where_tab_lands` — every row of scout table 2: press `⇥`,
  compare the focus with the prediction. 80 and 120 columns.
- **`every_context` / coverage slots / `every_key_the_bar_draws_is_answered` / `bound_actions`:** `ChangesPane` gains
  `tab`; `("⇥", "diff")`, `("⇥", "banner")` and `("c", "finish")` on the banner join `bound_actions`. Check: all green;
  NOTES gives the slot counts old → new.
- **The banner's `c`** (n2): `every_hint_the_focused_banner_draws_is_answered_by_banner_key` green with `c finish`.
- **The finish-summary journey rows** that name the banner's `c` word in `m6-conflicts.md` or TESTING.md:
  `grep -n "c continue" docs/design-notes/m6-conflicts.md TESTING.md`; listed in NOTES.
- **`OperationRefusal`**: both forms (editor row, pane sticky) read the count. Check:
  `the_amend_refusal_counts_the_tabs_it_names` and the editor-row test for the same arm.
- **n1:** `bin` on a real binary unchanged (`a_binary_file_still_reads_bin`); the staged-file `+1 −0` unchanged.
- **Captures** from scout Q5: each regenerated by its test, or `git diff --exit-code`.
- **Keymap:** §2.8 `c` row already says a merge opens the summary; no meaning changes. If a hint word is quoted there,
  the builder lists it; the conductor edits.
- **Journal/undo:** none.
- **MAP.md:** graft `tab_target`, `tabs_to_banner`; graft-ui `TabWord`, `ChangesPane { tab }`, `Counts`.

## Acceptance (name the verifying test at tick time)
- [ ] The three failing tests above, red then green.
- [ ] `tab_target_is_where_tab_lands` (scout table 2, every row).
- [ ] `the_diff_bar_names_the_banner_during_a_stop` (`⇥ banner`), and `⇥ graph` with nothing live.
- [ ] `a_merge_banner_says_c_finish`; a rebase banner still says `c continue`.
- [ ] `a_binary_file_still_reads_bin`.
- [ ] n1 body: either `an_untracked_file_shows_its_lines` (find 3: the read exists) or NOTES names the line proving no
      read exists and the row stays `select a file` (then **not** ticked as fixed; the conductor files a gap).
- [ ] Walk, real repo (`tests/it`): `abort_a_rebase_from_the_changes_pane_is_four_keys` — `⇥ ⇥ a y` from Files, count
      asserted ≤ 4, `git status` shows no rebase.
- [ ] Captures (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-tab-words-120x40.txt` and `-80x24`.
  - Frame 1: a rebase stop, focus on the changes pane's files: the bar ends `⇥ diff`.
  - Frame 2: `a`: the bottom row reads `rebase in progress · ⇥ ⇥ banner, then c continue  a abort`.
  - Frame 3: after one `⇥` (Diff): the bar ends `⇥ banner`. Frame 4: after the second, the banner is focused.
  - Frame 5: a merge stop, the banner focused: its bar has `c finish`, not `c continue`.
  - Frame 6: an untracked `new.txt` in the changes pane reads `new`, not `bin`.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large: two crates, a graft-ui enum change every hint test enumerates, a cycle table, six frames. One context if
n1's body stays out; if find 3 shows a graft-only fix that grows, hand off at the seam (graft-ui committed first).
