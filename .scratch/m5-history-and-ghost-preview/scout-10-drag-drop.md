# Scout — ticket 10 (drag-drop, name prompt, stash branch)

Sonnet `Explore`, 2026-09-21, read-only. Builds on `scout-app.md` §1–§2, which
it does not repeat. **Code, not intent** — every claim carries a line.

## 1. Which mouse kinds actually reach `update`

`MouseKind` has 8 variants (`crates/graft-core/src/event.rs:183-198`):
`Down(btn)`, `Up(btn)`, `Drag(btn)`, `Moved`, `ScrollUp`, `ScrollDown`,
`ScrollLeft`, `ScrollRight`. `MouseButton` is `Left|Middle|Right` (`:205-212`).

`mouse_event`'s match (`crates/graft/src/update/mouse.rs:35-107`) reaches:

| Kind | Line |
|---|---|
| `ScrollUp` / `ScrollDown` | `:35-36` |
| `Down(Left)` | `:37-70` |
| `Down(Right)` | `:74-93` |
| `Drag(Left)` **only if `next.detail.dragging`** | `:95-97` |
| `Up(_)` — any button | `:98` |
| `Moved` | `:101` |

Everything else falls to `_ => {}` (`:102`): `ScrollLeft`/`ScrollRight`,
`Down(Middle)`, **`Drag(Left)` while not dragging**, `Drag(Middle|Right)`.

**With a context menu open, only `Down(Left|Right)` are read at all**
(`:24-32`) — hover included is dropped for that frame.

## 2. Row and column mapping

`geometry::graph_row_at` (`geometry.rs:263-283`) takes **only `screen_row`**.
`screen_row - rows_above_graph(state)` gives a body offset; with
`state.detail.shown` it first confirms a graph rect exists via
`detail_split(...).0?` (`:264-269`); subtracts 1 more for the uncommitted-changes
virtual row (`:276-280`); then `absolute = history.top + offset`, bounds-checked
(`:281-282`).

**Column is never consulted for the graph hit-test.** A click anywhere across a
row's width maps to that row.

## 3. Divider claim and release

Claimed at `mouse.rs:47-51` — `Down(Left)` where
`geometry::detail_hit(...) == Some(DetailHit::Divider)` sets
`next.detail.dragging = true`. Released **unconditionally on any `Up(_)`**
(`:98`), not scoped to `Up(Left)` nor to being over the divider. While held,
`Drag(Left)` calls `drag_divider` (`:95-97`, `:121-135`).

`Bounds` (`geometry.rs:34-49`) is a plain `{x,y,width,height}` mirrored from
`ratatui::Rect` by hand plus `contains(column,row)`. **`pub(super)`** — visible
only inside `crates/graft/src/update`.

## 4. THE BLOCKING FINDING — badge column ranges are not knowable outside the widget

Nothing exposes a badge's on-screen column range outside
`crates/graft-ui/src/graph_screen`:

- `Columns` (`layout.rs:358`) / `Columns::compute` (`:371-424`) gives
  `subject_x` / `subject_width` for the **whole subject column**, not per badge,
  and is `pub(super)` — not in `mod.rs`'s `pub use` list (`mod.rs:33-36`).
- The per-badge x/width packing loop is `GraphScreen::draw_badges`
  (`render.rs:267-297`), a **private** `impl` method, called only from the
  widget's own `render` (`:154`).
- `badges_width` (`render.rs:329-339`, `pub(super)`) gives only the **total**
  cells all badges take, for truncation math.
- `GraphEntry.badges: Vec<Badge>` **is** `pub` (`mod.rs:325`), so the app crate
  can see *which* refs are on a row and their text via `Badge::text(icons)`
  (`badges.rs:82-116`) — but not **where** each lands after packing and
  truncation.

`crates/graft/src/update/geometry.rs` hand-mirrors `graft_ui::detail_pane`'s
layout to stay free of the rendering crate, **and a test greps for that**
(`geometry.rs:1-5`). So it has nothing to mirror for badges: the packing
algorithm and its inputs are private to `graph_screen`.

## 5. The credential modal, the one to copy

`CredentialPrompt` (`credential.rs:152-163`): `title`, `prompt`,
`field: PromptField` (`Open(TextField)` | `Secret(SecretField)`), `job: JobId`.

`credential_key` (`:317-388`) is modal: returns `false` only when
`state.credential` is `None` or on `Ctrl+C` (`:323`); swallows unused control
chords (`:326-327`) and Alt combos (`:330-332`). `Tab` deliberately falls
through (`:383-385`) — one field, nothing to move between.

`credential_paste` (`:391-397`): `false` if closed, else
`prompt.field.insert_str(text)`.

`text_field_open` (`state.rs:498-503`) has exactly four members: search prompt,
commit editor, stash prompt, credential.

**Draw order** (`view/frame.rs`): `draw_credential` is called **last**, after
`draw_context_menu` then `draw_confirm`, in every arm — `draw_journal`
(`:89-93`), `draw_stash` (`:112-116`), `draw_map` (`:147-151`), plain graph
(`:244-249`), and alone on `Screen::Status` (`:256`). Intent confirmed at
`frame.rs:718`. **`draw_commit_editor` (`:148-168`) never calls
`draw_credential`** — could not confirm how editor + credential co-occur.

## 6. Stash `b`

`shown_key` (`stash.rs:186-219`) claims `S`/`Esc`, `Up`/`k`, `Down`/`j`,
`space`, `s`, `Enter`, `p`, `d`, then `_ => {}` (`:218`). **`b` is unclaimed**
and falls into that wildcard. A new arm goes beside `p`/`d`, under the same
gate (`stash_key` `:157-163`, `:167-169`).

`StashState::selected_entry` (`:74-77`) → `Option<&StashEntry>`;
`StashEntry { index, commit, message, at, paths }`
(`crates/graft-git/src/stash.rs:29-42`). So
`next.stash.selected_entry().map(|e| (e.index, e.commit))`.

**`stash.selected` (the cursor) and `entry.index` (`stash@{N}`) are different
fields** that coincide today because rows are index-ordered. Do not conflate
them.

## 7. No disabled row exists

`MenuItem` (`context_menu.rs:57-63`) has exactly two fields: `action`, `label`.
No enabled/disabled/dim flag on `MenuItem`, `ContextMenu`, or the render loop
(`:177-201`) — the only per-row branch is cursor `Modifier::REVERSED`
(`:186-192`). Whole file read.

## 8. `b` and `t` confirmed unbound on the graph

Grep over `crates/graft/src/update/*.rs` finds only `Char('B')` at
`commit_graph.rs:45` (the commit-graph offer's one-shot initial, the exception
`02-keymap.md` §2.1 names) plus test echoes. No lowercase `b`, no `t`/`T`
anywhere — covering `keys.rs`'s `graph_key` and `pane_and_overlay_key` and
every overlay module. `02-keymap.md` §2.3 names both as design intent only.
