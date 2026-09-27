# 20l diff-space-stays-in-the-pane — `space` with the diff focused no longer folds the graph's merge, at any width

Status: draft
Blocked by: none by content. Crates graft + graft-ui: the conductor sequences it against whoever holds them.
Crates: graft (the detail pane's key handler, `update` tests, the pinned captures, MAP.md) + graft-ui (the `hints` filter, its tests, MAP.md) + `docs/design/02-keymap.md` §2.2 `Space` (verbatim, below).
Build with: **Sonnet**. One key claimed by the pane, one filter made uniform, decided below. The 20h sweeps judge both.
Needs: none.
Scout: none. Review `docs/reviews/m7-06e-20h.md` (Deviations, 20h; Question 1) and 20h's NOTES name the lines. Re-read each one before editing.
Source: KNOWN_GAPS M7/20h-diff-space (question). Ruling: planner, 2026-09-27.

## Facts relied on (lines as review m7-06e-20h and 20h's NOTES cite them; the reviewer checked them)
- The diff's handler has no arm for `' '`, so the key falls through:
  - `diff_key` has no `' '` arm (`update/detail/keys.rs:158-160`, `:313`), and neither does `diff_option_key`.
  - `detail_key` then returns `false`.
  - `update/keys.rs:263` → `graph_key` → `view_change` (`keys/view_change.rs:71`) → `Fold` on the merge under the graph's cursor.
- The bar agrees. The `PaneFocused` filter drops the graph's `space fold` only when `opens` (files list focused). With the diff focused, the hint stays (`hints.rs` `pane_focused_bar`, `:406-439`).
- Below 100 columns on the Detail tab, the graph screen is not drawn at all (`update/detail/control.rs:73-77`). So the fold changes a screen he can't see, and he finds a merge folded when he comes back.
- The planner's own 20h ruling said "with a pane focused the graph's `space` is not live" (20h Decision 1). The fall-through made that false only for the diff. The keymap sentence that says otherwise describes the code, not a decision (20h NOTES).

## Ruling: `space` belongs to the focused pane, at every width
While the detail pane has the keyboard, in its file list or its diff, `space` never reaches the graph, and the bar never offers the graph's `space fold`.
- With the diff focused, `space` does nothing and is not offered.
- In the file list, the behaviour is unchanged: a directory folds, a file does nothing.

**Why:**
- H6, "keys valid in the focused pane" (`docs/research/ux-principles.md:56`, checklist `:125`). A key that acts on an undrawn screen breaks visibility of system status.
- Consistency: if the rule depended on width, a key would change what it does when he resizes the window. The pane's other keys don't reach the graph, and `j`/`k` scroll the diff, not the graph.
- The fold is still one `Esc`/`Tab` away, on the screen where it shows.

**Rejected:**
- Keeping it only at ≥ 100 columns: the key would mean one thing or another depending on width.
- Giving `space` a pager meaning (page down) in the diff: that is a new key meaning with no journey or field note behind it. It would need a spec change and is not proposed.

**Where:** at the cause. The detail pane claims `' '` whenever it has the keyboard. `files_key` already does, on every row (`keys.rs:300`). `diff_key` and `diff_option_key` gain the same claim with no effect. Don't special-case `space` at `update/keys.rs:263`: the pane owns its keys. If the lines show a cleaner single place inside `detail_key`, use it and give the line in NOTES.

## Scope
In:
- graft `update/detail/keys.rs`: the claim.
- graft-ui `hints`: the `PaneFocused` filter drops the graph's `space fold` in **every** form, not only when `opens`. The `folds` field and its `space fold` for a directory are unchanged.
- Replace the tests that pinned the old behaviour. This is a spec change, not a disabled test: say so at tick time, and leave a one-line comment where each old test stood.
- The keymap row.
- Regenerate the pinned captures that change.

Out:
- The files list.
- The changes pane's staging `space` (§2.4) and the conflict screen's `space` (§2.4, `02-keymap.md:331`): these are their own handlers, and must not change.
- The blame and history places' `space` (`:416`, `:429`).
- `PaneOverOverlay`.
- Rebuilding the contact sheets (M7/20h-sheets, ticket 25 or 26).

## Signatures
None change. `HintContext::PaneFocused { row, opens, blamable, folds }` and `hints_for(context, divider, columns) -> Vec<Hint>` keep their shapes (20h).

## Tests (red first; name each at tick time)
1. graft `update::tests::pane_space::space_with_the_diff_focused_leaves_the_graph_as_it_was`.
   - Set up: the cursor on a merge, the detail pane open, the diff focused, then `space`.
   - Check: the graph's fold state is unchanged, and the bar holds no `space`.
   - Run it at 120×40 and at 80×24.
   - It **replaces** `space_with_the_diff_focused_folds_the_graphs_merge_and_the_bar_says_so`.
   - Red today.
2. graft-ui `graph_screen::hints::tests::pane_space::pane_focused_on_the_diff_drops_the_graphs_space_fold`. It **replaces** `pane_focused_on_the_diff_keeps_the_graphs_space_fold`. Red today.
3. These stay green, named at tick time:
   - `pane_bar_on_a_directory_row_offers_space_fold`
   - `space_on_a_directory_row_folds_it_and_the_bar_still_says_fold`
   - `pane_bar_on_a_commits_file_row_draws_no_space`
   - `pane_focused_keeps_the_changes_panes_space`
   - `every_key_the_bar_draws_is_answered`
   - `every_context_reaches_every_shape`
   - `every_swept_state_is_the_context_it_claims`
   - `pane_bar_space::capture_the_pane_bar_on_a_file_and_on_a_directory`
4. One changes-pane test that stages with `space` while its diff is focused stays green. Name it; the grep is `space` under the changes pane's tests. This shows the pane's own `space` wasn't swallowed.

## Keymap (apply verbatim)
In `docs/design/02-keymap.md` §2.2, replace the whole `Space` row (`:186`) with:

> | `Space` | Toggle stage where staging applies, else expand/collapse. In a commit's file list: a directory folds / unfolds; a file has nothing to toggle, so `space` is unbound there and not offered. While a pane has the keyboard (its file list or its diff), the graph's merge fold (§2.3) is not live and not offered: `space` never acts on a screen other than the focused one (Amended 2026-09-27, M7/spacefold; M7/20h-diff-space) |

§2.3 `:221` stays unchanged.

## Acceptance (name the verifying test at tick time)
- [ ] Tests 1–2 seen red before the change and green after. Tests 3–4 are green and named.
- [ ] Captures: grep `space fold` in `docs/shots/` for every **diff-focused** pane bar. 20h's NOTES name `m5-hints-detail-*` and `m7/diff-cursor-commit-120x40.txt`; re-check the list, don't trust it. Regenerate each pinned one through its own test and name the test. Each diff may lose only `space fold` and its re-spent width. List every contact sheet that quotes a changed capture, for M7/20h-sheets.
- [ ] Keymap row replaced verbatim. `git diff` shows one line.
- [ ] MAP.md is updated. graft: the detail pane claims `space` in both lists. graft-ui: `PaneFocused` drops the graph's `space fold` in every form.
- [ ] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --doc --workspace`.

## Touches (check each; NOTES says how)
- **Every caller of `detail_key`, and its `false` path:** check that no other key changes routing. Diff the `match` arms before and after.
- **Other screens with a focused diff** (changes pane, conflict screen, blame and history places): they have their own handlers. Grep for where each routes `' '`, and show one test per screen stays green (4 covers the changes pane; name the others or say none exists).
- **Captures and contact sheets:** see Acceptance. Past milestones' sheets are a record; don't edit them.
- **The mouse:** none (the wheel only, `update/mouse.rs:146-155`).
- **Journal/undo:** none (a fold is view state).
- **KNOWN_GAPS M7/20h-diff-space:** the conductor closes it from the report, and adds this ticket's changed captures to M7/20h-sheets.

## Size
Small (~25k).

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
