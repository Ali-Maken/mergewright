# confirm-box-takes-y: `y` confirms a confirm box, as in every preview

Status: done 2026-09-25 (conductor chose option (a); see Decisions). It had been blocked at find-and-pin 1 (operate.rs:657 label not a function of `then`). Before that: ready (unblocked 2026-09-24: P12 and P15 approved, P15 **option (b)** — never in the force-push box; picks
items 3 and 13; ADR-0044 §3; keymap §2.3b and §2.3c edited in the spec pass. Reconciled with scout 50 on 2026-09-25)
From: `docs/reviews/m6-usability.md` M1 (:65-75), the part that depended on P12.
Build with: builder-opus. Why: brownfield across two crates, a public graft-ui type change every hint test enumerates,
and a rebase on 52's change to the same enum. Not Fable: one match decides; the bar-vs-key test makes drift visible.
Scout: `.scratch/m6-conflicts/scouts/50.md` (main @ 3c480347). Read it before any source.
Needs:
- `confirm-box-swallows-unknown-keys` merged (the `_ => {}` arm, `confirm.rs:131-134`, that `y` goes above).
- lane O merged (its boxes); `last-file-opens-the-summary` (60) merged (shared walk rows).
- **Ticket 52 merged. 50 rebases on it** (see "Rebase on 52").
Crates: graft-ui (`hints/lists.rs`, `hints/catalogue.rs`, the `HintContext` enum, `graph_screen/tests.rs`), graft
(`update/confirm.rs`, `view/rows.rs`, tests, walks).
Lane: wave 2, fourth (after 60, 52).

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §3 (the rule, its three exceptions) and §4.
- `docs/design-notes/m6-usability-fixes.md` P15 (:138-145; its hint-bar text is the one this spec draws), the Journeys
  rows "Push" (:59) and "Rejected push" (:62), "Decided here" item 1 (:81-84).
- `docs/design/02-keymap.md` §2.3b (`y` row, :224) and §2.3c.
- `docs/adr/0038-m5-spec-decisions.md` §1 and §4.
- `crates/graft-ui/MAP.md` line 9 (`confirm`), line 22 (`HintContext`); `crates/graft/MAP.md` §`update::confirm`
  (:254-258), §`update::remote` (`push_box`, `undo_after_push_box`).
- 52's spec, Signatures (the `LiveC` change to the same enum).

## Decisions (conductor, 2026-09-25)
- **(a): the `y` word comes from the acting button's own label.** The word and the button come from one value, so
  they can't drift apart. It's the same principle as decision 2 in `m6-usability-fixes.md`, where one value makes
  both forms. An untracked file shows `y delete` over `[ Delete ]`; any other file shows `y discard`. The stash-drop
  box's safe button stays `Keep`, and `y` names that box's acting button (`y drop`).
- How it's built: `ConfirmThen::takes_y()` is the one exhaustive match for ADR-0044 §3 (lease push, undo-after-push
  and revert give `false`). `PendingConfirm::yes()` returns
  `takes_y().then(|| ConfirmYes::from_label(view.choices[1]))`. The key and the bar both read `PendingConfirm::yes()`.
  This replaces the Signatures' `ConfirmThen::yes`. `ConfirmYes` gains `Delete`.

## Where `y` is shown: the hint bar (decided here)
- **The box draws no footer** (scout 50: `confirm.rs:244-246`, empty bottom border). Every key a box answers is on the
  screen's hint bar today: `HintContext::ConfirmBox` → `hints/lists.rs:57-61` `←/→ move  ⏎ choose  esc cancel`, chosen
  at `view/rows.rs:332-333`.
- **`y` goes on that bar, first:** `y push  ←/→ move  ⏎ choose  esc cancel`. This is P15's approved text word for word.
- Why not a new footer on the box border, like the preview's (`preview_box.rs:42`):
  - H4 consistency is "same key, same meaning, shown where that modal's keys are shown" (UX §2 #4, #6). A box's keys
    are on the bar; `y` alone on the border would split one box's keys across two places. Moving all of them to the
    border duplicates the bar or blanks it — a redesign of every box, not this ticket.
  - The key means the same as the preview's `y` (confirm the one acting choice), and it leads its key row, as in the
    preview. That is the consistency ADR-0044 §3.1 asks for.
- **The word after `y`** is the acting button's label, lowercased (`y push`, `y drop`, `y discard`, `y abort`,
  `y start over`), per ADR-0044 §3.1 `y <verb>`. The preview's `y confirm` is not touched.

## Find and pin first (before any change; stop if the finding differs)
1. **Every box's acting label.** For each `PendingConfirm {` site in scout 50's table (11 production sites), record in
   NOTES: the `ConfirmThen` value, the label at index 1, the label at index 0. Expect index 1 to be the one acting
   choice and index 0 `Cancel` for every site ADR-0044 §3 makes `y`-live.
   - `operate.rs:657` (Discard Paths, "label = verb"): list every label it can produce and what decides it.
   - `undo.rs:509` `drift_box` (`Undo { redo }`): its acting label for `redo` false and true.
   - **Stop** (`blocked: box <site> has <n> acting choices / label not a function of then`) if any live box has more
     than one acting choice, or its acting label cannot be decided from its `ConfirmThen` alone.
2. **The bar over a box during a live operation** (the banner's skip and abort boxes). Pin which `HintContext` the
   bar gets while `state.confirm.is_some()` and `live_operation(state).is_some()`, after 52: expect a bare
   `ConfirmBox { .. }`, **not** wrapped in 52's `OperationLive(.., LiveC)`. Wrapped → **stop**, report it: the bar
   would draw a `c` hint the box swallows.

## Scope
**In:**
- **One decision point:** `ConfirmThen::yes`, one exhaustive match. The key (`confirm_key`) and the bar
  (`view/rows.rs` :333) both read it; nothing else decides. No flag on `ConfirmView`, no opener: the construction sites
  are **not** edited (the old spec's `open` and `with_yes` are dropped — the decision reads `then`, which every site
  already sets).
- **The rule** (ADR-0044 §3): `yes()` is `None` for `UndoAfterPush`, `RevertTo`, and `Operate(Push)` with
  `lease.is_some()` (`push_box.rs:30` lease and `after_push.rs:247` `repush_chosen`). `Some(verb)` for every other box
  in the find table (`Undo { redo }` drift box included: ADR-0044 :87 names only the revert drift box).
- **The key:** `KeyCode::Char('y')` with `yes().is_some()` acts on index 1 whatever the cursor is on. With `None` it
  falls to the swallow arm: box open, cursor and status untouched.
- **The bar:** `ConfirmBox { yes: Some(v) }` → `y <v.word()>` first, then today's three hints; `yes: None` → today's
  bar, byte-identical.

**Out:** the preview's `y`; the finish summary; the undo-after-push box's cursor; anything a box does on `Enter`; a
box-border footer; the construction sites.

## The failing test first
`graft/src/update/confirm.rs` tests: `y_confirms_the_push_box_from_cancel`.
- Setup: a plain push box, cursor on Cancel. Press `y`.
- Assert: one `Command::Operate(Push ..)` emitted, `confirm.is_none()`.
- Red today: the swallow arm keeps the box.

## Signatures
```rust
// graft-ui, hints: new enum; one variant changes (unit → struct).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmYes { Push, Drop, Discard, Skip, Abort, StartOver, Undo, Redo /* + one per extra label find 1 lists */ }
impl ConfirmYes {
    /// The word after `y` in the bar: the acting button's label, lowercased ("start over" for StartOver).
    #[must_use] pub fn word(self) -> &'static str;
}
// HintContext::ConfirmBox { yes: Option<ConfirmYes> }   // was the unit variant ConfirmBox

// graft, update/confirm.rs
impl ConfirmThen {
    /// The only place ADR-0044 §3 is decided. No wildcard on `ConfirmThen`; inside `Operate(op)` a wildcard to
    /// `None` is allowed for operations no box opens, with a comment saying so.
    #[must_use] pub(crate) fn yes(&self) -> Option<ConfirmYes>;
}
/// The body of today's `Enter` arm (`choose`, :139-201), split so `Enter` and `y` cannot answer differently.
fn act(state: &mut AppState, commands: &mut Vec<Command>, chosen: usize, next_id: /* confirm_key's next_id type */);
// confirm_key: KeyCode::Enter => act(.., view.selected(), ..);
//              KeyCode::Char('y') if pending.then.yes().is_some() => act(.., 1, ..);   // above the `_ => {}` arm
// view/rows.rs :333: HintContext::ConfirmBox { yes: pending.then.yes() }
```

## Rebase on 52
- 52 changes `HintContext::ChangesPane` and `OperationLive` (adds `LiveC`); 50 changes `ConfirmBox`. Same enum, same
  `every_context` (`catalogue.rs:34`), same per-context counts (`graph_screen/tests.rs:1982-1983`), same
  `view/rows.rs` (52 at :376-392 and :554; 50 at :333).
- Start from main **after** 52 merges. Take 52's variants, forms and counts as the base; add the `ConfirmBox` forms
  (`yes: None` plus each `ConfirmYes`) on top. Recount from 52's numbers; NOTES says old → new and why.
- Find-and-pin 2 is the one behavioural seam between the two.

## Touches (each entry, and how the builder checks it)
- **`HintContext::ConfirmBox` constructions:** `view/rows.rs:333`; tests `update/tests.rs:3896, :4180, :4244`
  (`box_over_the_graph`). Check: the build; each test passes with the box it builds.
- **`every_context`** yields `ConfirmBox` with `None` and each `ConfirmYes`. Check: `every_context` coverage and
  `every_key_the_bar_draws_is_answered` green over the new forms (its harness gets a box whose `yes()` is each value).
- **`update/keys/bound.rs`** and `every_bound_action_is_hinted_live_or_is_named_here`. Check: green; if `y` in a box
  needs naming there, name it.
- **Bar and key agree:** `the_bar_offers_y_exactly_where_y_acts` — a table over every find-1 site: bar has `y` ⇔
  pressing `y` acts.
- **The verb matches the button:** `the_y_word_is_the_acting_label` — for every find-1 site,
  `then.yes().map(ConfirmYes::word) == Some(labels[1].to_lowercase())`.
- **Construction sites:** not edited. The `PendingConfirm {` grep is no longer an acceptance check (it had 5 test-only
  hits anyway: `confirm.rs:206, :390`, `update/tests.rs:4246`, `update/search/tests.rs:122`, `view/frame/tests.rs:134`).
  Check instead: `git diff --stat` shows none of the 11 site files except `confirm.rs`.
- **Captures of a box's bar.** Check: `grep -rln "←/→ move  ⏎ choose" docs/shots` (not `⏎ act`, which matches the
  preview). Each hit is regenerated by its own test; NOTES lists file → test. Known hits: `m3-confirm-discard.txt`,
  `m5-ui-undo-after-push-80x24.txt`, `m6/fix-shell-merge-abort-and-undo-120x40.txt`; the builder confirms
  `m6-09-start-over-box-*` and `fix-start-over-whole-file-*` (scout 50 could not).
  - Byte-identical: the lease push box (`m4-02-push-box-lease-*`), the undo-after-push box, the revert drift box.
    Check: `git diff --exit-code` on those files.
  - A bar with no regenerating test: report it, don't hand-edit.
- **Contact sheets** m3, m4, m5, m6: the builder lists which; the conductor rebuilds (`scripts/contact-sheet.sh`).
- **The narrow bar:** `a_narrow_bar_keeps_the_y_hint` at 80×24, push box and discard box: the bar starts `y push` /
  `y discard` after its margin.
- **Walks moved to `y`:** `walk-back-a-done-file` (`j d y`), `walk-back-a-whole-file-choice` (`k d y`),
  `b-abort-shell-merge` (`a y`) — `tests/it/m5_walk/conflict_journeys.rs:989-1005, :1038-1060, :795-806`; note rows
  `docs/design-notes/m6-conflicts.md:504, 505, 511`. Check: `journey_counts_match_the_note` with the rows below.
- **Walks that keep `→ ⏎`** (they prove the cursor path still works; not moved): `tests/it/m3_walk.rs:510`,
  `m4_walk/push.rs:329, :370`, `m4_walk/credentials.rs:81`, `m4_walk/hangs_and_undo.rs:241`,
  `two_boxes_over_content.rs:309`, `remote_by_key.rs:910`. Check: each green; NOTES names the box each drives.
  - In `m4_walk/push.rs`, add one plain-box leg `P y` and record the observed count (expect 2).
  - Where a site drives the lease, undo-after-push or revert box, add one assertion there that `y` leaves the box up.
- **`.scratch/m6-conflicts/pty-walk-m6.py`** and any `.scratch/*/pty-walk*.py` with `→ ⏎` on the three moved rows.
  Check: `grep -n "'→', *'⏎'" .scratch/*/pty-walk*.py`; update the three rows only.
- **Journal and undo:** none new; `y` runs `act`. Check: `y_and_enter_emit_the_same_commands`, one row per `yes()`
  value (Enter with the cursor on index 1 vs `y` from Cancel).
- **Keymap:** already edited (§2.3b, §2.3c). Read, no edit.
- **TESTING.md** (acceptance ticket edits it): grep `→ ⏎`; list in NOTES the push, discard and moved-walk lines that
  gain "or `y`".
- **MAP.md:** graft-ui line 9/22 (`ConfirmYes`, `ConfirmBox { yes }`); graft §`update::confirm` (`yes`, `act`, the `y`
  arm in `confirm_key`).

## Acceptance (name the verifying test at tick time)
- [x] Find-and-pin 1: the site table in NOTES; `the_y_word_is_the_acting_label` green (`update/confirm.rs` tests; 14 rows, the discard site twice).
- [x] Find-and-pin 2: `the_bar_over_a_box_during_a_live_operation_is_the_box_s_own` (`update/keys/live_c_tests.rs`): 5 table rows × 3 frames × {abort, skip} → bare `ConfirmBox { yes: Some(..) }`, no `c` hint.
- [x] `y_confirms_the_push_box_from_cancel`: red first (panicked `y left the box open`), then green.
- [x] `y_is_swallowed_where_the_box_does_not_take_it`: UndoAfterPush, RevertTo, the lease box from `push_box` (both sites' `then`) plus `repush_chosen` itself. Also shown against real git by `remote_by_key::capture_fix_box_takes_y` (origin unmoved after `y`) and the `m4_walk` legs `push-rejected-then-lease` and `push-plain` (`y` leaves the box up).
- [x] `y_and_enter_emit_the_same_commands` (10 `y`-live rows: same commands, same state after)
- [x] `the_bar_offers_y_exactly_where_y_acts` (every row of `every_box`, through `view::rows::box_context` and `hints_for`)
- [x] `a_narrow_bar_keeps_the_y_hint` (`view/tests.rs`, 80×24, the discard box and the push box)
- [x] `every_context_reaches_every_shape` (graft-ui), `every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here` (graft), all green
- [x] `journey_counts_match_the_note` green at 3 / 3 / 2. `P y` leg: new scenario `m4_walk` `push-plain-by-y`, test `push_plain_by_y`. Its count is 2 keys (`P`, `y`) because those are the only two keys the scenario sends; the m4 walk has no count check.
- [x] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-box-takes-y-120x40.txt` and `-80x24`. Test: `remote_by_key::capture_fix_box_takes_y`. **Differs from the spec on frame 2:** the bottom row is the rejection's sticky line (`push rejected: … · P force with lease`), not the three-key bar. The status line draws a sticky ahead of the bar, and did so before this ticket. The header says so. No `y push` appears anywhere (asserted).
  - Scene: the plain push box; second frame the lease box.
  - **What right looks like:** frame 1's bottom row (the hint bar) reads `y push  ←/→ move  ⏎ choose  esc cancel`,
    whole at 120 and at 80; the box's bottom border is plain `╰…╯` as today; `[ Cancel ]` under the cursor. Frame 2's
    bar is `←/→ move  ⏎ choose  esc cancel`, no `y`.
- [x] MAP.md updated (doc, no test): graft-ui `ConfirmBox { yes }` and `ConfirmYes`; graft `PendingConfirm::yes`, `ConfirmThen::takes_y`, and the `y` arm of `confirm_key`.

## For the conductor (exact text, in this spec's merge commit)
In the `journey-counts` fence, set these `keys` (comments unchanged):
```
walk-back-a-done-file         3
walk-back-a-whole-file-choice 3
b-abort-shell-merge           2
```

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: one graft-ui enum change on 52's base, one match, many capture regenerations. One context. If it grows, hand
off at the seam: graft-ui committed first.

## NOTES

**2026-09-25, builder (opus), on main @ 6f8c033a (52 merged). BLOCKED at find-and-pin 1. No code changed.**
Used scout 50 (its site table matched the code at every row).

`blocked: box operate.rs:657 has 1 acting choice / label not a function of then`

Find 1: sites (`then` / index 1 / index 0):
| Site | `then` | [1] | [0] |
|---|---|---|---|
| `update/stash.rs:482` | `Operate(StashDrop)` | Drop | **Keep** (not `Cancel`; still the safe choice) |
| `update/banner.rs:495` | `Resume(Skip)` | Skip | Cancel |
| `update/banner.rs:528` | `Resume(Abort)` | Abort | Cancel |
| `update/undo.rs:483` | `RevertTo` (excluded) | Revert | Stop |
| `update/undo.rs:509` | `Undo { redo }` | `Undo` (redo false) / `Redo` (true) | Stop |
| `update/operate.rs:515` | `Operate(Discard(Patch))` | Discard | Cancel |
| `update/operate.rs:657` | `Operate(Discard(Paths([path])))` | **`Discard` or `Delete`** | Cancel |
| `update/resolver/restart.rs:74` | `RestartConflict` | Start over | Cancel |
| `update/remote/push_box.rs:30` | `Operate(Push)`: plain / lease | Push (:191) / lease `act` (:221, excluded) | Cancel |
| `update/remote/after_push.rs:221` | `UndoAfterPush` (excluded) | Undo locally (+ [2] Re-push old tip) | Cancel |
| `update/remote/after_push.rs:247` | `Operate(Push)` lease (excluded) | as push_box :221 | Cancel |

The problem: at `operate.rs:643-647`, `Delete` is chosen for an untracked entry, `Discard` otherwise. The flag
comes from `status_entry(next, &path).untracked`, and it is **not** in `then`: `Target::Paths` holds only the
path (`graft-ops/src/operation.rs:151`). So `ConfirmThen::yes()` cannot return `Delete` for that box, and
`the_y_word_is_the_acting_label` cannot pass for an untracked file. The m3/04 review closed exactly this
mismatch (the comment at :639-642): `y discard` above a `[ Delete ]` button.

Options for the spec (pick one):
- (a) `yes()` decides only whether `y` is live. The bar's word is the view's own label at index 1, lowercased,
  read at `view/rows.rs` `box_context`. The type would be `ConfirmBox { yes: Option<ConfirmYes> }` with a
  `Delete` variant, mapped from that label, or a word carried as text. One decision still holds for the key;
  the word cannot drift from the button because it *is* the button. Cheapest option, graft-only apart from the enum.
- (b) Put the verb in `then`: a `ConfirmThen::Operate` sibling or a field (e.g. `Discard { untracked: bool }`).
  It edits a construction site, which the spec says not to do, or `graft-ops`, which is outside my crates.
- (c) Accept `y discard` over `[ Delete ]`. Not recommended: it brings back the m3/04 defect.

Find 2 (read, no test yet): the box's bar is **not** wrapped. `box_context` (`view/rows.rs:324-340`) returns a
bare `HintContext::ConfirmBox`. It is asked first in `footer_context` (:367) and `hint_context` (:561), before
`live_wrapped`. `live_wrapped`'s own list (:399-405) doesn't name `ConfirmBox` either. So 52 doesn't block.
The pinning test is still to write.

Next step once the spec answers: start from the failing test `y_confirms_the_push_box_from_cancel` as written.
Everything else in the spec stands.

**2026-09-25, builder (opus), resumed with decision (a). Done.** Used scout 50. Did not compact.

What was built:
- graft-ui adds `ConfirmYes` and changes the variant to `HintContext::ConfirmBox { yes }`.
- graft adds `ConfirmThen::takes_y` (the ADR-0044 §3 match) and `PendingConfirm::yes`. `confirm_key`'s `Enter` and
  `y` both call `act`. `box_context` reads `yes()`.
- None of the box-opening sites changed (a diff on stash, banner, undo, operate, resolver/restart and remote/ is empty).

Deviations from the spec, each on purpose:
- The spec named `ConfirmThen::yes`. The decision lives in `PendingConfirm::yes`, because the word is read from the view.
- The old test `a_key_the_box_does_not_bind_leaves_it_open*` pressed `y` on the push box. It now presses `Y` and `q`.
- `two_boxes_over_content::capture_box_keeps_on_y_for_docs_shots` is removed. The plain push box takes `y` now, so
  that test's claim is false. `fix-box-keeps-on-y-120x40.txt` is retaken over the **lease** box by
  `capture_fix_box_takes_y`, with a new header and a new caption.
- The sweep fixture `box_over_the_graph` (`update/tests.rs`) builds each `yes` form. `None` is the revert guard;
  `Some(v)` is the undo guard labelled with `v.label()`.
- `keys/bound.rs` gains `("y", <word>)` for each of the 9 words.

Counts: coverage slots 54 → 63 (25 = `None`, 54–62 = each `ConfirmYes`). `every_context` gains 9 forms.

Journey rows lowered in `docs/design-notes/m6-conflicts.md` by me (the builder), as asked:
`walk-back-a-done-file` 4→3, `walk-back-a-whole-file-choice` 4→3, `b-abort-shell-merge` 3→2. The walks now send `y`.

Walks that keep `→ ⏎` are unchanged and green. They drive: `m3_walk` (discard), `m4_walk/push` (plain and lease push),
`credentials` (plain push), `hangs_and_undo` (push, undo-after-push), `two_boxes_over_content` (plain push),
`remote_by_key` (plain push). The pty scripts had no `'→', '⏎'` rows.

Captures retaken (file → test):
- `m3-confirm-discard`, `m3-confirm-delete-untracked`, `m3-undo-drift` → `view::tests::capture_m3_*`. The
  delete-untracked header said "`Discard` beside it"; it now says `Delete`, which is what the box draws.
- `m5-ui-confirm-over-graph-80x24` and `-160x48` → `two_boxes_over_content::capture_two_boxes_over_real_content_for_docs_shots`.
- `m6/fix-plain-box-after-rebase-187x50` → `remote_by_key::capture_fix_plain_box_after_rebase`.
- `m6/fix-shell-merge-abort-and-undo-120x40` → `resolver::shell_begun::a_on_the_banner_aborts_a_shell_merge`.
- `m6/fix-start-over-whole-file-120x40` → `resolver::file_list::start_over::d_on_a_done_modify_delete_file_reopens_its_card`.
- `m6/m6-09-start-over-box-120x40` and `-80x24` → `pty-walk-m6.py` on `start-over-brings-the-markers-back`. A second
  run reported "settled".
- `m6/fix-box-takes-y-*` (new) and `m6/fix-box-keeps-on-y-120x40` → `remote_by_key::capture_fix_box_takes_y`.

Byte-identical (diff exit code 0): `m4-02-push-box-lease-*`, `m5-ui-undo-after-push-80x24`, `m4-remote-keys`.

**Stale, not retaken:** `m4-01-push-box-plain-80x24` and `-160x48` (pty, `pty-walk-m4.py`). They already showed the
graph's bar under the box, which predates rulings 07b. A retake wrote an unredacted
`/var/folders/.../graft-remote-…` path, so I reverted it. The m4 script needs its path redaction back before a retake.

Contact sheets: I rebuilt m6 (3 parts: 176 195 / 178 148 / 137 605 B). The m5 sheet (`m5-contact-sheet-2`) and the m3
sheet include retaken captures; the conductor rebuilds those.

TESTING.md lines that gain "or `y`" (the acceptance ticket edits them): 1261 (discard untracked, `d →`), 1302
(stash-drop), 1490 (push, `→ ⏎`), 2332–2334 (the three moved walks: the keys become `j d y`, `k d y`, `a y` and the
counts become 3, 3, 2).

Question (rule 7, no line filed): after a rejection the lease box's keys are never on the bar, because the sticky line
covers them. Is that intended? It predates this ticket; `fix-box-takes-y-*` frame 2 shows it.
