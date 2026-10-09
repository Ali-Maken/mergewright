# 20d — `⇧⇥` runs the pane cycle backwards; R5' words take the shorter way

Status: done (20d-b closed the graph bar box)
Build with: **Sonnet**. A focus-order fix with one invariant that a test states outright (back undoes forward), then
20c's already-specified `BannerWay`/`way_keys`. Nothing structural, no signature another ticket builds on. Not Fable.
Scout: none — 20c NOTES check 2 is the scout (driven, lines cited). Say so in the report.
Needs: 20c merged (`MARK_RESOLVED`, `tab_keys`, `RefusedAt`, `refusal_tabs`).

## Ruling (planner, 2026-10-09): **defect**, option (a)
- Keymap §2.2 (`02-keymap.md:205`): `Tab` / `Shift+Tab` = next / **previous** pane. Previous of X is the pane whose next is X.
- The code's own doc says so (`update/banner/focus_keys.rs:50-51`: "`BackTab` is the same cycle run backwards"); the
  handlers don't (`focus_keys.rs:31-35`, `:73`; 20c NOTES check 2).
- ux-principles H4 (`ux-principles.md:54`): same key, same meaning; Shift-Tab reversing Tab is the platform convention.
  Today, with a banner drawn, `⇧⇥` **never reaches the graph** — a one-way door on the most common key pair (H3 control).
- (b) rejected: `⇧⇥ banner` from the file list only would ship words built on a back-cycle that contradicts the forward
  one and its own doc.

Forward (unchanged, `banner/stop/tab.rs` `tab_target`): Graph → Files → Diff → banner → Graph.
Backward (target): Graph → banner → Diff → Files → Graph. Today: Graph → Diff, Files → banner (both wrong); banner → Diff,
Diff → Files (right).
Way to the banner after the fix (pane open): Graph fwd 3 / back 1 → `⇧⇥ banner`; Files 2 / 2 → tie, forward `⇥ ⇥ banner`;
Diff 1 → `⇥ banner`.

## Crate set (one builder)
| Crate | Files |
|---|---|
| graft | `update/banner/focus_keys.rs`, `update/banner/stop/tab.rs`, `update/undo.rs`, `update/banner/resolve_words.rs`, `commit_editor/refusal.rs` (only if the speller call changes), `view/rows.rs` (bar input only), `update/tests/keymap_words/*`, `tests/it/resolver/shell_begun.rs`, `tests/it/commit_editor_writes.rs`, MAP.md |
| docs | captures the re-shoot changes; m6/m8 contact sheets via `scripts/contact-sheet.sh` only |

graft-ui: **not in the set.** If the graph bar's banner hint can't carry `⇧⇥` without a graft-ui signature change →
stop, report the signature (`blocked: needs graft-ui`).

## Signatures (20c's, now built)
```rust
// graft (crate-private)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BannerWay { pub forward: u8, pub back: Option<u8> } // back: None when never reached
pub(crate) fn banner_way(state: &State) -> BannerWay;   // counted by stepping the SAME fns the Tab / BackTab
                                                        // handlers call — never a parallel table
pub(crate) fn way_keys(way: BannerWay, keys: &Keymap) -> Option<String>; // shorter bound way, ties forward;
                                                        // one direction unbound → the other; both → None
// tab_keys(tabs, keys) unchanged; way_keys calls it for forward. RefusedAt.tabs → RefusedAt.way.
```
The back step is **derived from the forward one** (the predecessor in `tab_target`'s order), not a second hand-kept
order. If that isn't possible without restructuring `take_focus` → stop, report the line.

## Check first (file:line in NOTES; surprise → stop)
1. Every focus state where a banner is drawn: pane open / closed, `edit` stop (graph holds keys, §2.6a :370), resolver
   open (`Esc` lands on banner, :368). List each forward cycle as built. A state with no forward cycle → note, no fix.
2. Other `BackTab` users outside the graph screen (commit editor `02-keymap.md:427`, list ↔ file pane :526, stash/
   message prompts): `rg -n 'BackTab|prev_pane' crates/graft/src` → list. Not changed; must stay green.

## Acceptance (each tick names its verifying test inline, at tick time)
- [x] Back undoes forward: for every state in check 1, from every focusable place, `Tab` then `⇧⇥` returns to the start,
  and `⇧⇥` then `Tab` does too. Test: `update::banner::focus_keys::tests::back_tab_is_the_tab_cycle_reversed_in_every_banner_state` (red first — seen red: "Tab from Graph → Files, then BackTab"; green after).
- [x] Banner drawn, pane open, graph focused: one `⇧⇥` focuses the banner; from the banner, `⇧⇥` → Diff; from Files,
  `⇧⇥` → Graph (driven). Test: `keymap_words::focus::shift_tab_from_the_graph_reaches_the_banner` (written after the fix: red **not observed**; the pre-fix behaviour it rejects is the one the cycle test saw red).
- [x] No banner drawn: `Tab`/`⇧⇥` order unchanged. Test: existing pane-cycle tests green unedited (name them): `keys::tab_words_tests::tab_target_is_where_tab_lands`, `keys::live_c_tests::tab_banner_is_hinted_only_where_one_tab_reaches_it`, `banner::tests::tab_comes_back_round_to_the_banner`, `banner::tests::tab_into_the_banner_waits_its_turn_behind_the_detail_pane`, `banner::tests::nothing_takes_focus_when_there_is_no_banner_to_focus` (`detail::keys` cycle untouched).
- [x] R5': graph + pane open, `u` → `finish or abort the merge first · ⇧⇥ banner`. Test:
  `keymap_words::undo_journal::from_the_graph_with_the_pane_open_the_clause_takes_the_shorter_way` (written after the fix, red not observed);
  `resolver::shell_begun::u_on_an_entry_from_before_a_shell_merge_refuses_in_words` (:491) updated, quote old/new: `"⇥ ⇥ ⇥ banner"` → `"⇧⇥ banner"`, plus a driven one-`⇧⇥` check.
- [x] Ties go forward: from Files, the clause reads `⇥ ⇥ banner`. Test: `keymap_words::undo_journal::from_the_file_list_a_tie_goes_forward`.
- [x] `app.prev_pane = "none"` → `⇥ ⇥ ⇥ banner`; `app.next_pane = "none"` → `⇧⇥ banner`; remapped `prev_pane = "p"` →
  `p banner` (**deviation**: `p` collides with `graph.pull`, `rewrite.pick`, `stash.pop` — `with_overrides` reports it; test uses `f6` → `F6 banner`). Test: `keymap_words::undo_journal::the_way_is_spelled_from_his_keymap` (three cases).
- [x] Every banner clause goes through `way_keys` (the `c` refusal, commit-editor `operation_refusal`, `undone`'s
  `OperationLive` arm). Test: `keymap_words::stops` sweep green; `commit_editor_writes.rs:475` unchanged (it is the P16 step-count comment, no banner way). Also updated: `resolver::editor_words::every_way_to_open_the_editor_refuses_mid_operation` graph case `"⇥ ⇥ ⇥ banner, then c finishes it"` → `"⇧⇥ banner, then c finishes it"`; `keymap_words::resolve::the_banner_clause_is_spelled_from_his_keymap` "unbound" case now unbinds both `next_pane` and `prev_pane` (one unbound → the other way, per ruling).
- [x] Graph `OperationLive` bar: graph focused, pane open → draws `⇧⇥ banner` (one press works, §2.1). Test:
  `keymap_words::stops::the_graph_bar_names_the_way_to_the_banner_with_the_pane_open` (red first). **Closed by 20d-b B+1** (test written, seen red). Pushes a pinned
  narrow capture over width → stop, report line and widths; no rank change.
- [x] Sweeps green unedited: `every_bound_action_is_hinted_live_or_is_named_here`,
  `no_key_a_live_operation_bar_draws_refuses_while_live`, `no_literal_keys::no_user_facing_literal_spells_a_default_key` (full workspace nextest run, see NOTES).
- [x] Check 2's other `BackTab` users green unedited (name the tests): `commit_editor::tests::tab_walks_subject_body_options_button_and_back`, `stash::tests::tab_flips_the_scope_and_enter_saves_what_is_chosen`, `confirm::tests::tab_toggles_the_cursor`, `name_prompt::tests::tab_toggles_checkout_only_for_a_branch`, `banner::tests::tab_escape_and_backtab_give_the_keyboard_back`.
- [x] Captures re-shot (full `GRAFT_UPDATE_SHOTS=1 cargo nextest run --workspace`: **no file under docs/shots changed** (status clean there), so no sheet re-assembled; no shot held the graph-pane-open banner clause) (`GRAFT_UPDATE_SHOTS=1`), each changed file named in NOTES with its changed line; m6/m8 sheets
  re-assembled by script, each ≤ 200 KB. By eye: user; colour unverified.
- [x] MAP.md (graft): `BannerWay`, `banner_way`, `way_keys`, `RefusedAt.way`; focus cycle line says back = reverse. Checked by reading.

## Touches (report each, how checked)
- **Focus cycle callers:** `take_focus`, `tab_target`, `tabs_to_banner`, `one_tab_to_banner` (`tab.rs:14`, :78) — the
  bar's input becomes "one press either way"; every `refusal_tabs` caller.
- **Hints:** graph `OperationLive` bar, banner's own bar (`Tab` gives keys back — unchanged), `?` row `⇧⇥ Previous pane`
  (`docs/shots/m8-help-*-120x36.txt:13-14` — wording unchanged, confirm).
- **Captures:** any shot holding `⇥ ⇥ ⇥ banner` or the graph bar with a live op and pane open; m6/m8 sheets.
- **Keymap doc (conductor, after merge):** `02-keymap.md` §2.6a :391 → "`⇥` per pane between him and the banner, or `⇧⇥`
  per pane the other way, whichever is fewer (ties forward)"; §2.2 :205 unchanged.
- **Journal/undo:** words only, no new write. **Keymap namespace:** no new key; `Shift+Tab` is §2.2's.
- **20c box R5'**: close as superseded by 20d (conductor).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k tokens. One context.

## NOTES (builder, 2026-10-09)

No scout: 20c NOTES check 2 served (conductor's instruction). No compaction.

**Cause fixed (not symptom).** `focus_keys.rs` `take_focus` took the banner on `BackTab` from Files (the predecessor of Graph in the
*pane-only* cycle), and `hand_the_keyboard_back` hard-coded `BackTab → Diff`. Now the cycle is one fn, `stop/tab.rs` `next_of`;
`prev_of` is derived from it (the place whose `next_of` is here), and `take_focus`, leaving the banner, `tab_target` and
`banner_way` all step those two. Back: Graph → banner → Diff → Files → Graph.

**Check 1.** States with a banner and a cycle: conflict stop and `edit` stop (`rewrite_stop_tests::edit_stop`), pane closed
(Graph ⇄ banner) and open (Graph → Files → Diff → banner). Resolver open: `modal_owns_keys` (`state.rs:974`) → `tab_target`
`None`, no cycle, no fix. All four states in the cycle test.

**Check 2** (`rg BackTab|prev_pane crates/graft/src`): `stash.rs:219,403`, `confirm.rs:180`, `detail/keys.rs:163`,
`name_prompt.rs:275`, `finish/keys.rs:50`, `preview/keys.rs:61`, `journal/keys.rs:78`, `commit_editor/keys.rs:62`,
`cli/man.rs:303`. None changed.

**Signature changes.** `BannerWay` is `pub` (not `pub(crate)`): `operation_refusal` and `refusal_way` are `pub` and integration
tests call them. `refusal_tabs` renamed `refusal_way -> BannerWay`; `operation_refusal(.., way: BannerWay, ..)`;
`CommitEditor::refusal(.., way)`; `BannerWay::tabs(n)` for forward-only. `banner_way` returns `Option` (no banner → `None`).

**Graph bar box — blocked: needs graft-ui.** `view/rows.rs:603` feeds `HintContext::OperationLive { tab_to_banner: bool }`;
graft-ui draws the fixed `WAY_BACK = hint(hint_keys::NEXT_PANE, "banner")` (`graft-ui/src/graph_screen/hints/filters.rs:264`,
used `:92-96`). A bool cannot say `⇧⇥`. Needed: `tab_to_banner: bool` → e.g. `way_to_banner: Option<WayToBanner>`
(`Forward | Back`) plus a `PREV_PANE` hint key / `WAY_BACK_PREV` hint. Today the bar draws no banner hint with the pane open
on the graph (true, not a false promise). Test `the_graph_bar_names_the_way_to_the_banner_with_the_pane_open` not written.

**Captures.** `rg "⇥ ⇥ ⇥ banner" docs/shots TESTING.md` → none. Re-shoot run: 6195/6196 passed; the one failure is `graft-graph::it raster::write_shots` (`crates/graft-graph/tests/it/raster.rs:894`), only under `GRAFT_UPDATE_SHOTS=1` - passes without it; outside crate set, not touched, not investigated (question for conductor). No capture changed.

**Gates:** fmt ok, clippy -D warnings ok, nextest workspace 6196 green except that env-only shot writer, test --doc ok.

**Red first.** Only the cycle test was seen red before the fix; the other new tests were written after it (red not observed).

## 20d-b additions (conductor, as found, no new decision)

Part A stopped at the graph-bar box (blocked: needs graft-ui). Continue in graft-ui (+ graft wiring, docs):

- [x] B+1 — the box above: graph `OperationLive` bar, graph focused, pane open, draws `⇧⇥ banner` via `way_keys`/`BannerWay`, not the fixed `WAY_BACK` (`filters.rs:264`). Remaps respected as in the R5' row. Test: `keymap_words::stops::the_graph_bar_names_the_way_to_the_banner_with_the_pane_open` (red seen: bar without the fix read `c continue · ⏎ changes · …`, no banner hint; green after; covers `f6` → `F6 banner`, `prev_pane = none` → no hint, file list → no hint). Re-shot: `docs/shots/m6/fix-c-live-frames-187x50.txt:115` gains leading `⇧⇥ banner ·` (drops `j/k move` to fit); `m6-contact-sheet-3.txt` re-assembled by script (168 595 B).
- [x] B+2 — `docs/design/02-keymap.md` §2.2: one line saying `⇧⇥` runs the pane cycle in reverse, banner included. Doc, checked by reading.

## NOTES 20d-b (builder, 2026-10-09)

No scout: Part A NOTES named the code (conductor's instruction). No compaction.

**Shape.** graft-ui: `OperationLive.tab_to_banner: bool` → `way_to_banner: Option<WayToBanner>` (`Forward | Back`, exported from
`graph_screen`); `WAY_BACK_PREV = hint(PREV_PANE, "banner")` (`filters.rs`), new hint key `PREV_PANE` (`app.prev_pane`).
`every_context` yields each `LiveC` × {Forward, Back, None}; coverage slots 115–118 (`COVERAGE_SLOTS` 115 → 119).
graft: `banner::one_press_to_banner(state) -> Option<WayToBanner>` (`stop/tab.rs`): Forward where `one_tab_to_banner`, else Back
where `banner_way(..).back == Some(1)` and `app.prev_pane` is bound; `view/rows.rs` feeds it. `bound.rs` gains `app.prev_pane`
(the bar now draws it live; the sweeps demand it).

**Tests changed (expected by the feature):** `live_c_tests::no_graph_row_context_is_unwrapped_while_live` (pane-open graph form
`false` → `Some(Back)`); `it::resolver::shell_begun::capture_fix_c_live_frames` (`!bar.contains("⇥ banner")` → bar has
`⇧⇥ banner` and no forward `⇥ banner`); sweeps in `update/tests.rs` now filter states by `one_press_to_banner`.

**Gap found (not fixed, out of scope — question for conductor):** focus cycling matches raw `KeyCode::Tab`/`BackTab`
(`banner/focus_keys.rs:29-30`, `detail/keys.rs:163`), not `app.next_pane`/`app.prev_pane`. So a remapped `prev_pane = "f6"`
is *spelled* `F6 banner` (words, bar) but F6 does not move focus. Pre-existing since Part A's `way_keys` and before for `Tab`.

`raster::write_shots` fails under `GRAFT_UPDATE_SHOTS=1` (known, filed, not touched).
