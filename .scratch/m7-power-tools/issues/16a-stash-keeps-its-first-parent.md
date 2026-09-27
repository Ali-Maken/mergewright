# stash-keeps-its-first-parent — in the laid-out history a stash commit follows only its first parent, so its index and untracked commits are never drawn

Status: draft
Blocked by: none
Crates: **graft-graph only** (`inclusion`). Disjoint from graft: runs beside the serial graft chain. graft must
compile **unchanged** against it (16 wires it in).
Source: note "Pass 2a" R7, gap row :858. Respecified from scout `.scratch/m7-power-tools/scouts/16.md` §1, §2, §5:
parents are followed in **graft-graph** (`inclusion.rs:143` `Frontier::decide`, `kept_parents` `:181`, which keeps all
parents under `AllParents`); nothing drops a stash's; graft-graph knows no stash except the Oids handed in
(`Inclusion::hidden_stashes`, `inclusion.rs:55`).
Build with: Opus. An inclusion rule inside the frontier that decides what is drawn; a wrong placement shows as wrong
rows only on real histories, and `reinclude`'s replay must stay consistent with it.
Scout: done (16). No second scout.
Needs: none.

## Decided
- `Inclusion` learns the stash entries' own commits (every entry graft knows, `stash@{0..}`), through a setter, so
  graft's existing construction compiles unchanged (don't add a pub field).
- **Rule:** a commit in that set keeps its **first parent only**. Its 2nd and 3rd parents (index, untracked) go to the
  frontier a hidden stash's parents go to today (`decide`, `:153`), so a commit reachable only through those edges
  is not shown — the same mechanism as `H`, applied to two edges instead of the whole stash.
- The first parent is untouched: a commit reachable only through a stash's **first** parent (a rebased-away base) is
  still shown. Marking it is 16's job (from `graft_git::history_reads::only_in_stash`, 14a), not the layout's.
- `hidden_stashes` (`H`) is unchanged: a hidden stash hides as today, first parent included.
- `Inclusion::changed_from` counts a change in the stash set, so `reinclude` replays from the first row it changes
  (ADR-0026: by handle; look the stash set up **by handle per link, not by `Oid`**, converting once when the inclusion
  is set; if `hidden_stashes` is looked up by `Oid` per link today, follow its shape and say so in NOTES).
- `Follow::FirstParent`: unchanged (already first parent only).

## Read first
- Scout 16 §1, §2, §5.
- `crates/graft-graph/MAP.md` `inclusion`, `layout::source`, `layout::stream` (`truncate`, `Invalidated`).
- ADR-0026 (handles, not ids). Source after the MAP: `graft-graph/src/inclusion.rs` (`:20-36` `reinclude`, `:55`,
  `:65` `changed_from`, `:143-181`).

## Signatures
```rust
impl Inclusion {
    /// Every stash entry's own commit. Each keeps only its first parent in the laid-out history.
    pub fn with_stashes(self, stashes: BTreeSet<Oid>) -> Self;
    pub fn stashes(&self) -> &BTreeSet<Oid>;
}
```
`Inclusion::default()` has an empty set: no behaviour change until graft calls `with_stashes` (16).

## Acceptance (each box names its test at tick time)
- [ ] `a_stash_keeps_only_its_first_parent` (red first) — `stash` with staged changes: the index commit is not
      shown; the stash and its first parent are.
- [ ] `a_stash_with_untracked_drops_both_extra_parents` (red first) — `stash -u`: neither the index nor the untracked
      root is shown, and no lane is left open for them (the row after the stash has no dangling lane).
- [ ] `a_rebased_away_base_is_still_shown` — the stash's first parent off every branch stays a row.
- [ ] `hiding_a_stash_still_hides_its_first_parent_line` — `hidden_stashes` + `with_stashes` together = today's `H`.
- [ ] `changing_the_stash_set_reincludes_from_the_first_changed_row` — `reinclude`'s `Invalidated.from_row` is the
      stash's row, not 0.
- [ ] `an_empty_stash_set_changes_nothing` — the existing inclusion and layout tests green, untouched.
- [ ] `crates/graft-graph/MAP.md` `inclusion`: the two fns and the rule, one line each.

## Touches (check each, report how)
- **Layout fingerprints / chunk rebuilds:** shapes change only on stash rows and below where lanes closed. Check:
  `a_rebuild_after_truncate_is_checked` and the stream tests green.
- **Ghost Preview** lays out a slice with `graft_graph::layout` (`update::preview`): default inclusion there, so no
  change. Check: grep `Inclusion` outside graft-graph, list every hit and whether it now needs `with_stashes`
  (16 does the wiring; this ticket only lists).
- **graft compiles unchanged.** Check: workspace gates with no edit outside graft-graph.
- **Perf:** one set lookup per link. Check: the graft-graph layout tests' timing unchanged (no new test here); the
  release frames run is 16's, after wiring.

## Rules / Gates
As rewrite-engine.

## Size
Small-medium (~30k).
