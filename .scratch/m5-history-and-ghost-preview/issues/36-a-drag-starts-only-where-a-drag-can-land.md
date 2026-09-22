# a-drag-starts-only-where-a-drag-can-land

Status: ready
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

- [ ] A drag cannot start while the search prompt or a focused banner owns the
      keyboard — test: name it. **Must fail before the fix.**
- [ ] A resize either cancels the drag or is a disclosed deviation — say which,
      and name the test or the NOTES line.
- [ ] A press at a graph row's height but in the detail pane's columns gives
      `over = None` — test: name it. **Must fail before the fix.**
- [ ] The 14-space run is gone from `hint_bar.rs:446` and from all three
      captures and the contact sheet — quote the corrected sentence.
- [ ] Both stale doc references name tests that exist — quote the new wording,
      and say if 33 had already fixed `hit.rs:10`.
- [ ] A name typed into the stash-branch prompt survives a refused preview —
      test: name it. **Must fail before the fix.**
- [ ] `border_row`'s cut-label branch has a direct test — name it.
- [ ] `selected_refusal` cannot panic — name the test or the type-level reason.
- [ ] Q1 answered with a line: which keys the overlay claims and which the
      graph does. If any bar key is dead there, it is fixed or reported.
- [ ] Whole suite green; report before and after.

Rule 6: you do not tick "looks right". The user does.

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
