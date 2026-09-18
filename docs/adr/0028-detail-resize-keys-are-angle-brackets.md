# Detail-pane resize keys are `<` and `>`, not Ctrl+arrows

Status: **Accepted** (user, 2026-09-17). Changes the M2 design note
(`docs/design-notes/m2-details-and-diffs.md`, Keymap row and "Keys:" bullet)
and ticket `m2-details-and-diffs/10-detail-keys-and-focus`.

## Context

The M2 design note bound `Ctrl+←/→` (side-by-side pane) and `Ctrl+↑/↓`
(stacked pane) to resize the detail pane. On macOS those chords belong to
the system: Mission Control and Spaces take `Ctrl+←/→`/`Ctrl+↑/↓` before the
terminal sees them (ADR-0005: macOS is a primary platform). The binding would
work on Linux and silently do nothing on the Mac the user lives on.

## Decision

- **`<` shrinks the pane, `>` grows it**, in 4-cell steps, whenever the pane
  is shown and the layout has a divider: columns in `Right` (side by side,
  ≥ 160 columns), rows in `Below` (stacked, 100–159 columns). One key pair,
  one meaning ("less pane / more pane") in both orientations, so the user
  never has to know which way the divider runs.
- The **focused divider** is the layout's single divider: `Right` has one
  vertical `│`, `Below` one horizontal `─`. The keys act on it regardless of
  which region (Graph, Files, Diff) holds keyboard focus, because the pane
  being shown is what makes the divider exist.
- **No-op** where no divider exists: the `Tab` layout (< 100 columns), the
  pane hidden, or the Map view covering the frame. The keys do nothing and
  show nothing; they are not an error.
- Divider **drag** with the mouse and the **4-cell step** stay as specified.
  The size persists per orientation in `<common_dir>/graft/layout`, since a
  column count set side by side means nothing as a row count when the window
  narrows into the stacked layout.

## Rejected

- **`+`/`-`**: `+` is already "unfold every lane" on the graph (docs/design/03-graph-layout-algorithm.md §3, TESTING.md); rebinding it while the pane is shown would make one key
  mean two things depending on state.
- **`Ctrl+arrows`**: taken by macOS, above.
- **`Alt+←/→`/`Alt+↑/↓`**: free (DESIGN.md uses `Alt+↑/↓` only inside the
  rebase editor) and the natural fallback if `<`/`>` prove awkward in daily
  use. Not bound now: two bindings for one action is one more thing to
  document, and `Alt` on macOS terminals depends on the "Option as Meta"
  setting, which is off by default in Terminal.app.

## Consequences

- `<`/`>` were unbound everywhere (docs/design/02-keymap.md §2.2–2.5 keymap, M1 keys, every
  M2 spec, source): no collision, no rebind.
- The keys are listed with the pane's other key hints (`w W v [ ]`) and in
  `?` context help, since `<`/`>` for resize is not a convention users guess.
- If `<`/`>` turn out awkward, a follow-up ADR adds `Alt+arrows` as an alias
  without removing them.
