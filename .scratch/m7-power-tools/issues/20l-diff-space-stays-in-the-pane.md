# 20l diff-space-stays-in-the-pane — `space` with the diff focused no longer folds the graph's merge, at any width

Status: done
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
- [x] Tests 1–2 red before, green after (spec change: they replace the old pins, one-line comment at each old site). `update::tests::pane_space::space_with_the_diff_focused_leaves_the_graph_as_it_was` red: `120x40 left: ["fold"] right: []`. `graph_screen::hints::tests::pane_space::pane_focused_on_the_diff_drops_the_graphs_space_fold` red at `PaneFocused { row: BranchRow { current: false }, opens: false, blamable: true, folds: false }`. Tests 3 green in the full run (4184/4184): `pane_bar_on_a_directory_row_offers_space_fold`, `space_on_a_directory_row_folds_it_and_the_bar_still_says_fold`, `pane_bar_on_a_commits_file_row_draws_no_space`, `pane_focused_keeps_the_changes_panes_space`, `every_key_the_bar_draws_is_answered`, `every_context_reaches_every_shape`, `every_swept_state_is_the_context_it_claims`, `pane_bar_space::capture_the_pane_bar_on_a_file_and_on_a_directory`. Test 4: `it::changes_pane_writes::space_on_a_hunk_stages_only_that_hunk` (diff focused, `space` stages) green.
- [x] Captures regenerated (`GRAFT_UPDATE_SHOTS=1`); each diff only loses `space` and re-spends width: `m5-hints-detail-{80x24,120x40,160x48}.txt` (`hint_bar::capture_hint_bar_for_docs_shots`), `m7/diff-cursor-commit-120x40.txt` (`diff_cursor::capture_the_commit_diff_cursor`), `m7/blame-then-commit-120x40.txt` (`blame_place::capture_blame_then_commit_120x40`; not on 20h's list). Also the quoted bar in `hint_bar::the_detail_tab_hides_the_keys_a_focused_pane_does_not_answer` (`space  1` → `1  F  H`). Remaining `space fold` in docs/shots: `m7/pane-bar-file-vs-directory-120x40.txt` (files list on a directory: correct), m6 records. Sheets quoting changed captures: `docs/shots/m5-contact-sheet-1.txt`, `docs/shots/m5-contact-sheet-captions.tsv` (past record, not edited); no M7 sheet yet.
- [x] Keymap row replaced verbatim; `git diff docs/design/02-keymap.md` shows one line.
- [x] MAP.md updated: graft `keys.rs` entry; graft-ui `PaneFocused` folds entry.
- [x] Gates workspace-wide: fmt, clippy -D warnings, nextest 4184 passed / 3 skipped, test --doc pass.

## NOTES (20l)
- Claim placed once, in `diff_option_key` (`update/detail/keys.rs`, first match arm): reached from `diff_key` and `files_key` fallbacks; `files_key`'s own `' '` arm still fires first. No other arm changed.
- Changes pane: write keys (staging `space`) run before `diff_key`; test 4 green. Conflict screen: `operate::space_on_a_conflicted_file_stages_it_as_the_resolution` green. Blame place `space` (`place/keys.rs:56`): own handler, no key-level test exists. History place: none found.
- Mouse, journal: untouched. No scout: ticket named the lines; each re-read.

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
