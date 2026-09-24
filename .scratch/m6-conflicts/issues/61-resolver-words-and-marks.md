# resolver-words-and-marks: `no more conflicts this way`; a stop line that keeps a short origin; no tint on a hand-edited both-take

Status: ready
From: user picks 2026-09-24 items 7 (batch 6, "shorten both") and 8 (batch 2, `no more conflicts this way`); ADR-0044
§2 row "took both, then a hand edit"; KNOWN_GAPS M6/15 :693, M6/21 :769-770; `docs/log/m6.md:78`.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: (1) `update::resolver` child `walk`: the cross rule's rule 3 (`no more conflicts · ⏎ marks <file> done`,
and a card's side-keys form) and every place that string reaches (notice, sticky status, the bar). (2)
`update/banner/stop.rs`: `stop_reason`, `stop_wordings` — the exact rungs today, with an example per origin (`begun
outside Graft`, `from a previous run`); where the footer picks the fullest rung that fits (`graft-ui` `hints/fitting.rs`
`fit_status_line` / `StopWording`) and on what width. (3) where `view::resolver` builds `DocView` and whether
`Choices.edited` is reachable there; how `panes/draw.rs` (after `conflict-band-and-state-colours`) picks the took-both
split.
Needs: `conflict-band-and-state-colours` merged (the took-both split this changes); ticket 51 merged (crate order).
Crates: graft, graft-ui.
Lane: wave 2, sixth (last build before acceptance).

## Read first
- `.scratch/m6-conflicts/user-batch.md` items 2 and 6.
- `docs/design/02-keymap.md` §2.6 the `n` / `N` row (edited 2026-09-24).
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §2 (the hand-edit row).
- `crates/graft/MAP.md` §update::resolver (the cross rule :343; `Choices` :355), §update::banner stop.rs (:307-311).
- `crates/graft-ui/MAP.md` lines 12 (`DocView`), 15 (panes), 16 and 22 (`StopWording`, `fit_status_line`).

## Scope
**In:**
- **Batch 2:** the cross rule's rule 3 says exactly `no more conflicts this way`, no keys, as a notice (not a second copy
  in the bar; the bar keeps its own `⏎ done`). A card's form: `no more conflicts this way` too, the side keys stay in the
  bar only. Every place the old string was drawn.
- **Batch 6:** `stop_wordings` gains a rung right after the full one, **before** any rung that drops the origin:
  - shell: `<kind> in progress · from the shell · <n> conflicted`
  - a previous run: `<kind> in progress · from last run · [step k of m · ]<n> conflicted`
  - then the existing drops, in order, unchanged. Singular counts as today (`1 conflicted` reads the same).
  The full rung keeps its words (`begun outside Graft`, `from a previous run`, `files conflicted`).
- **Took both, then a hand edit:** `DocView` carries whether the open file's result was edited by hand
  (`Choices.edited`); when it is, the result rows of a `Took(Both)` block get no tint (ADR-0044 §2). Separators and the
  side panes as for took both.

**Out:**
- The 80x24 relaunch stop line (KNOWN_GAPS :772).
- Short forms (`short-words-from-the-draft`).
- Any change to when `n`/`N` wrap or cross (P2 and ticket 15 stand).

## The failing test first
- graft `update::resolver` walk tests: `n_past_the_last_block_says_no_more_conflicts_this_way` — every block taken, no
  other file open: `n` → notice exactly `no more conflicts this way`; the status line does not repeat it with keys.
  Red today: `no more conflicts · ⏎ marks <file> done`.
- graft `update::banner` stop tests: `a_shell_stop_keeps_a_short_origin_before_dropping_it` —
  `stop_wordings(shell merge, 13 conflicted)[1] == "merge in progress · from the shell · 13 conflicted"`. Red today:
  rung 1 drops the origin.

## Signatures
```rust
// graft-ui/src/conflict_resolver.rs
pub struct DocView { /* existing …, */ pub hand_edited: bool }   // set from Choices.edited for the open file
// graft/src/update/banner/stop.rs: signatures unchanged; `stop_wordings` returns one more rung.
// graft/src/update/resolver/walk.rs: the rule-3 words become a const
pub(crate) const NO_MORE_THIS_WAY: &str = "no more conflicts this way";
```

## Touches (each entry, and how the builder checks it)
- **Every string that said `no more conflicts`** (walk, card, done file `n`/`N`, tests).
  - Check: `grep -rn "no more conflicts" crates` — every hit uses the const or asserts it.
- **The key row showing the words twice** (`docs/log/m6.md:78`). Check: `no_more_conflicts_is_not_repeated_in_the_bar`.
- **Stop line at the widths he uses:** 187 with the changes pane open, 160, 120.
  - Check: `stop_line_keeps_the_short_origin_at_187_with_the_pane_open` (a `view` test at the graph side's real width),
    and the `m6-18-launch-shell-begun-*` captures regenerated; NOTES says which width shows which rung.
  - Monotonic: more room never loses a part. Check: the existing `stop_wordings` ordering test, extended.
- **KNOWN_GAPS M6/21 :769-770** (the origin never drawn; 160 drops the count): the builder reports what each width now
  draws; the conductor closes or re-words the rows.
- **Hand-edited both-take.** Check: `a_hand_edited_both_take_has_no_result_tint` (graft-ui buffer) and
  `doc_view_carries_hand_edited_from_choices` (graft).
- **Other screens:** the banner's own line (`merge in progress · 2 files conflicted · ⏎ resolve …`) is built from the same
  wordings. Check: `m6-18-*` captures, and the banner row tests green.
- **Walk:** `two-files-out-of-order` and any scenario asserting the old notice. Check: `every_scenario::journey`.
- **Captures:** any showing `no more conflicts` (`grep -rln "no more conflicts" docs/shots`), `m6-18-*`. Regenerate; list.
- **Docs:** `crates/graft/MAP.md` (the cross rule's words, `stop_wordings`' rungs); `crates/graft-ui/MAP.md` line 12
  (`hand_edited`).

## Acceptance (name the verifying test at tick time)
- [ ] `n_past_the_last_block_says_no_more_conflicts_this_way`: red first, then green.
- [ ] `a_shell_stop_keeps_a_short_origin_before_dropping_it`: red first, then green.
- [ ] `a_previous_run_stop_keeps_a_short_origin`
- [ ] `no_more_conflicts_is_not_repeated_in_the_bar`
- [ ] `stop_line_keeps_the_short_origin_at_187_with_the_pane_open`
- [ ] `a_hand_edited_both_take_has_no_result_tint`, `doc_view_carries_hand_edited_from_choices`
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-stop-line-short-origin-187x50.txt` (a shell
      merge, 13 files conflicted, the changes pane open) and `fix-no-more-this-way-120x40.txt`.
  - **What right looks like:** the stop line reads `merge in progress · from the shell · 13 conflicted`, whole; the
    resolver's notice reads `no more conflicts this way`, and the bottom row does not repeat it.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small to medium: three independent word/flag changes sharing the resolver's view files. One context.

## NOTES
