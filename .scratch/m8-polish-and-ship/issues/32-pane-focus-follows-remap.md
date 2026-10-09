# 32 — a remapped pane key moves focus wherever `⇥` / `⇧⇥` did

Status: draft
Build with: **Sonnet** (provisional; conductor confirms after the scout). ADR-0055 already decided the mechanism
(translate at the edge, dispatchers keep matching defaults); this finds where the pane keys miss the edge and routes
them through it. Driven tests judge it. **Opus** instead if the scout puts the cause in `Keymap::translate`'s own
context resolution (graft-ui), since that touches every remap. Not Fable: no new signature, nothing structural.
Scout: **yes** (brownfield, ADR-0035), `scout`, ≤1 page, questions in "Check first" 1–3.
Needs: none (20d, 20d-b merged).
Source: KNOWN_GAPS `M8/pane-focus-ignores-remap` (**broken**); 20d NOTES 20d-b "Gap found".

## Ruling (planner, 2026-10-09)
**Defect.** With `app.prev_pane = "f6"`, the graph bar and the refusal words spell `F6 banner` (20d-b), and F6 does
not move focus. The hint promises a key that does nothing.
- `02-keymap.md` §2.1 :183-185: "Graft translates a pressed chord to the action's default chord **before any pane
  sees it**; a default whose action was moved is unbound." :189-190: every surface spells from the live table.
  ADR-0055 §3: `translate` runs once, before any dispatcher. Its Consequences: the sweep asserts "every remapped key
  acts".
- ux-principles H4 (`ux-principles.md:54`): same key, same meaning, every pane. A remap that works on one pane and
  not the next is two meanings for one action.

**1. Remaps of `app.next_pane` / `app.prev_pane` drive pane focus everywhere `⇥` / `⇧⇥` drive it as the §2.2 pane
cycle.** That is: graph ↔ file list ↔ diff (detail pane), entering the banner, and leaving it (§2.6a :384 is the same
cycle reversed, §2.2 :205, 20d). Wherever the bar or the words spell either action, pressing the spelled chord does
what the words say.

**2. Not the commit editor, nor any place with its own `⇥` row.** §2.10 :495 (commit editor), §2.6b :427 (finish
summary), §2.7 :444 (rebase editor), §2.9 :480 (stash prompt), §2.16 :456 (branch picker), §2.17 :526 (history),
the confirm box and the name prompt: each `⇥` there is that place's own row, a different action. §2.1 :191-193: "a
place answers its own rows … before anything it falls through to"; :196-197 and ADR-0055 amendment: inside a text
field no line acts. So an `app.next_pane` remap neither moves nor kills them. If any of those places **spells**
`app.next_pane` / `app.prev_pane` while acting on its own `⇥` row → in scope: spell its own row instead.

**3. The raw `⇥` / `⇧⇥` do not stay bound alongside.** §2.1 :184-185: a moved default is unbound — one rule for
every action. Keeping them live would (a) make these two actions the only ones whose default survives a remap, and
(b) steal `⇧⇥` from any action he rebinds onto it (a silent collision, the top complaint in ADR-0055's research). He
keeps both with the list form, `prev_pane = ["shift+tab", "f6"]` (§2.1 :182).

**Fix the cause, not the dispatchers.** Do not add `F6` arms or per-dispatcher keymap lookups to `focus_keys.rs` /
`detail/keys.rs` — that is a second path ADR-0055 rejected. The handlers keep matching `Tab` / `BackTab`; the pressed
key must reach them translated. Where it doesn't, that is the cause.

## Check first (scout; file:line in its file; surprise → stop)
1. Where `translate` is called (`update`), and whether `banner/focus_keys.rs:29-30` and `detail/keys.rs:163` see the
   pressed key **before** it, or after it under a context where `app.*` isn't live. Name which, with lines.
2. Whether `app.next_pane` / `app.prev_pane` are in `defaults()` with contexts covering graph, detail pane and banner;
   whether the banner's `⇥` / `⇧⇥` leave row is spelled from them or from a separate id. A separate id → stop, report
   (an action-id question, not a build call).
3. `rg -n 'KeyCode::(Tab|BackTab)|next_pane|prev_pane' crates/graft/src crates/graft-ui/src` → every site, each marked
   *pane cycle* or *own row* (decision 2). 20d NOTES check 2 is the starting list: `stash.rs:219,403`, `confirm.rs:180`,
   `detail/keys.rs:163`, `name_prompt.rs:275`, `finish/keys.rs:50`, `preview/keys.rs:61`, `journal/keys.rs:78`,
   `commit_editor/keys.rs:62`, `cli/man.rs:303`, plus `banner/focus_keys.rs:29-30`.
4. Why the ADR-0055 remap sweep ("every remapped key acts") missed it: name the test and the gap (pane keys exempt, or
   banner / detail contexts not walked).

## Crate set (one builder)
| Crate | Files |
|---|---|
| graft | the `translate` call site (scout names it), `update/banner/focus_keys.rs`, `update/banner/stop/tab.rs`, `update/detail/keys.rs`, `update/tests/keymap_words/*`, the ADR-0055 remap sweep (scout names it), any own-row place from check 3 found spelling `app.*`, MAP.md |
| graft-ui | `keymap/` **only** if check 1 puts the cause in `translate`'s context resolution. Any other graft-ui file → stop, `blocked: needs graft-ui`, name the signature |
| docs | `docs/design/02-keymap.md` §2.2 :205 (one clause), captures the re-shoot changes, sheets by `scripts/contact-sheet.sh` only |

## Signatures
None new. Calls `Keymap::translate` as ADR-0055 §3 defines it; `banner_way`, `way_keys`, `one_press_to_banner`
(20d / 20d-b) unchanged. A signature change in graft-ui → stop and report it.

## Acceptance (each tick names its verifying test inline, at tick time)
- [ ] The gap's case: `prev_pane = "f6"`, live op, pane open, graph focused: bar reads `F6 banner`; `F6` focuses the
  banner; `F6` again → diff. Test: `keymap_words::focus::the_remapped_way_to_the_banner_is_the_key_that_takes_it` (red
  first; quote the red).
- [ ] Remap drives the whole cycle: `next_pane = "f5"`, `prev_pane = "f6"`; in every banner state of 20d's cycle test
  (no banner; banner with pane closed / open; `edit` stop) and from every focusable place, `F5` lands where `⇥` landed
  before the remap and `F6` where `⇧⇥` did. Derived by stepping `tab_target` / `prev_of`, not a hand table. Test:
  `keymap_words::focus::remapped_pane_keys_run_the_same_cycle_in_every_banner_state` (red first).
- [ ] Moved defaults are unbound: same remap, `⇥` and `⇧⇥` move no pane focus anywhere in the cycle; with
  `prev_pane = ["shift+tab", "f6"]` both move it. Test: `keymap_words::focus::a_moved_pane_default_is_unbound_unless_listed`.
- [ ] Own rows survive a pane remap (decision 2): with `next_pane = "f5"`, `prev_pane = "f6"`, `⇥` still walks the
  commit editor, the finish summary, the stash prompt, the confirm box, the name prompt, the branch picker, the rebase
  editor (<120 cols) and history (<120 cols) exactly as before; `F5` / `F6` type nothing and move nothing inside a text
  field. Test: `keymap_words::focus::a_place_keeps_its_own_tab_under_a_pane_remap` (one case per place).
- [ ] The ADR-0055 remap sweep covers `app.next_pane` / `app.prev_pane` in graph, file list, diff and banner focus (the
  gap check 4 named, closed). Test: that sweep, named, seen red on today's code before the fix.
- [ ] No surface spells a pane key that doesn't act: every hint or words string that spells `app.next_pane` /
  `app.prev_pane` (graph bar, banner bar, `way_keys` clauses, detail pane hints, `?` rows) is checked under the f5/f6
  remap by pressing what it spells. Test: covered by the sweep above, or name the test that does it.
- [ ] 20d's tests green unedited: `banner::focus_keys::tests::back_tab_is_the_tab_cycle_reversed_in_every_banner_state`,
  `keymap_words::focus::shift_tab_from_the_graph_reaches_the_banner`, `keymap_words::undo_journal::the_way_is_spelled_from_his_keymap`,
  `keymap_words::stops::the_graph_bar_names_the_way_to_the_banner_with_the_pane_open`, and 20d's own-row list
  (`commit_editor::tests::tab_walks_subject_body_options_button_and_back`, `stash::tests::tab_flips_the_scope_and_enter_saves_what_is_chosen`,
  `confirm::tests::tab_toggles_the_cursor`, `name_prompt::tests::tab_toggles_checkout_only_for_a_branch`,
  `banner::tests::tab_escape_and_backtab_give_the_keyboard_back`).
- [ ] Sweeps green unedited: `every_bound_action_is_hinted_live_or_is_named_here`,
  `no_key_a_live_operation_bar_draws_refuses_while_live`, `no_literal_keys::no_user_facing_literal_spells_a_default_key`.
- [ ] Captures: full `GRAFT_UPDATE_SHOTS=1 cargo nextest run --workspace`; default keymap, so **no shot is expected to
  change** — any that does, named in NOTES with its changed line and why; sheets re-assembled by script, ≤200 KB each.
  `raster::write_shots` fails under that env (ticket 29) — note, don't touch. By eye: user; colour unverified.
- [ ] `02-keymap.md` §2.2 :205 gains: "Remapped `app.next_pane` / `app.prev_pane` move focus wherever `Tab` /
  `Shift+Tab` do, banner included; the defaults are then unbound (list both to keep them). A place's own `Tab` row
  (§2.6b, §2.7, §2.9, §2.10, §2.16, §2.17) is not the pane cycle." Checked by reading.
- [ ] MAP.md (graft): where pane keys are translated. Checked by reading. KNOWN_GAPS row closed by the conductor.

## Touches (each in the report's **Touches:** line, how checked)
- **Every raw `Tab` / `BackTab` match site** from check 3, each marked pane cycle (now fed translated) or own row
  (unchanged, test named above).
- **Focus cycle:** `take_focus`, `tab_target`, `next_of`, `prev_of`, `banner_way`, `one_press_to_banner` (`stop/tab.rs`).
- **Hints and words:** graph `OperationLive` bar (`WAY_BACK` / `WAY_BACK_PREV`, graft-ui `filters.rs`, read only),
  banner's own bar, detail pane hints, `?` rows `⇥ Next pane` / `⇧⇥ Previous pane`, every `way_keys` clause (`c`
  refusal, commit-editor `operation_refusal`, `undone`'s `OperationLive` arm), the man page (`cli/man.rs:303`), the
  tour card.
- **Sweeps:** the three above, the ADR-0055 remap sweep, `keymap_words::stops`.
- **Captures:** any shot spelling `⇥` / `⇧⇥`; m6/m8 sheets.
- **Mouse focus:** clicks move focus without keys; confirm unaffected by the existing mouse tests (name one).
- **Journal/undo:** none (focus only). **Keymap namespace:** no new key or action id.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~40k tokens. One context.

## Conductor addition (scout contradiction, rule 7 — verify before building)

Scout `scouts/32-pane-focus.md`: the remap step (`edge::key`, `edge.rs:41`) runs before every dispatcher, so the defect ("F6 does not move focus") could not be confirmed from code. Nothing was run.

- [ ] V0 — first, before any other box: a test with `app.prev_pane = "f6"` (and one with `app.next_pane`) presses the remapped key from the graph with the pane open and from the banner, and asserts focus moves as §2.2 says. Name it. **If it is green on HEAD, the defect does not exist: stop, report, do not build the rest** (the conductor closes the gap row). If red, it is the red-first test for the boxes above.
- [ ] V1 — the sweep gap the scout found: `sweep_tables.rs:29-30` skips Graph `app.*_pane` as "pane closed". Cover it with the pane open. Test: name it.
