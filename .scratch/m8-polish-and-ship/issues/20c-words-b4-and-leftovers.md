# 20c — words: B4 (`space mark resolved`) fitted, R5's three-Tab path, 20b leftovers

Status: draft (planner spec-only pass, 2026-10-09)
Build with: **Sonnet**. Strings, one short-form row, one speller widened; every judgement call has a stop rule. Not Fable:
no signature another ticket builds on, nothing structural. Scout: none expected; **Check first** is the scout (20b's
pattern). Say so in the report.
Needs: 20b Part A and B merged (`tab_keys`, `RefusedAt`, `refusal_tabs`, `askpass_words` exist).

## Crate set (one builder; two-cargo-helper cap holds)

| Crate | Files |
|---|---|
| graft-ui | `graph_screen/hints/lists.rs`, `graph_screen/hints/hint.rs` (`short_form`), their tests, MAP.md |
| graft | `update/banner/resolve_words.rs`, `update/banner/stop/tab.rs`, `update/undo.rs`, `commit_editor/refusal.rs` (only if the speller's call changes), `update/tests/keymap_words/*`, the `resolver::shell_begun` test, comments `operate/stage_discard.rs:107`, `m5_walk/conflicts.rs:218`, `operate/tests.rs:293`, MAP.md |
| graft-ops | `tests/it/aborted_fetch.rs:6` — **comment only** |
| docs | `docs/shots/m6/*` re-shot files, `docs/shots/m6/m6-contact-sheet-*.txt` (via `scripts/contact-sheet.sh`), TESTING.md only where it quotes a changed line |

Not touched: graft-ui `palette*`, `empty_state*` (list any label there in NOTES; question, not fixed).

## Rulings

Principles: keymap §2.1 (ranked selection with a width budget; "`u undo` is fitted second … what gives way is the
lowest-ranked hint"; a displayed key must work; every key spelled from the live table), §2.2 (`Tab`/`Shift+Tab` next /
previous pane), §2.6a :391; ux-principles §1.1 (KLM: M 1.35 s dominates, K ~0.2 s; Rule 1 drops M on an anticipated key),
§1.3 (steps = keys + decisions), §2 H4 consistency, H6 recognition, H9 next key named, §3.1, §3.6.

| # | Ruling |
|---|---|
| **R6' (B4)** | **Short form, not a fitting change.** Full label stays `mark resolved` (20b R6: a label is a verb; `⏎ resolve` and `space resolved` side by side differ by one letter for two different verbs, §3.6). Its **short form is `mark`** — added to the existing short-form table, the rung that already turns `c continue` into `c cont` at 80 (`docs/shots/m6/fix-tab-words-80x24.txt:25`). Why short form: the 80-col bar is 77 cells today (`⏎ resolve · M mergetool · e edit · space resolved · c cont · ⇥ diff · ? keys`); `mark resolved` makes it 82. `mark` makes it 73, so `⇥ diff` (R4's only pointer to the banner path, pinned) stays. Why `mark`, not `resolved`: the short form must be a prefix of the full label, as `cont` is of `continue` — he recognises it as the same word he reads at 120 and in the refusal (H6, H4) — and must stay a verb (R6). Why not a fitting change: re-ranking `M mergetool` or `⇥ diff` changes every narrow bar's order for one label (§2.1 rank is per-place, not per-width) and moves pinned captures that have nothing to do with R6. The refusal (status line) always reads the full `space mark resolved`. `u`-second is untouched: this bar holds no `u`. |
| **R5' (flag)** | **Words accepted as true; the path gets the shorter way when one exists.** `⇥ ⇥ ⇥ banner` (graph, detail pane open) is true (H9, §2.1). Counted (§1.3): 3 K, 0 M — same chord repeated, fully anticipated (KLM Rule 1), ~0.6 s; within "2–3 do", never 5–6. But §2.2 already binds `Shift+Tab` = previous pane. **If** one previous-pane press from the graph lands on the banner (check first 2 — driven, not read), the clause names the **shorter direction**: `⇧⇥ banner` (spelled from the live table). Ties go forward. Every banner clause uses it (one speller, 20b R5), so from the file list it stays `⇥ ⇥ banner` unless back is shorter. **If** it does not, `⇥ ⇥ ⇥ banner` stands as built — no new key, no Tab-order change (the cycle is spatial, H4; reordering it for one state moves every Tab test). |
| **R5'' (graph bar)** | The graph `OperationLive` bar draws `⇥ banner` (`one_tab_to_banner`). With the detail pane open it may promise one press where three are needed (§2.1 "a key displayed must work"). Check first 3; if untrue, the bar's banner hint goes through the same speller. If that pushes a pinned narrow capture over width → **stop**, report the line and the widths; no rank change here. |
| **L1** | m6 contact sheets re-assembled from their per-scenario files with `scripts/contact-sheet.sh` (ADR-0037) — they still quote 20a/20b's old words (`m6-contact-sheet-4.txt:879, :1128` `no remote named`; `:1263, :1515` and sheet-5 `:398` `space resolved`; the evil-merge line). Never hand-edited. |
| **L2** | graft-ops `tests/it/aborted_fetch.rs:6` comment: `refused · nothing written` → `nothing written` (20b R8). graft comments at `operate/stage_discard.rs:107`, `m5_walk/conflicts.rs:218`, `operate/tests.rs:293`: `space marks resolved` → `space mark resolved`. |

## Signatures

```rust
// graft-ui (pub) — 20b's planned const, now with its short form
pub const MARK_RESOLVED: &str = "mark resolved";
pub const MARK_RESOLVED_SHORT: &str = "mark";   // the short-form table's row for MARK_RESOLVED

// graft (crate-private) — only if check first 2 finds a shorter back way; else unchanged
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BannerWay { pub forward: u8, pub back: Option<u8> }  // back: None when it never reaches the banner
pub(crate) fn banner_way(state: &State) -> BannerWay;                // State = the type refusal_tabs takes today;
                                                                     // counted from the focus-order fns the Tab /
                                                                     // Shift+Tab handlers call — never a parallel table
pub(crate) fn way_keys(way: BannerWay, keys: &Keymap) -> Option<String>; // shorter bound direction, ties forward;
                                                                     // one direction unbound → the other; both → None
// tab_keys(tabs, keys) keeps its signature; way_keys calls it for the forward case. RefusedAt.tabs → RefusedAt.way.
```

## Check first (record each with file:line in NOTES; any surprise → stop, report, no fix)
1. `continue → cont` comes from `hints/hint.rs` `short_form`, and the fitter shortens before it drops a hint. Its comment
   says "the table is the user's": quote it. If it means user config (not a design table) → stop, report.
2. Drive it: live merge, detail pane open, graph focused. Press previous-pane once: which pane has focus? Record the
   action id bound to `Shift+Tab` (keymap table). Then from the file list: back presses to the banner.
3. Same fixture: the graph's `OperationLive` bar — does it draw `⇥ banner`? Press `⇥` once: banner focused?
4. `rg -n 'space resolved|marks resolved|"resolved"|⇥ ⇥ ⇥|refused · nothing written' crates docs TESTING.md` → full list
   of tests, captures, docs to update, in NOTES. Any palette/`?` label for this action: list only.

## Acceptance (each tick names its verifying test inline, at tick time)
- [ ] B4: CONFLICTS row, changes pane, 120 cols → bar reads `space mark resolved`; 80 cols → `space mark`, and `⇥ diff`
  and `? keys` still drawn. Tests: graft-ui `hints::tests::a_conflict_row_says_mark_resolved` (both widths, red first) +
  `m5_walk::every_scenario::hints_at_80_show_three_and_more` and `conflict_interim::tab_walk::capture_fix_tab_words_80x24`
  green, quoted words updated only (quote each changed line in NOTES).
- [ ] B4: `c` refused **on** a CONFLICTS row → status ends `space mark resolved` (full, any width); bar and refusal both
  read `MARK_RESOLVED`. Test: the existing `keymap_words::resolve` pin, updated (quote it).
- [ ] B4: `space` remapped (`[keys.<ctx>] <action> = "x"`) → bar and refusal both spell `x mark resolved`. Test:
  `keymap_words::resolve::the_banner_clause_is_spelled_from_his_keymap` extended, or a new case (name it).
- [ ] R5' — if check 2 found back shorter: graph + pane open, `u` → `finish or abort the merge first · ⇧⇥ banner`, one
  previous-pane press focuses the banner (driven). Test: `keymap_words::undo_journal::from_the_graph_with_the_pane_open_the_clause_takes_the_shorter_way`
  (red first); `resolver::shell_begun::u_on_an_entry_from_before_a_shell_merge_refuses_in_words` drives the new way.
  Previous-pane unbound (`"none"`) → `⇥ ⇥ ⇥ banner`. Test: the remap test above, new case.
  — if not: box ticked "accepted as built", with check 2's recorded focus and line; no code.
- [ ] R5'' — if check 3 found the bar untrue: bar's banner hint spelled by `way_keys`. Test:
  `keymap_words::stops::the_graph_bar_names_the_way_to_the_banner_with_the_pane_open` (red first). If true: tick with
  check 3's line, no code.
- [ ] Sweeps green unedited: `every_bound_action_is_hinted_live_or_is_named_here`,
  `no_key_a_live_operation_bar_draws_refuses_while_live`, `no_literal_keys::no_user_facing_literal_spells_a_default_key`.
- [ ] L2: comments changed (`rg -n 'refused · nothing written|space marks resolved' crates` → no hits; paste it).
- [ ] Captures re-shot (`GRAFT_UPDATE_SHOTS=1`), each named in NOTES. Expected at least:
  `docs/shots/m6/fix-tab-words-80x24.txt` (`… e edit · space mark · c cont · ⇥ diff · ? keys`),
  `docs/shots/m6/fix-tab-words-120x40.txt` (`space mark resolved · c continue · ⇥ diff`),
  `docs/shots/m6/stash-clash-take-esc-pane-120x40.txt` (124 cells full → expect `space mark`, `⇥ diff` and `? keys`
  kept; anything else dropped → stop, report). By eye: user; colour unverified.
- [ ] L1: `scripts/contact-sheet.sh` re-run for m6 (its usage line, captions from `m6-contact-sheet-captions.tsv`).
  Check: every embedded block equals its per-scenario file (script or `diff`, paste the command); `rg -n 'space resolved|evil merge: not checked' docs/shots/m6` → no hits; `no remote named` only where its source file still holds it
  (`operation.rs:910`'s producer — 20b R3 amendment); each sheet ≤ 200 KB (`wc -c`).
- [ ] MAP.md (doc, checked by reading): graft-ui `MARK_RESOLVED`, `MARK_RESOLVED_SHORT`, short-form row; graft
  `BannerWay`/`way_keys` (if built), `RefusedAt`, `tab_keys`.

## Touches (report each with how it was checked)
- **Same words elsewhere:** the `c` refusal (`resolve_words`), commit-editor `operation_refusal` and `stops.rs` sweep (go
  through the speller; shipped keys kept, W4), `live_c_tests` (`⇥ ⇥ banner` off-row), `conflict_interim` relaunch test,
  graph `OperationLive` bar, `undone`'s `OperationLive` arm, palette/`?` label (list only).
- **Captures:** check 4's list + the three above + m6 sheets 1–5. M8 sheet: conductor.
- **Docs (conductor, after merge):** `02-keymap.md` §2.6a :391 — "one `⇥` per pane between him and the banner, or one
  `⇧⇥` per pane the other way, whichever is fewer" (only if R5' built); §2.6a :387 prose `Space` marks it resolved
  (unchanged, prose). TESTING.md only where it quotes a changed line.
- **Journal/undo:** words only. **Keymap namespace:** no new key; `Shift+Tab` is §2.2's.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k tokens. One context.
