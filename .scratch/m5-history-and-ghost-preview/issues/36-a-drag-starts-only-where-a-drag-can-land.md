# a-drag-starts-only-where-a-drag-can-land

Status: done
Blocked by: **33** (shares `graft`, `graft-ui`).
Crates: `graft`, `graft-ui`.
Build with: **Sonnet.** Every item below arrives with the line that proves it
and the shape of the fix. None of them decides an approach.
Scout: **not needed** — every line is quoted below, from the Minor section of
`docs/reviews/m5-10-30-31.md`.

This is the review's eight Minors, triaged by the conductor and filed as one
group because they share two crates. Three are real defects the user can hit
(m1, m3, m6); the rest are a lying caption, two stale doc references, a
missing test and a panic path.

**Check each line against the tree you actually have before you fix it.**
Tickets 33, 34 and 35 landed after this review was written, and 33 in
particular rewrote the hit-test seam. If a finding is already fixed, say so
and tick the box naming what fixed it — that is a true tick.

## m1. A drag can start while the search prompt owns the keyboard

`crates/graft/src/update/drag.rs:146-156`. `can_start` lists eight conditions
by hand and omits the search prompt (`search_prompt`) and the focused banner
(`banner_focus`). With the prompt open, a left press on a badge starts a drag;
the prompt then claims every key (`update/mod.rs:582-585` routes to
`search_key` before `key_event`), so `drag_key` never runs — and **the footer
reads `release drop on a row  esc cancel` while the user is typing**. Ticket
10 piece 4 is explicit: "a drag cannot start while … a prompt … is open."

**Fix.** `state.text_field_open() || state.banner_focus || …`, keeping the
overlay/menu/preview conditions. Prefer the existing predicate over a ninth
hand-listed condition — a hand list is what failed here.

## m2. A resize does not cancel a drag

`Event::Resize` (`update/mod.rs:590-598`) does not touch `state.drag`, but
ticket 10 piece 4 lists "a resize" among the cancellations. Harmless today
(the next `Drag` event re-reads `over`), but it is a **sixth undisclosed
deviation** where the NOTES list five.

**Fix.** Clear `drag` there, or add it to the NOTES as a disclosed deviation.
Either is acceptable; say which you chose and why.

## m3. "Over" ignores the column

`drag.rs:71` sets `over` from `graph_row_at(state, mouse.row)`, which takes
the **row only** (`geometry.rs:283`). Dragging sideways into the detail pane
at the same height keeps the graph row as the target, and **releasing there
opens the drop menu** for a row the pointer is not on. The ticket says: "the
working-tree row, header, footer, pane → None."

**Fix.** Check the column against `primary_pane_area` before accepting the
row. This is the same family as ticket 33's M1 — a drag landing somewhere the
user is not pointing — so if 33 changed how `over` is computed, build on what
it left, do not restore the row-only call.

## m4. A 14-space run in three shipped captions

`crates/graft/tests/it/hint_bar.rs:446` contains a 14-space run mid-sentence
("at 80 the              Detail tab owns the screen"), reproduced at line 10
of `docs/shots/m5-hints-detail-{80x24,120x40,160x48}.txt`. A string
continuation swallowed a newline but not its indentation. Textual, and it
stands on the capture alone (rule 7).

**Fix** the string and regenerate those three captures. They are in the
contact sheet the user scrolls, so the sheet needs regenerating too.

## m5. Two doc references name tests that do not exist by that path

- `crates/graft/MAP.md:567` (the `bound_actions` entry) still says
  `every_hinted_key_is_bound`, **twice**; ticket 31 renamed it.
- `crates/graft/src/view/hit.rs:10` cites
  `view::tests::badge_spans_match_the_drawn_cells`; the test is
  `graft_ui::graph_screen::tests::…`. **Ticket 33 may already have rewritten
  this line** — its M1 was that very seam. Check before touching it.

## m6. A refused `stash branch` preview loses the typed name

`update/name_prompt.rs::create` sets `state.name_prompt = None` before
`open_preview`; on `!opened` it restores `stash.shown` but **not the prompt**,
so a name typed while a write is running is gone and must be retyped.

**Fix.** Keep the prompt open, with the status line explaining the refusal.
A name the user typed is their work; losing it to a transient refusal is the
smallest version of the thing this project exists to prevent.

## m7. `border_row`'s cut-label branch has no direct test

`frame_box.rs::border_row` gained a second branch (`full > inner`: reserve one
blank cell before `right`). The sweep reaches it only where a title happens to
overflow at one of five widths — so it is covered by coincidence.

**Fix.** A one-line unit test with a label longer than `width - 2`, pinning
the branch regardless of which widget's title grows.

## m8. `ContextMenu::selected_refusal` indexes unchecked

`graft-ui/src/context_menu.rs::selected_refusal` does `self.items[self.cursor]`.
Every constructor today guarantees a non-empty list (`for_drop` is only
reached with non-empty `verbs`; `drop_menu` returns `None` otherwise), so
**this is not reachable** — it is not a defect. But `.get(self.cursor)
.and_then(..)` costs nothing and removes a panic path from a widget crate.

Do **not** replace the panic with a silent wrong answer (rule 3). `None` here
must mean "no refusal", which is what the call site already handles.

## Also: answer the review's Q1

`view/frame.rs:415` draws the Detail tab's bar from `pane_hint_context(state)`
when an overlay (Map, journal, commit editor) owns the primary tab — so the
bar offers `c`, `m`, `r`, `b`, `t`, `y`, `x`, `/` **with the overlay behind
the tab**. The sweep never builds an overlay-plus-pane state (`screen_for`,
`update/tests.rs:3588`, has none). **No line proves a key is dead there**, so
the review filed it as a question, not a defect, and so does this ticket.

**Build that state and find out.** Does the overlay's `shown_key` claim those
keys before the graph does? Report the answer with the line. If some are dead,
that is a defect and the fix is ticket 31's rule — *every key the bar draws is
answered by the handler that has focus* — applied to a state the sweep cannot
currently reach; extend `screen_for` so it can. **If answering it needs a
decision about which bar the Detail tab should draw, stop and report** — that
is a design call, not yours.

## NOT yours

- The review's **Q2** (`1 first-parent` on the Detail-tab bar). Answered, and
  a design question; the conductor has filed it as a gap.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's. Report rows.
- The 22 unwalked acceptance scenarios — a separate gap, not this ticket.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] A drag cannot start while the search prompt or a focused banner owns the
      keyboard — tests: `update::drag::tests::drag_cannot_start_while_the_search_prompt_is_open`,
      `update::drag::tests::drag_cannot_start_while_a_banner_has_focus`.
      **Failed before the fix**, confirmed by reverting `can_start` and
      re-running: both panicked ("no badge was picked up over the
      prompt"/"...under the focused banner").
- [x] A resize either cancels the drag or is a disclosed deviation — **fixed**
      (cleared `state.drag` on `Event::Resize`, `update/mod.rs`), not
      disclosed as a deviation — test: `update::drag::tests::resize_cancels_a_drag`.
- [x] A press at a graph row's height but in the detail pane's columns gives
      `over = None` — test: `update::drag::tests::dragging_into_the_detail_pane_gives_no_target`.
      **Failed before the fix**: reverting `drag_mouse`'s Drag arm to the old
      `geometry::graph_row_at(state, mouse.row)` call reproduced `over =
      Some(0)` at 160 columns with the detail pane split beside the graph.
- [x] The 14-space run is gone from `hint_bar.rs:446` and from all three
      captures and the contact sheet — corrected sentence: "the pane is open
      on the tip with its diff focused: at 80 the Detail tab owns the screen,
      wider it sits beside the graph."
- [x] Both stale doc references name tests that exist — `crates/graft/MAP.md`
      (`bound_actions` entry, was `every_hinted_key_is_bound` twice) now says
      `every_key_the_bar_draws_is_answered`, the test's real name
      (`update/tests.rs:3376`). `hit.rs:10` **was already fixed by ticket
      33** — it already reads
      `graft_ui::graph_screen::tests::badge_spans_match_the_drawn_cells`,
      which exists (`crates/graft-ui/src/graph_screen/tests.rs:1756`); no
      change needed there.
- [x] A name typed into the stash-branch prompt survives a refused preview —
      test: `update::name_prompt::tests::a_refused_preview_keeps_the_typed_name`.
      **Failed before the fix**: reverting `create`'s `!opened` branch to
      drop `reopen` reproduced the prompt closed with `typed(&after)` panicking
      ("the prompt is open").
- [x] `border_row`'s cut-label branch has a direct test — test:
      `frame_box::tests::border_row_reserves_a_blank_cell_before_a_cut_labels_corner`.
- [x] `selected_refusal` cannot panic — test:
      `context_menu::tests::selected_refusal_on_an_out_of_range_cursor_is_none_not_a_panic`
      (also the type-level reason in the doc comment: `.get`, not indexing).
- [x] Q1 answered with a line, **and it is a real defect, filed as a
      question because fixing it is a design call**: see "Q1 — answered"
      below. Not fixed by me.
- [x] Whole suite green; report before and after — see report.

Rule 6: you do not tick "looks right". The user does.

## Q1 — answered, with lines, not fixed

**Yes, the bar lies in this state.** With `state.map.shown` (or
`state.journal.shown`/`state.stash.shown`) true and the Detail tab holding
focus (`state.detail.focus != Graph`, `size.0 < 160` so the panes sit
side-by-side rather than a Tab strip is irrelevant — reproduced at 80
columns, the Tab layout `render_overlay_with_detail` uses), the pane's own
bar (`view/frame.rs:433`, `pane_hint_context` → `PlainRow` →
`plain_row_list`, `graft-ui/src/graph_screen/hints.rs:736-748`) draws `b
branch`, `r rebase`, `t tag`, `c commit`, `y sha`, `m merge`, `S stash`, `J
journal`, `/ search`. None of them do anything: `update/keys.rs:180`
(`pane_and_overlay_key`) tries `detail::detail_key` first (claims `v w W [ ]
L` and cursor motion in the diff, none of these nine), then
`super::map::map_key`, whose `shown_key` (`update/map.rs:157-176`) claims
**every** key while `map.shown` — including these — and does nothing with
them (`_ => true` at `map.rs:176`). `journal::shown_key`
(`update/journal.rs:198-234`, `_ => {}` at 234, still returns `true`) and
`stash::shown_key` (`update/stash.rs:184-219`, `_ => {}` at 219) do the same,
except for `Tab`/`BackTab`.

Verified empirically, not just read: a throwaway test built exactly this
state (`AppState { size: 80x24, map.shown: true, detail.shown: true,
detail.focus: Diff, ..default() }`) and pressed each of the nine keys
through `update::update`. Every one left the state and the command list
unchanged. (Not committed — a scratch probe, per the ticket's "not yours to
fix".)

**Why I did not fix it.** Ticket 31's rule is "every key the bar draws is
answered by the handler that has focus" — but *which* handler has focus
here is exactly the ambiguity: `detail.focus != Graph` says the pane has it,
yet the bar drawn is the graph's own `PlainRow` context (branch/tag/merge/
etc.), which belongs to a row on a graph that is not even the visible
screen (the overlay is). Two different fixes are both plausible and neither
is mine to pick: (a) route these nine keys through to the graph's own
handlers even while an overlay+pane share the Tab layout, or (b) stop
`pane_hint_context` from drawing graph-row hints in this state and give the
pane its own (probably empty, or overlay-aware) bar instead. Reporting per
the ticket's own instruction: "if answering it needs a decision about which
bar the Detail tab should draw, stop and report."

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** (`cargo test --doc` is vacuous workspace-wide — run it, never
offer it as evidence.)

`CARGO_TARGET_DIR=/tmp/graft-target-g36`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

Which of the eight were already fixed by 33/34/35. Whether m2 was fixed or
disclosed. Q1's answer, with the line. Anything unverified — which is not
broken.
