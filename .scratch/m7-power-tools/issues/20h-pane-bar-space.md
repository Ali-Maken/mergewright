# 20h pane-bar-space — the pane bar draws `space` only where the pane answers it; the full sweep back

Status: draft
Blocked by: 20g (merged, partial). **Blocks 20f** (its pane-bar re-shoots and hint sweeps must see this bar).
Crates: graft-ui (hints) + graft (the context's builder, the sweeps, `docs/shots/`).
Build with: Opus — the sweep's per-form states are intricate (20e/20g were Opus); one field, one filter, but every exhaustive match and sweep must follow.
Needs: none.
Scout: none. 20g's NOTES name the seams (`20-blame-place.md:480-487`; 20e's finding `:417-420`).
Ruling: planner, 2026-09-27, KNOWN_GAPS M7/spacefold.

## Ruling — fix the bar, not the handler

**What the bar draws today.** With the detail pane focused on a commit's file row, the bar reads `space fold`; `space`
there does nothing (`crates/graft/src/update/detail/keys.rs:282-288` toggles a directory only, per 20e's NOTES
`20-blame-place.md:417-418`). The sweep caught it: 210 hits, all `space fold` on `PaneFocused { opens: true,
blamable: true }`, every row kind (`20-blame-place.md:483-484`).

**Where the words likely come from** (MAP, not source — confirm the line before editing): `space fold` is the graph's
merge fold (keymap §2.3:221). The graph's row contexts take it from `TAIL` (graft-ui MAP.md:23: "`WorkingTreeRow` takes
no `y`, `x` or `space` from it"), and `PaneFocused` filters out only `S`, `J`, `M`, `O` and `⏎` (same line). So the
graph's `space` leaks into the pane's bar, where the pane has the key. If the builder finds it comes from elsewhere,
the rule below still holds; filter it where it enters, and say where in NOTES.

**Decision:**
1. `PaneFocused` drops the graph's merge-fold hint (`space` / `fold`) from the row it wraps: with a pane focused the
   graph's `space` is not live. A `space` the row's own list gives for the pane (the changes pane's staging) is kept.
2. `PaneFocused` gains `folds: bool` — the files list has focus **and** the cursor is on a directory row. Then the bar
   offers `space fold` (same key, same verb, same meaning as the graph's fold: fold / unfold). Placed after `⏎` and
   `B` when present.
3. The handler is unchanged. A file row of a commit's file list has nothing to stage and nothing to fold; `space` stays
   unbound there and is not drawn.

**Why not the handler:**
- Nothing in the keymap gives `space` a meaning on a commit's file row (§2.2:186 is generic; §2.4 is staging; §2.5 has
  none). Inventing one (fold the parent directory, show/hide the diff) is a new behaviour with no journey behind it.
- The bar is "what the cursor does right now" (graft-ui MAP.md:20). A hint for a dead key breaks H6, "keys valid in
  the focused pane" (`docs/research/ux-principles.md:56`, checklist `:125`).
- The directory fold is a working key that would otherwise be advertised nowhere (the m5/24 defect class, graft-ui
  MAP.md:23).

Rejected: the enum `PaneFocus { Diff, Files { on_directory } }` replacing `opens` (removes the impossible
`folds && !opens`, but rewrites every `opens` site; not worth it now). `every_context` never produces
`folds && !opens`; the doc comment says so.

## Keymap amendment (the conductor applies it before the build)
`docs/design/02-keymap.md` §2.2, the `Space` row (:186) becomes:

> | `Space` | Toggle stage where staging applies, else expand/collapse. In a commit's file list: a directory folds / unfolds; a file has nothing to toggle, so `space` is unbound there and not offered. While a pane has the keyboard, the graph's merge fold (§2.3) is not live (Amended 2026-09-27, M7/spacefold) |

## Scope
In:
- graft-ui `hints`: the filter (1), the field (2), `every_context`, `pane_focused_form` (**restore the form 20e dropped**,
  `(true, true)`, plus the `folds` forms), `tests::coverage_slot`, `COVERAGE_SLOTS`.
- graft: where `PaneFocused` is built (grep `PaneFocused {`), setting `folds` from the detail cursor; the sweeps
  (`update/tests.rs` `classified_form`, `screen_for`, `presses_for`, `states_for`, and `update/keys/bound.rs`, per
  `20-blame-place.md:333`) gain a state with the cursor on a directory row.
- Check the code matches the keymap amendment above.

Out: the handler at `update/detail/keys.rs:282` (unchanged); `PaneOverOverlay` (draws no `space`; a directory there
stays unhinted — note it, don't build it); 20f's walks and re-shoots.

## Signatures
```rust
// graft-ui, hints/context.rs
pub enum HintContext {
    // …
    PaneFocused {
        row: Box<HintContext>,
        opens: bool,
        blamable: bool,
        /// The files list has focus and the cursor is on a directory row: `space fold`.
        /// Never true without `opens`; `every_context` does not produce that shape.
        folds: bool,
    },
    // …
}
```
No other signature changes. `hints_for(context, divider, columns) -> Vec<Hint>` keeps its shape.

## Acceptance
- [ ] Red first: the restored form (`pane_focused_form` with `(true, true)`) turns `every_key_the_bar_draws_is_answered`
      red before the fix (quote the hit count), green after. Name the commit of each.
- [ ] `NEVER_HINTED` unchanged (`git diff` on it empty).
- [ ] graft-ui tests: `pane_focused_drops_the_graphs_space_fold` (every graph row context wrapped: no `space` hint) ·
      `pane_focused_keeps_the_changes_panes_space` · `pane_focused_folds_offers_space_fold_after_enter_and_b`.
- [ ] graft tests: `pane_bar_on_a_commits_file_row_draws_no_space` · `pane_bar_on_a_directory_row_offers_space_fold` ·
      `space_on_a_directory_row_folds_it_and_the_bar_still_says_fold` (fold, then unfold, same hint).
- [ ] `every_key_the_bar_draws_is_answered` and `every_context_reaches_every_shape` green over **every** `PaneFocused`
      form, no form left out, no comment excusing one.
- [ ] Capture `docs/shots/m7/pane-bar-file-vs-directory-120x40.txt` (≤ 200 KB): two frames, the detail pane focused on
      a commit, cursor on a file row, then on a directory row. **Right:** the file-row bar has no `space`; the
      directory-row bar reads `… space fold …`; nothing else in the two bars differs except what the row changes.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, test --doc.

## Touches (check each, name how)
- Every `PaneFocused {` constructor and match, both crates (grep; the compiler finds the rest).
- graft-ui hint tests and `COVERAGE_SLOTS` (79 → the new count; say it).
- Captures: any bar drawn with a pane focused on a graph row loses `space fold`. grep `space fold` in `docs/shots/`
  finds 3 files, all m6 (`m6/m6-contact-sheet-3.txt`, `m6/fix-c-live-frames-187x50.txt`,
  `m6/m6-contact-sheet-captions.tsv`). Say for each whether its bar is a pane-focused one. Past milestones' sheets are
  a record: don't edit them; list the stale ones in NOTES. M7 captures that change are 20f's re-shoots.
- Keymap: §2.2 `Space` (amendment above). §2.3:221 unchanged.
- The mouse (§2.15): a click on a directory row — say whether it does what `space` does; don't change it.
- Journal/undo: none (no write).

## Size
Small–medium (~35k).
