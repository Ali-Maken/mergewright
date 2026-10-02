# ADR-0055: Keys are one table; a remap is translated at the edge

Status: accepted (planner, 2026-10-02, M8 pass 2a; design is ours, user memory "stop bringing him design decisions").
Amends DESIGN.md §2.1's last bullet. Design note: `docs/design-notes/m8-polish-and-ship.md`, Hard-to-reverse choice 2.

## Context

PROMPT §5 asks for remappable keys; `docs/design/02-keymap.md` §2.1 promises "every binding is remappable" with
collisions warned at startup. Today every key is matched by name in ~50 `*_key` dispatchers, and the hint bar, the
`.` menu labels and the confirm boxes each spell keys from their own strings. Research (`keymap-help-remap.md`): the
top complaint is a silent remap collision (gitui #2704, lazygit #5763); tig cannot clear a key (#816); help that shows
default keys after a remap is a bug (helix #13483).

## Decision

1. **One table.** `graft_ui::keymap::defaults()` holds every binding of `02-keymap.md` as data: action id, context,
   default chords, label. The hint bar, `?`, the palette, `.` labels, the tour card and the man page all spell keys
   through `Keymap::spell`. No surface keeps its own key string.
2. **Action ids are the config's API.** `<context>.<verb>`, snake_case, from 02-keymap.md's words (`graph.merge`,
   `app.undo`). Shown beside each key in `?`. **Never renamed after release**; a rename keeps the old id as an alias.
3. **Translate at the edge.** `update` calls `Keymap::translate(context, pressed)` once, before any dispatcher. A
   remapped chord becomes the action's **default** chord; a default chord whose action moved elsewhere becomes
   `Unbound` (dropped, with nothing else done). Dispatchers are not rewritten.
4. **Config:** `[keys.<context>] <verb> = "<chord>"` or a list; `"none"` unbinds. Problems are warnings, never fatal:
   unknown action, unparsable chord, a collision (both actions named, defaults included), a reserved chord
   (`Ctrl+C`). The graph opens as normal and the status row says `config: N problems · ? shows them`.

## Why not the alternatives

- **Rewrite every dispatcher to match on actions:** ~50 functions, every one a chance to drop a key; the same result
  with the risk spread across the codebase.
- **Remap by editing the hint strings only:** help would lie, the exact helix complaint.
- **An in-app key recorder** (JetBrains, VS Code): heavy for a TUI; he has not asked. File only.

## Consequences

- A sweep test remaps every action to a free chord, walks every context, and asserts no default key is spelled and
  every remapped key acts. It is what keeps the edge honest.
- `update::keys::bound_actions` and the hint tables move from key strings to `ActionId`.
- Modal text fields (commit editor, prompts) are not remapped: typed characters stay characters.
