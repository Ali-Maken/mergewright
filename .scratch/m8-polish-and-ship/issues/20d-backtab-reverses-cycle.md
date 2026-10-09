# 20d — `⇧⇥` runs the pane cycle backwards; R5' words take the shorter way

Status: draft
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
- [ ] Back undoes forward: for every state in check 1, from every focusable place, `Tab` then `⇧⇥` returns to the start,
  and `⇧⇥` then `Tab` does too. Test: `update::banner::focus_keys::tests::back_tab_is_the_tab_cycle_reversed_in_every_banner_state` (red first — fails today on Graph and Files).
- [ ] Banner drawn, pane open, graph focused: one `⇧⇥` focuses the banner; from the banner, `⇧⇥` → Diff; from Files,
  `⇧⇥` → Graph (driven). Test: `keymap_words::focus::shift_tab_from_the_graph_reaches_the_banner` (red first).
- [ ] No banner drawn: `Tab`/`⇧⇥` order unchanged. Test: existing pane-cycle tests green unedited (name them).
- [ ] R5': graph + pane open, `u` → `finish or abort the merge first · ⇧⇥ banner`. Test:
  `keymap_words::undo_journal::from_the_graph_with_the_pane_open_the_clause_takes_the_shorter_way` (red first);
  `resolver::shell_begun::u_on_an_entry_from_before_a_shell_merge_refuses_in_words` (:491) updated, quote old/new.
- [ ] Ties go forward: from Files, the clause reads `⇥ ⇥ banner`. Test: `keymap_words::undo_journal::from_the_file_list_a_tie_goes_forward`.
- [ ] `app.prev_pane = "none"` → `⇥ ⇥ ⇥ banner`; `app.next_pane = "none"` → `⇧⇥ banner`; remapped `prev_pane = "p"` →
  `p banner`. Test: `keymap_words::undo_journal::the_way_is_spelled_from_his_keymap` (three cases).
- [ ] Every banner clause goes through `way_keys` (the `c` refusal, commit-editor `operation_refusal`, `undone`'s
  `OperationLive` arm). Test: `keymap_words::stops` sweep green; `commit_editor_writes.rs:475` updated if its state's way changed (quote).
- [ ] Graph `OperationLive` bar: graph focused, pane open → draws `⇧⇥ banner` (one press works, §2.1). Test:
  `keymap_words::stops::the_graph_bar_names_the_way_to_the_banner_with_the_pane_open` (red first). Pushes a pinned
  narrow capture over width → stop, report line and widths; no rank change.
- [ ] Sweeps green unedited: `every_bound_action_is_hinted_live_or_is_named_here`,
  `no_key_a_live_operation_bar_draws_refuses_while_live`, `no_literal_keys::no_user_facing_literal_spells_a_default_key`.
- [ ] Check 2's other `BackTab` users green unedited (name the tests).
- [ ] Captures re-shot (`GRAFT_UPDATE_SHOTS=1`), each changed file named in NOTES with its changed line; m6/m8 sheets
  re-assembled by script, each ≤ 200 KB. By eye: user; colour unverified.
- [ ] MAP.md (graft): `BannerWay`, `banner_way`, `way_keys`, `RefusedAt.way`; focus cycle line says back = reverse. Checked by reading.

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
