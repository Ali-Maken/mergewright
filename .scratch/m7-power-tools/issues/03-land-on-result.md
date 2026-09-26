# land-on-result — every write that moves HEAD lands the cursor on the new HEAD; a fast-forward pull sets edits aside

Status: ready
Blocked by: none
Crates: graft (`update/operate.rs` `operated`, `banner/stop.rs` `land_on_head`); graft-ops (`sets_aside` for a
fast-forward `Pull`).
Source: KNOWN_GAPS :773 ("Only `--continue` lands the cursor on the new commit": `operate.rs` sets `Done` and
refreshes; the head landing is set only by a finished continue in `banner/stop.rs`; after `m m` on `feature` the cursor
stays on `feature`'s row; UX §3.1). KNOWN_GAPS :805 ("A fast-forward pull over overlapping unsaved edits still fails
with git's words; set-aside covers rebase and checkout only", ADR-0045 :97). Note "Pass 2a" R7, "Where I'd differ" 4.
Build with: Opus. One landing rule applied across kinds, and an ADR-0045 extension with a clash path.
Scout: `scout` first (ADR-0035).
Needs: none. `rewrite-stops` and `branch-picker` need it.

## Decided
- **The landing rule** (UX §3.1): after a **finished** write, the cursor lands on the new HEAD's row, whatever row it
  was on. The detail pane follows as it does for `--continue` today. Kinds: Commit, Amend, Merge, Rebase, Rewrite,
  CherryPick, Revert, Pull, Checkout, Reset, BranchFromStash.
  - Not undo/redo (the journal's own landing, G-list), and not Fetch/Push/Stash*/Branch*/Tag* (HEAD doesn't move).
- **The same function lands both paths:** `land_on_result(state, kind, new_head)` replaces `land_on_head`'s
  continue-only use.
- **Wait for the new HEAD's row:** if the row isn't laid out yet, use the existing `head_landing` pending mechanism.
  Never block a frame.
- **A fast-forward pull over unsaved edits git would refuse:**
  - It is wrapped in the ADR-0045 §4 set-aside (stash create → store → reset --hard → `pull --ff-only` → stash apply →
    drop), one entry.
  - The preview line: `your edits: N files set aside, put back after`.
  - A clash lands on the resolver (ADR-0045).
  - Edits git doesn't refuse over (they don't touch the pulled paths) are left alone, as today.
- **Stop:** words and a status line are unchanged, except the cursor position.

## Read first
- Note "Pass 2a" R7 rows :773 and :805; "Where I'd differ" 4.
- `docs/research/ux-principles.md` §3.1.
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §4, §8.
- `crates/graft/MAP.md`: `update::operate` (`operated`, `Operated`), `update::banner` `stop.rs` (`land_on_head`,
  `head_landing`), `update::remote` (the pull path). `crates/graft-ops/MAP.md`: `operation` (`sets_aside`), the
  set-aside wrap.

## Scout brief (≤ 1 page)
1. `operated`'s `Done` arm per kind, and where the cursor is left today (the line).
2. `land_on_head` / `head_landing`: how a continue lands, and how it waits for the row.
3. The pull path: is the pull previewed (with `y`) or launched directly? Where would a set-aside line be shown?
4. `sets_aside` and the wrap: which kinds, and how `Pull` differs (a remote spawn, `in_flight`).
5. Every test and capture that asserts the cursor position after a finished write (`grep -rn` over `m3_walk`,
   `m5_walk`, `m6_*`; captures whose `▸` row would move).

## Signatures
```rust
// graft
pub fn land_on_result(state: &mut AppState, kind: OperationKind, new_head: Oid);
// graft-ops: Operation::sets_aside() true for Pull when fast-forward and the pulled paths overlap unsaved edits
```

## Acceptance
- [ ] **App:**
  - `after_m_m_the_cursor_is_on_the_merge` (red first, :773);
  - `after_a_commit_the_cursor_is_on_it`;
  - `after_a_cherry_pick_the_cursor_is_on_the_new_commit`;
  - `after_a_checkout_the_cursor_is_on_the_new_head`;
  - `landing_waits_for_an_unlaid_row`;
  - `undo_keeps_its_own_landing`.
- [ ] **graft-ops:**
  - `a_ff_pull_over_overlapping_edits_sets_them_aside` (red first, :805);
  - `a_ff_pull_whose_put_back_clashes_stops_as_a_clash`;
  - `a_ff_pull_over_unrelated_edits_is_unchanged`;
  - `the_ff_pull_entry_is_one_entry_and_u_restores_edits`.
- [ ] **Walks:**
  - `pull_ff_over_edits` (m6/m7 harness): `p`, then `y` if previewed. The edits are back, and HEAD is fast-forwarded.
  - Each existing walk that asserted the old cursor row is updated. NOTES lists each walk with its before/after row.
- [ ] **Captures:** every changed capture is re-taken (`GRAFT_UPDATE_SHOTS=1`, then a second pass without it, all
      `settled`). NOTES lists each, with "cursor row only" or what else changed. New:
      `docs/shots/m7/landed-after-merge-120x40.txt`. **Right:** `▸` on the merge commit's row; the status line
      unchanged from today's words.
- [ ] MAP.md (graft, graft-ops). TESTING.md rows for the new walk and capture.

## Touches (check each, report how)
- **Every finished-write landing across M3–M6:** scout Q5. Each walk and capture is listed; changed ones re-taken.
- **The detail pane following the cursor:** check `the_detail_pane_follows_the_landing` (add).
- **`--continue` landing:** now the same function. Check: the m6 continue tests green.
- **Set-aside (ADR-0045):** a new kind. Check: all stash-clash/set-aside tests green.
- **The pull/push `in_flight` state machine:** Check: the m4/m6 pull tests green.
- **Journal/undo:** the ff-pull entry records the put-back like rebase's. Check: the undo test above.
- **Keymap:** none. **ADR-0045:** amendment rows owed (the conductor): §4 kinds gain the ff Pull.

## Rules / Gates
As rewrite-engine.

## Size
Medium-large (~60k). Split if needed:
- **-a:** landing;
- **-b:** the ff-pull set-aside.
