# Scout: graft-ui for M6 group 02 (ui-conflict-resolver)

Explore (sonnet), 2026-09-22, read-only. Written by the conductor from the
scout's reply — the `Explore` agent has no write tool (same gap as M5).
Every claim carries its line (CLAUDE.md rule 7).

## 1. `hints.rs` — contexts, `planned()`, the modal list

- `HintContext` variants: `crates/graft-ui/src/graph_screen/hints.rs:221-330` —
  `WorkingTreeRow` (222), `BranchRow { current }` (224), `PlainRow` (230),
  `Marked(Box<HintContext>)` (235), `Banner { skippable, conflicted }` (239-249),
  `Preview` (252), `ChangesPane { unmerged }` (254-258), `Journal` (261),
  `Stash` (264), `Map` (268), `PaneOverOverlay { opens }` (291-296),
  `Dragging` (299), `PaneFocused { row, opens }` (318-323).
- **`planned()` is used in exactly one place**: `changes_pane_list` at
  `unmerged = true` — `M mergetool` (`hints.rs:841`), `space resolved`
  (`hints.rs:843`). The **banner's `M`/`e` are already `Live`**
  (`banner_list`, `hints.rs:817-822`). The ticket's "ChangesPane and Banner
  draw them as planned" is half wrong: only ChangesPane's two are planned.
- **No variant is a "draw the whole list, ignore width" model to copy.** One
  shared arm in `whole_bar` covers `Banner | Dragging | Journal | Stash | Map |
  PaneOverOverlay` (`hints.rs:398-403`): `(drawable(context, divider), None)` —
  no `. more` trailer. All of them still run through `fit_list`'s shrink ladder
  (`hints.rs:571-593`). They have never shrunk because they are 2-5 hints and
  always fit, **not** because shrinking is disabled.
  **`Resolver`'s ~9 keys will be the first list to actually exercise the
  ladder.** Treat the 80-column drop order as new ground, not a known path.
- `every_context()`: `hints.rs:669`.
- **`bound_actions()` is not in `graft-ui`** — it is
  `crates/graft/src/update/keys.rs:732`.

## 2. Unmerged rows in the detail pane

- No `crates/graft-ui/src/detail/` directory: it is
  `crates/graft-ui/src/detail.rs` + `crates/graft-ui/src/detail_pane/`.
- `FileStatusView::Unmerged { how }` defined at `detail.rs:142-145`, drawn at
  `detail_pane/files.rs:54-55` (`how.to_string()` in the trailing column;
  `status_letter` never reaches it — comment `files.rs:21-24`).
- **`how` today is the hardcoded literal `"unmerged"`**:
  `crates/graft/src/jobs/detail.rs:361-368`, `sanitize_line("unmerged")` at
  `:367`. The comment there says carrying the real XY pair needs `graft-git` —
  which is G1's `UnmergedHow`, wired by G4.

## 3. `frame_box` and the column helpers — the ticket's premise is wrong

- `frame_box.rs` is `crates/graft-ui/src/frame_box.rs`, **not** under
  `graph_screen/`. Helpers: `cells` (18), `border_row` (36-66),
  `content_row` (~80).
- **Map, Journal and Stash do not use it.** Its callers are `confirm.rs`,
  `credential_prompt.rs`, `name_prompt.rs`, `preview_box.rs`.
  `map_screen.rs` / `journal_screen.rs` / `stash_screen.rs` draw their own
  borders; grep finds zero `frame_box::*` hits in them. The resolver's border
  work is bespoke, modelled on `map_screen`, or on `frame_box` only if the
  resolver wants a *box*, not a full-frame overlay.
- `snap_to_columns` (`graph_screen/layout.rs:591`) and `column_boundaries`
  (`graph_screen/layout.rs:84`) live in a different file again, and are driven
  from the app crate (`crates/graft/src/view/frame.rs:9,873,977`), not by the
  overlay widgets.

## 4. The `▏` cursor gutter (m5/35)

`row_rule()` — `crates/graft-ui/src/graph_screen/render.rs:444-475`, glyph
selection ~468, ASCII fallback `|`. Graph-row specific, not a shared helper:
the resolver needs its own draw call modelled on its shape logic.

## 5. `every_key_the_bar_draws_is_answered` — and the seam it exposes

- The test is `crates/graft/src/update/tests.rs:3377`: `every_context()` (from
  graft-ui) × `Divider` × `SWEPT_WIDTHS`, checked against `bound_actions()`
  (`update/keys.rs:732`) via `dead_keys_on` / `screen_for` (`tests.rs:3821`).
- **Registering a new `HintContext` variant takes four edits in two crates:**
  (a) the enum + `every_context()` in `hints.rs`;
  (b) arms in `marked_form` (`hints.rs:929`) and `pane_focused_form`
      (`hints.rs:962`) — both exhaustive, no wildcard;
  (c) an arm in `screen_for` (`crates/graft/src/update/tests.rs:3821`) —
      exhaustive, no wildcard, builds a real `AppState`;
  (d) the variant's keys in `bound_actions()`
      (`crates/graft/src/update/keys.rs:732`).

**(c) and (d) are in the `graft` crate.** The ticket's `Crates: graft-ui` /
`Needs: none` is therefore un-buildable as filed: adding
`HintContext::Resolver` fails the workspace-wide compile until the two `graft`
arms exist. **Conductor's ruling (2026-09-22): G2's crate set is widened to
`graft-ui` plus exactly two `graft` files** — `src/update/tests.rs::screen_for`
and `src/update/keys.rs::bound_actions` — and nothing else in `graft`. Logged
as a spec defect (ADR-0036: a cross-crate need is a spec defect, not a build
decision).
