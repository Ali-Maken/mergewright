# Review: M5 group 03 — hint-bar-redesign

Reviewed at merge `14f404c` (group commit `ee855e4`), read-only. Ticket:
`.scratch/m5-history-and-ghost-preview/issues/03-hint-bar-redesign.md`.
Standard judged against: ADR-0037 §4, the user's words — *three most
relevant keys for the current context* at 80 and 120, *never all hidden*,
*a feature nobody can see doesn't exist*.

**Not run here (conductor's instruction — a builder holds a worktree):** no
cargo command. Gates as reported by the conductor at the merge: fmt ok ·
clippy ok · 1904/1904 nextest, 29 skipped · doctests vacuous (0 doctests
per crate). Heavy tier, release perf, fuzz, bench: not run this review.
Everything below was settled by reading source; where a runtime check
would settle it better, the command is named.

Counts: **2 Critical · 6 Major · 10 Minor.**

---

## Critical

### C1. The changes pane never gets its own hints — `hint_context` tests `on_working_tree` first

`crates/graft/src/view/rows.rs:238-249`:

```rust
if state.on_working_tree {
    return HintContext::WorkingTreeRow;
}
if state.on_changes_pane() {
```

`on_changes_pane()` is `detail.shown && detail.spec == WorkingTree`
(`update/state.rs:345`). The pane opens on the Uncommitted-changes row and
the cursor stays there: `selected_spec()` yields `WorkingTree` *because*
`on_working_tree` is true (`update/selection.rs:148`), and the canonical
fixture `view/tests.rs:2030 changes_pane_state` sets both
`on_working_tree: true` and `spec = WorkingTree`. So whenever the second
branch could be true, the first has already returned. `ChangesPane` is
unreachable in the shipped app.

Effect: with the changes pane open the footer reads
` ⏎ changes  c commit  S stash  . more` — it tells the user to open the pane
they are in, and `space stage` / `d discard` (m3-working-tree-undo-and-stash/04,
"the write keys are hinted on the pane that can write") are gone. The
unmerged variant (`M mergetool  e edit  space resolved`) is unreachable too.
This is the one daily context where "the three most relevant keys for the
current context" changes most, and it is the one that does not change.

Why nothing caught it: the old graft-ui test
`the_write_hints_appear_only_on_the_changes_pane` was deleted; its
replacement `the_changes_pane_hints_replace_the_row_context` sets
`hint_context` by hand. No test drives `hint_context(state, rows)` from an
`AppState` (grep: `HintContext` appears in `crates/graft` only in `rows.rs`
and the bound-keys test). Scope item 4 is marked done under `Status: done`.

Fix: test `on_changes_pane()` first (the pane is the more specific state),
and add an app-side test: `changes_pane_state(120, 40)` → `graph_view` →
`hint_context == ChangesPane { unmerged: false, detail_shown: true }`, plus
the same for a branch-badge cursor row and a marked row. Runtime check that
settles it today: `cargo test -p graft view::tests::capture_m3_changes_pane
-- --ignored --nocapture` and read the last row of
`docs/shots/m3-changes-pane-120x40.txt`.

Note (pre-existing, not verified here): at <100 columns the `Tab` layout
puts the pane over the whole body and the M3 80x24 capture's last row is a
diff line, so the graph footer may not draw at all on the changes pane at
80. If so, "never all hidden" fails there for a different reason; belongs
to the acceptance group to check.

### C2. Dead and mis-bound keys are advertised at the user's daily width

At 80 columns on a branch row the bar is ` m merge  r rebase  ⏎ open  . more`;
on a plain row ` ⏎ open  b branch  r rebase  . more`. `m`, `r`, `b`, `t` are
bound nowhere (`update/keys.rs` grep: no `Char('m')`, `Char('b')`,
`Char('t')`; `Char('r')` only with Ctrl). Two of the three keys shown on
every commit row do nothing. That is ADR-0037 §4 inverted: the bar is how
anything is discoverable, and it now teaches keys that do not exist.

Also hinted and not bound as worded, in `ChangesPane { unmerged: true }`
and `Banner` (`hints.rs:224-242`):

- `M mergetool` — no `mergetool` anywhere in `crates/graft/src/update/`.
  `M` is the Map toggle, and only while `detail.focus == Graph`
  (`update/map.rs:145`). The guard test passes it because `bound_keys()`
  lists `"M"` and the test compares key strings, not `(key, verb)`.
- `space resolved` — `Space` on an unmerged entry is *refused* with a sticky
  (`update/operate.rs:168-172`, `conflict_refusal("stage")`). Passes the
  guard because `"space"` is bound — to stage.

The builder disclosed `m/r/b/t` and `s/a` in NOTES and in the test's
`pending` list; `M mergetool` and `space resolved` are not disclosed. The
ticket gave two instructions that conflict (item 1 "the note's table
verbatim"; item 6 "every key hinted is … bound in code"); the builder chose
the first. The user's outcome is still dead hints in `main`. Scope item 6
has no acceptance tick, so no gate could fail on it.

Fix, one of: (a) hold the hint for a verb until its group binds it — e.g.
`hints_for` filters the table through a `bound: &dyn Fn(&Hint) -> bool` the
app supplies from the real dispatch, or `view::rows` strips unbound hints
and lets the list refill from the right so the count stays 3/6/all; (b)
merge this group only together with the groups that bind `m r b t`
(07/08). Either way make the guard compare `(key, verb)` against a table
derived from the dispatch functions, not a hand-typed string list, and drop
the context-agnostic `pending` carve-out.

---

## Major

### M1. `. more` opens a menu that lists none of the dropped keys

`hints.rs:33` `MORE = hint(".", "more")`; `.` opens the context menu
(`update/keys.rs:180`, `context_menu::open_selected`). `ContextMenu::for_row`
(`crates/graft-ui/src/context_menu.rs:54-70`) lists Copy SHA, Compare with
this, Open details (+ Explain when marked); `for_working_tree` lists Open
details alone. None of the hints the width rule dropped at 80 (`p pull`,
`P push`, `f fetch`, `u undo`, `S stash`, `J journal`, `/ search`, `j/k move`)
are behind it. "more" is a promise the key does not keep, so at 80 columns
the dropped keys are reachable by no visible path — the design note's own
label, but the user's outcome. Fix: the menu lists the dropped hints as
items (dispatching the same actions), or the verb changes to what `.` does
(`. actions`) and ADR-0037's "drop the rest" is accepted with eyes open.

### M2. Reachable footer states with no keys at all

`footer.rs:57-91` precedence is unchanged (ticket item 2), so these still
blank every key:

- `Status::Failed(_)` and every `Status::Sticky { keys: None }` — the
  common case (hook rejected the commit, stage refused on a conflict,
  `$EDITOR` failed). It sits until `Esc`, and the bar names no key, not even
  `esc dismiss`.
- `notice` (`Busy`/`Done`/`Copied`), `compare_footer`, `reveal` — transient,
  arguably fine, but each is a frame with zero keys.

Tick (a) `no_context_hides_every_key` proves `hints_for` is non-empty; it
does not prove the footer is never keyless. Fix the sticky case cheaply:
`view::status::graph_problem` supplies `keys: Some("esc dismiss")` when the
status carries none, so `fit_status_line` keeps it whole. Leave the
transient slots, but say so in TESTING.md.

### M3. About a dozen working keys are now hinted at no width

Dropped from the bar entirely (they were visible ≥144 before): `q quit`,
`F2 status`, `F3 renderer`, `+ expand/collapse lanes`, `Space fold`,
`1 first-parent`, `F focus`, `H hide stash`, `x compare`, `M map`,
`O overview`, `Ctrl+R refresh`, `PgUp/PgDn`, `g/G`, `e`, `i`, `a amend`.
`?` help does not exist until M8, and `. more` does not list them (M1).
"all at 160+" means the table's all, not the bound set — at 160 the branch
row shows 12 hints and 22 cells of blank rail. The ADR asked to *drop or
abbreviate* at narrow widths, not to make features invisible at every
width. The `+` fold hint is the builder's own named deviation; the others
are the table's silence. Fix: a shared tail (the old list's leftovers, in
the old order) appended after the context list in the ≥160 tier, or listed
under `.`. Keymap §2.3 names all of them, so this is not a key allocation.

### M4. `<> resize` lost on a commit's detail pane; `GraphView::detail_shown` is now write-only

Only `ChangesPane` adds `<> resize` (`hints.rs:253`). The old footer added
it whenever `detail_shown` (ADR-0028 / `detail-resize-and-persistence`);
a commit's detail pane has the same divider and lost its hint. In graft-ui
nothing reads `GraphView::detail_shown` any more (`graph_screen/mod.rs:134`
is the only non-test hit) — `view::rows` copies it into the `ChangesPane`
variant and the field itself is dead. Fix: a `CommitDetail { detail_shown }`
context (or `detail_shown` on `BranchRow`/`PlainRow`), and delete the dead
field or use it.

### M5. The seam is untested and the captures do not exercise it

Scope item 4 (`view/rows.rs` builds `hint_context` from `AppState`) has no
test at all — C1 is the direct result. The 12 captures are `TestBackend`
frames from `crates/graft-ui` with `hint_context` set by hand
(`graph_screen/tests.rs capture_hint_bar_for_docs_shots`), not "the
existing pty pipeline on a corpus shape" the ticket asked for; the capture
headers say so, the NOTES do not. They prove the footer draws a given
context correctly; they do not show the app picking that context for a
branch row, a marked row or the changes pane. Fix: the app-side test named
under C1, and regenerate at least the four 80x24 captures from an
`AppState` fixture so the contact sheet shows the real path.

### M6. The dead-key guard cannot close what it claims

`update/tests.rs:3164 every_hinted_key_is_bound_or_a_named_pending_group`:

- `bound_keys()` (`update/keys.rs:620`) is a hand-typed string list with no
  test that its entries are true; `"M"` and `"space"` are "bound" while the
  hinted verbs are not (C2).
- Compares `hint.key` only, never the verb.
- `pending` is context-agnostic: `a amend` or `s stage` hinted on any row
  context would pass.
- `update::mod` made `keys` `pub mod` for one function the in-crate test
  could reach at `pub(super)`.

Fix: derive the bound set from the dispatchers (a `const` table each key
function is written against), compare `(key, verb)`, scope `pending` per
context, and give scope item 6 an acceptance tick that names this test.

---

## Minor

1. `hints.rs:154-156` `unreachable!("handled in hints_for")` in non-test
   code (PROMPT.md §6 bans panicking paths outside tests). Restructure so
   `context_list` cannot be called with `Marked`/`Preview` — one `match` in
   `hints_for`, or a `RowContext` sub-enum the two wrap.
2. `fit_status_line` (`hints.rs:288-290`): when ` · keys` alone does not fit
   it draws `fit(" · keys")`, spending three cells on a separator with no
   reason in front of it. Drop the separator in that branch.
3. `split_reason_and_keys` (`update/remote.rs:580`) splits on the first
   ` · `; `pull refused: would overwrite X (uncommitted) · commit or stash
   first` yields `keys = "commit or stash first"` (not keys — harmless), and
   a path containing ` · ` would mis-split. The right fix is `ops` returning
   the pair; note it at the `graft-ops` seam rather than parse.
4. `ChangesPane.unmerged` is "any entry unmerged" (`rows.rs:243`), so while
   one conflict exists every clean file's row also loses `space stage` /
   `d discard`. The note says "unmerged *file*". Use the cursor entry.
5. `hints.rs:7-8` doc comment cites `docs/design/02-keymap.md` for the old
   MAP entry — it is `crates/graft-ui/MAP.md`. And "the design note's
   priority table, verbatim" is untrue for `Journal`/`Stash`
   (`hints.rs:152-153`): those lists are the builder's (consistent with
   keymap §2.9/§2.11, absent from the note). Say so.
6. Below ~36 columns `fit_list` returns 3 + more regardless
   (`hints.rs:128` `count <= floor`) and `render.rs` `fit()` then cuts a
   hint in half — "never a hint cut in half" holds only from ~36; the test
   range starts at 40. Either document the floor or drop `. more` first.
7. `opening_view` (`rows.rs:110`) hints ` ⏎ open  b branch  r rebase  . more`
   before any history exists.
8. Tick (f) tests `keys = "p pull · P force"`, not the real
   `"p pull first · P force with lease"`. The real rejection line is 72
   cells and never needs cutting at 80 (the capture confirms), so the
   mechanism is only exercised in practice by the stale-lease line
   (`push refused: origin/main moved since your last fetch (stale lease) · f
   to fetch, then look`, ≈86 cells) and the auth line — neither drawn at 80
   in any test. Add one frame test on the stale-lease line.
9. `credential_by_key.rs:221` `keys.as_deref().unwrap_or_default()` — in a
   test and followed by an assert, so loud; still the pattern CLAUDE.md
   rule 3 names. `expect("the auth line carries keys")` says what it means.
10. KNOWN_GAPS rows the NOTES call 493/495 are at 499/501 now; row 501's
    journal half (`not undoa…`) stays open, as the NOTES say. Conductor to
    move 499 and half of 501 to `docs/gaps/closed.md`.

---

## Acceptance tick spot-check

13 ticks checked (every tick in the NOTES). All 13 name a test or artifact
that exists. **11 hold against the requirement; 2 hold against the code
only:**

- (a) `no_context_hides_every_key` — true of `hints_for`; the footer as
  drawn can be keyless (M2).
- "12 captures … each footer matches the note's mockup" — the footers
  match; the contexts were set by hand, so the captures do not show the
  app choosing them, and for the changes pane it does not (C1, M5).

Held: (b) (c) (d) (e) (g) exact; (f) on mechanism (Minor 8); (h) —
`push_lease_refused_stale` exists (`m4_walk/push.rs:239`), the capture's
last row is the full 72-cell line, the NOTES say honestly it needed no
change; grep tick (only `hints.rs:8`, a comment); KNOWN_GAPS tick (honest
partial); MAP tick; scout line.

Not a tick, but the requirement that failed: scope item 6 ("every key
hinted is … bound in code") has no acceptance box at all.

## Conductor's specific questions

- **Context-sensitive or fixed list?** Genuinely context-sensitive across
  five contexts (working-tree row, branch row, current branch, plain row,
  marked); each context's list is fixed, which is what the note specified.
  The sixth context (changes pane) is unreachable (C1).
- **Any keyless state?** Yes — every keyless `Failed`/`Sticky`, plus the
  transient slots (M2). Below ~36 columns hints are cut (Minor 6).
- **Sticky keys preserved at 80 on the rejection line?** The logic is
  right (`fit_status_line` keeps ` · keys` whole, cuts reason with `…`,
  `footer.rs:67` passes `width - 1` for the leading space). The real
  rejection line fits at 80 anyway; the stale-lease and auth lines are
  where it bites, untested at 80 (Minor 8).
- **`bound_keys()` exclusions and the dropped `+` hint.** Excluding `m r b t
  s a` from `bound_keys()` is correct — they are not bound. Hinting them
  anyway is the problem (C2). Dropping `+` is a real loss with no
  compensating path (M3); the keymap names `+`, so hinting it needs no
  allocation.
- **Does the guard test close the dead-key hole?** No (M6): key-only match,
  hand-typed bound list, context-free carve-out. `M mergetool` and
  `space resolved` pass it today while being dead as worded.
- **Compaction.** The NOTES' "conductor resumed and finished in the same
  worktree" is not what happened per the conductor. Signs consistent with a
  resume-from-summary in the second half: the untested seam and its
  ordering bug (C1, M5), the captures built the quick way rather than as
  the ticket specified and the deviation unlisted (M5), the write-only
  `detail_shown` (M4), the mis-cited doc comment (Minor 5). The first half
  (`hints.rs`, its tests, `Status::Sticky`) is clean.

## Architecture and readability

Crate rules hold: `hints.rs` is pure (`unicode_width`, `crate::fit`),
graft-ui has no new deps, graft reads graft-ui types only. `hints.rs` is
299 lines, plainly written, well commented; `fit_list`'s loop is clear.
`Status::sticky()` constructor is the right shape for the reshaped variant
and every site compiles through it. `view::status::graph_problem` beside
`problem` (two readings rather than a split field) is the right call and
the doc says why. `pub mod keys` for one function is wider than needed
(M6). ARCHITECTURE.md unaffected.
