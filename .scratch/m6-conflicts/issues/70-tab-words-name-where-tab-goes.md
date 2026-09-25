# tab-words-name-where-tab-goes: every `⇥` hint and refusal names where `⇥` really lands

Status: done except the n1 body pin (stopped by its rule; see NOTES)
From: `docs/reviews/m6-usability-final.md` N6 (:178-191). Folds in **n2** (:223-227: `c continue` vs `c finish` on a
merge; same hint file) and **n1** (:212-221: an untracked file reads `bin` and its body `select a file`; both halves in
scope, because the scout found the read exists).
Build with: **Sonnet**. The work is mechanical and bounded: word changes, one context flag, and a cycle table the scout
already drew. One cause is still untraced (n1's dropped entry), but it has a stop rule below, so it can't turn
structural unnoticed.
Scout: `.scratch/m6-conflicts/scouts/tab-words-name-where-tab-goes.md` (at `36efb090`).
Needs: none by signature. **Order:** builds after 67, 71 and group B (68+69). Rebase on all three; shared files are listed
under Touches.
Crates: graft-ui (`graph_screen/hints/lists.rs`, `HintContext`, `detail_pane/files.rs`, `graph_screen/tests.rs`), graft
(`update/banner/stop.rs`, `update/commit_editor.rs`, `update/detail/ops.rs`, `view/rows.rs`, `view/detail.rs`, tests,
`tests/it`). Hand off at the seam if the context runs short (graft-ui committed first).
Group: **C** (the changes pane and the banner's words).

## Read first
- `docs/reviews/m6-usability-final.md` N6 (:178-191), n1 (:212-221), n2 (:223-227); task row :74.
- This ticket's scout (whole).
- `crates/graft/MAP.md` :244 (`OperationRefusal`), :303 (`take_focus`), :318 (`one_tab_to_banner`), :671
  (`footer_context`), :674 (`hint_context`).
- `crates/graft-ui/MAP.md` :137 (`HintContext::ChangesPane { unmerged, live }`), the `Banner { skippable, conflicted }`
  entry.
- `docs/design-notes/m6-usability-fixes.md` "Decided here" item 2.
- `docs/design/02-keymap.md` §2.1, §2.8 banner rows (:301-302).
- Spec 69 "Decided 1" (its blanking in `view/rows.rs`, which this ticket edits next to).

## The cause (rule 7, lines at `36efb090`, from the scout)
- **The pane's `⇥` word** is a literal `⇥ graph`, used by both arms of the one `ChangesPane` table (unmerged `:261`,
  plain `:270`, `changes_pane_table` `lists.rs:252-274`).
- **The cycle** (`detail/ops.rs:403-408`, `banner.rs:256-275`, leaving the banner `banner.rs:~300`) has no layout branch.
  It is the same at ≥100 and <100 columns. Scout table §2.
- **The refusal has two literal arms** (`commit_editor.rs`): (Merge, InShell) `"⇥ banner, then c finishes it"`
  `:148-150`, and (_, InGraft) `"⇥ banner, then c continue  a abort"` `:163`. Neither counts the tabs, and the
  InGraft arm says `c continue` on a merge.
- **n2:** `banner_list(skippable, conflicted)` has no operation kind, so `c continue` (`lists.rs:197`) can't become
  `c finish`.
- **n1:** `files.rs:58-61` draws `bin` for every row with absent counts. An untracked row carries
  `FileStatusView::Untracked` (`jobs/detail.rs:413`), and its content is read (`jobs/detail.rs:288,315-317,455`;
  graft-git `diff/read/tests.rs:~603-619`). The body still reads `select a file` because `show_cursor`
  (`ops.rs:276-285`) finds no diff entry for the row. **Why it finds none: not traced** (pinned below).

## Decided here
1. **The pane's words follow the cycle, and the cycle stays as it is.** Every `⇥` in the pane names its target from a
   single reading of the cycle. The refusal names the real count. The rejected alternative (Files' `⇥` jumping to the
   banner) is unchanged from the first draft: H4, P14.
2. **The reading of the cycle comes from the same calls the cycle makes.** `tab_target` asks whether a banner is drawn
   through the call `take_focus` uses. It never works it out again from "an operation is live", because the scout
   could not confirm that the two always agree. `one_tab_to_banner` keeps its `!modal_owns_keys()` check (`stop.rs:347-352`).
   That check is separate from 69's `modal_owns_bottom_row`, and neither replaces the other.
3. **The banner's own `⇥ graph` (`:209`) stays a literal.** It is true in every state, because leaving the banner always
   sets Graph. It is pinned by the banner row of `tab_target_is_where_tab_lands`, not rewired.
4. **n2 is a `merge` flag on `Banner`,** set by `hint_context` from the live operation's kind (a merge begun in Graft or
   in the shell; `c` opens the finish summary on both). On a merge the banner says `c finish`. The pane's refusal
   InGraft arm on a merge also says `c finish` (`⇥ … banner, then c finish  a abort`). The InShell arm keeps
   `c finishes it`.
5. **Both refusal arms take their count from one helper,** `tab_keys`. The word forms stay as they are, and the count
   is the only change.
6. **n1, word:** match the row's own status. `Untracked` with absent counts reads `new`. Other rows with absent counts
   read `bin`. There is **no `Counts` enum**. The first draft had one, but the row already carries its status, and the
   enum would have touched every counts reader for nothing.
7. **n1, body:** in scope. The fix goes test-first. `an_untracked_file_shows_its_lines` starts red. Before fixing, the
   builder traces `selected_entry` (`ops.rs:48-53`) → `show_cursor` (`:276-285`) and names in NOTES the line that
   drops the entry (rule 7). **Stop** if the cause is outside graft, or if the fix needs a new read rather than a
   dropped one. Hunk staging in untracked files stays out (M3 deferral).
8. **Journey** "rebase stop, from the changes pane, abort": `⇥ ⇥ a y` **4**, unchanged, and every word on the way is
   now true.

## Scope
**In:** `tab_target`, `tabs_to_banner`, `tab_keys`; the pane's `⇥` word; both refusal arms; the `merge` flag and
`c finish`; `new`; the untracked body.
**Out:** the cycle; `a` aborting from the pane (P14); the banner's `⇥ graph`; the `NamePrompt` `⇥ toggle` (`:77`);
hunk staging in untracked files.

## Signatures
```rust
// graft, update/banner/stop.rs (beside `one_tab_to_banner`; not banner.rs, which 68 and 69 edit)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabTarget { Graph, Files, Diff, Banner }
/// Where one `Tab` goes from the current focus: the one reading of the cycle (`DetailState::cycle_focus`,
/// `banner::take_focus`, leaving the banner). `None`: `Tab` moves no focus here.
#[must_use] pub(crate) fn tab_target(state: &AppState) -> Option<TabTarget>;
/// How many `Tab`s from here reach the banner; `None` when no banner is drawn (the call `take_focus` makes).
#[must_use] pub(crate) fn tabs_to_banner(state: &AppState) -> Option<u8>;
/// `"⇥ banner"`, `"⇥ ⇥ banner"`: the keys both refusal arms name.
#[must_use] pub(crate) fn tab_keys(tabs: u8) -> String;
// `one_tab_to_banner(state)` becomes `!modal_owns_keys() && tabs_to_banner(state) == Some(1)`; same output.

// graft-ui, graph_screen/hints
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabWord { Graph, Diff, Banner }            // `word(self) -> &'static str`: "graph" | "diff" | "banner"
// HintContext::ChangesPane { unmerged: bool, live: LiveC, tab: TabWord }   (both arms of `changes_pane_table` read it)
// HintContext::Banner { skippable: bool, conflicted: bool, merge: bool }   (`merge` → `c finish`)
```

## The failing tests first
- graft-ui: `the_files_list_bar_names_where_tab_goes`: `ChangesPane { tab: Diff, .. }` ends `⇥ diff`, both arms
  (`unmerged` true and false). Red today.
- graft-ui: `a_merge_banner_says_c_finish`: `Banner { merge: true, .. }` has `c finish`. Red today (`c continue`).
- graft `update`: `the_amend_refusal_counts_the_tabs_it_names`: a rebase stop, focus Files, `a`: the keys read
  `⇥ ⇥ banner, then c continue  a abort`, and pressing `⇥` twice puts the keyboard on the banner. Red today.
- graft-ui: `an_untracked_file_reads_new_not_bin`. Red today.
- graft: `an_untracked_file_shows_its_lines`: a cursor on an untracked `new.txt` shows its content, not `select a file`.
  Red today.

## Touches (each entry, and how the builder checks it)
- **Files shared with earlier tickets (rebase, then re-read the lines):**
  - `graft/src/view/rows.rs`: **69** edits `graph_view` and `box_context` there. This ticket edits `hint_context`, the
    `merge` flag and `ChangesPane { tab }`. Check: 69's `a_modal_owns_the_bottom_row` stays green.
  - `crates/graft/MAP.md`: 67, 68, 69 and 71. `crates/graft-ui/MAP.md`: 71. These are separate entries, so it's a merge
    and not a rewrite.
  - `TESTING.md` and the m6 contact sheet: all four earlier tickets. The builder lists rows and captures; the conductor
    edits and rebuilds.
  - **No shared source file** with 67 (graft-ops `preview.rs`, `update/preview.rs`, `update/operate.rs`), 71
    (`update/context_menu*`, graft-ui `context_menu.rs`) or 68 (`update/banner.rs`, `update/state.rs`). Check:
    `git diff --stat <base>..` lists none of them. `commit_editor.rs` is not 68's: 68's words live in `banner.rs`.
- **`one_tab_to_banner` / `OperationLive { tab_to_banner }`:** re-expressed with the same output. Check: its tests are
  green and the captures are byte-identical.
- **`tab_target` against the real cycle:** `tab_target_is_where_tab_lands`. For every cell of scout table §2, press
  `⇥` and compare the focus with `tab_target`. Run at 80 and 120 columns. The untraced cell (not live, pane hidden,
  Graph) is asserted as whatever the key does, and NOTES records it.
- **`every_context` / coverage slots / `every_key_the_bar_draws_is_answered` / `bound_actions` /
  `every_hint_the_focused_banner_draws_is_answered_by_banner_key`:** `ChangesPane` gains `tab`, and `Banner` gains
  `merge`. `("⇥","diff")`, `("⇥","banner")` and `("c","finish")` on the banner join `bound_actions`. All must be
  green, and NOTES gives the slot counts old → new.
- **Both refusal arms** (editor row and pane sticky forms): `the_amend_refusal_counts_the_tabs_it_names`,
  `a_shell_merge_refusal_counts_its_tabs` (the InShell arm, from Files: `⇥ ⇥ banner, then c finishes it`), and
  `a_graft_merge_refusal_says_c_finish`.
- **n1:** `a_binary_file_still_reads_bin`; the staged file's `+1 −0` is unchanged.
- **Captures:** the scout found 31 files matching `⇥ graph|c continue`. The builder runs the suite, and then
  `git diff --stat docs/shots` lists every file a test rewrote. Each rewrite goes in NOTES with the reason. A file no
  test writes (historical: m3/m5 dated, `prototype-journeys.txt`, the captions tsv) stays as the record of its
  milestone, and NOTES lists it as "not regenerated". This includes 69's regenerated `fix-box-takes-y-*` and
  `m3-09-undo-drift-box-*` if they move again.
- **Design text:** `grep -n "c continue" docs/design-notes/m6-conflicts.md TESTING.md docs/design/02-keymap.md`. Hits
  about a merge are listed in NOTES, and the conductor edits them.
- **Keymap:** no meaning changes. **Journal/undo:** none.
- **MAP.md:** graft `tab_target`, `tabs_to_banner`, `tab_keys`, `one_tab_to_banner`'s new body, and the n1 cause line.
  graft-ui `TabWord`, `ChangesPane { tab }`, `Banner { merge }`, and `files.rs`'s `new`.

## Acceptance (name the verifying test at tick time)
- [x] Four of five red then green: `the_files_list_bar_names_where_tab_goes`, `a_merge_banner_says_c_finish`, `the_amend_refusal_counts_the_tabs_it_names`, `an_untracked_file_reads_new_not_bin`. The fifth (`an_untracked_file_shows_its_lines`) is under the pin, not ticked.
- [x] `tab_target_is_where_tab_lands` (every cell, 80 and 120 columns).
- [x] `the_diff_bar_names_the_banner_during_a_stop` (`⇥ banner`); `⇥ graph` with nothing live.
- [x] A rebase banner still says `c continue` (a row in `a_merge_banner_says_c_finish`).
- [x] `a_shell_merge_refusal_counts_its_tabs`; `a_graft_merge_refusal_says_c_finish`.
- [x] `a_binary_file_still_reads_bin`.
- [ ] **NOT TICKED (stop rule).** **Pin (the scout could not confirm):** NOTES names the line where `show_cursor` loses the untracked row's entry,
      and `an_untracked_file_shows_its_lines` proves the fix. If the cause is outside graft or needs a new read, **not**
      ticked: stop and report with the line.
- [x] Walk, real repo (`tests/it`): `abort_a_rebase_from_the_changes_pane_is_four_keys`: `⇥ ⇥ a y` from Files. The
      count is asserted ≤ 4, and `git status` shows no rebase.
- [x] (`capture_fix_tab_words_120x40`, `_80x24`; frame 3 differs, frame 6 body not fixed, see NOTES) Captures (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-tab-words-120x40.txt` and `-80x24`.
  - Frame 1: a rebase stop, focus on the changes pane's files. The bar ends `⇥ diff`.
  - Frame 2: `a`. The bottom row reads `rebase in progress · ⇥ ⇥ banner, then c continue  a abort`.
  - Frame 3: after one `⇥` (Diff), the bar ends `⇥ banner`. Frame 4: after the second, the banner is focused.
  - Frame 5: a merge stop, the banner focused. Its bar has `c finish`, not `c continue`.
  - Frame 6: an untracked `new.txt` in the changes pane. The row reads `new`, not `bin`, and the body shows its lines,
    not `select a file`.
- [x] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large: two crates, two context fields that every hint test enumerates, a cycle table, one traced cause, six
frames. One context. If the n1 body trace grows, hand off at the seam: the graft-ui half is committed first.

## NOTES (builder, 2026-09-25)
- Scout used: `.scratch/m6-conflicts/scouts/tab-words-name-where-tab-goes.md`. **I compacted once mid-ticket.** Remaining: only the n1 body.
- **n1 body cause:** the entry is not dropped (`crates/graft/src/jobs/detail.rs:272` gives untracked rows entries). graft-git `crates/graft-git/src/diff/read.rs:330-332` returns `empty_diff` for `FileStatus::Untracked` without spawning git (M3 deferral), so the diff arrives `Ready` with `hunks: []`. The fix needs a new read in graft-git, outside this ticket's crates. **Stopped**; red test removed, not ignored. Needs a ticket in graft-git.
- **Question:** the pane refusal is `Sticky`; after one `⇥` its `⇥ ⇥ banner` still stands (frame 3 shows the refusal, not the diff bar's `⇥ banner`). Only `Esc` clears it. Should a focus move clear it?
- Coverage slots 63 → 73. `every_context` now yields only real forms (banner `skippable: !merge`; pane `tab` per liveness).
- `bound_actions` gained `("⇥","diff")`.
- Captures rewritten by tests: m5 `m5-hints-changes-{80x24,120x40,160x48}`, `m5-ui-banner-conflict-{80x24,120x40,160x48}`; m6 `fix-a-mid-merge-answers-in-words-{187x50,80x24}`, `fix-c-in-pane-finishes-187x50`, `fix-editor-names-the-banner-{120x40,80x24}`, `fix-landing-b-launch-{120x40,187x50}`; new `fix-tab-words-{120x40,80x24}`. Reason: `⇥ diff`, `⇥ ⇥`, `c finish`. m5 and m6 contact sheets rebuilt (parts ≤180 KB).
- Not regenerated (no test writes them): `m6-18-launch-shell-begun-*` (5, **now stale**: `c continue` on a merge), `m6-06-relaunch-sticky-*`, `m6-07-all-resolved-*`, `m3-changes-pane-*`, `m5-hints-ladder.txt`, `prototype-journeys.txt`, `fix-c-live-frames`.
- Design text on a merge saying `c continue`, for the conductor: `docs/design-notes/m6-conflicts.md:258,373,375`; `TESTING.md:2415`.
- `one_tab_to_banner` now reads `banner_view` via `tabs_to_banner`; suite green, capture bytes not diffed one by one.
