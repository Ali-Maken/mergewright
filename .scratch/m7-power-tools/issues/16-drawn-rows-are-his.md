# drawn-rows-are-his — stash internals leave the graph; stash commits lose `evil merge: unchecked`; the force box names what leaves

Status: ready
Blocked by: none
Crates: graft (the history walk's inclusion, the radar's `EvilUnchecked`, the push box), graft-ui (the detail header,
the push box rows), graft-ops (the read of the commits that leave), graft-graph only if the scout finds the stash
parents are dropped in layout rather than in the walk.
Source: note "Pass 2a" R7. The rows, quoted:
- **:858** "Stash internals are drawn in the history: `index on main:` and `untracked files on main:` rows under
  `stash@{0}`, an old commit beside its rebased copy — field notes :38, 'these are in my history twice'."
- **:839** "`evil merge: unchecked` on every stash commit's detail header: the radar adds `EvilUnchecked` for any
  commit with 2+ parents, and a stash always has them."
- **:859** "The force box names a SHA, not what leaves the remote: `origin/main cda3fd7 → 9304b58` / `rewrites history
  for everyone who has cda3fd7` would drop a teammate's commit without naming it (H5)."

Build with: Opus. A history-inclusion change with layout and perf knock-ons.
Scout: `scout` first (ADR-0035).
Needs: none.

## Decided
- **:858:**
  - A stash's 2nd and 3rd parents (the index and untracked commits) are **never** drawn.
  - A commit reachable **only** through a stash (e.g. the pre-rebase base his stash was made on) is drawn as `◌`,
    muted, with the badge `only in stash@{n}`. It is not drawn as a normal row.
  - `H` (hide stash) hides these too.
  - Distinction (UX §3.6): `◌` + the words vs `○`/`●` for his commits.
- **:839:** `EvilUnchecked` is never added for a stash commit. The detail header shows no evil-merge line on one.
- **:859:** the force box lists what leaves the remote. It reads `rev-list <new>..<remote-old>`, capped at 3 rows, then
  `+N more`:

  ```
  leaves origin/main:  cda3fd7  Fix typo            Sara  2h
                       b1ade02  Log session ids      Sara  4h
  ```

  The old `rewrites history for everyone who has cda3fd7` line stays, below it. Nothing leaves → `nothing on origin/main
  is lost`.

## Read first
- Note "Pass 2a" R7; field notes :38.
- `crates/graft/MAP.md`: history (`History`, inclusion, `Included`), `view::radar`, `update::remote::push_box`.
  `crates/graft-graph/MAP.md`: inclusion/stash handling if any. `crates/graft-ops/MAP.md`: `history_reads`
  (`commits_between`). `crates/graft-ui/MAP.md`: `detail_pane::header`, the push box.
- `docs/design/02-keymap.md` §2.3 `H`.

## Scout brief (≤ 1 page)
1. Where stash commits enter the walk (refs/stash + reflog?), and where their parents are followed. The function and
   line, in graft or graft-graph.
2. How `H` hides the stash today.
3. The radar's `EvilUnchecked` rule site, and how a stash commit is recognisable there.
4. The push box's rows and its data (what's already read about the remote's old tip).
5. Perf: does excluding parents change `Included`'s re-read (KNOWN_GAPS "inclusion change re-reads history")? Name the
   cost path.

## Signatures
```rust
// graft-ops: reuse history_reads::commits_between(repo, git, new, remote_old, stop) (the scout confirms its shape)
// no new pub API expected; name changed fns in NOTES
```

## Acceptance
- [ ] `stash_index_and_untracked_commits_are_not_drawn` (red first, :858).
- [ ] `a_commit_only_a_stash_reaches_is_drawn_as_only_in_stash`.
- [ ] `h_hides_the_only_in_stash_rows_too`.
- [ ] `a_stash_commit_has_no_evil_merge_line` (red first, :839).
- [ ] `the_force_box_lists_what_leaves` (red first, :859); `nothing_leaving_says_so`; `more_than_three_is_cut_with_more`.
- [ ] **Captures:**
  - `docs/shots/m7/stash-in-history-120x40.txt`. **Right:** no `index on` / `untracked files on` rows; one `◌` row
    `only in stash@{0}`.
  - `docs/shots/m7/force-box-leaves-120x40.txt`. **Right:** `leaves origin/main:` with the subject and author of each
    commit.

  Colour unverified. Every changed older capture is re-taken and listed in NOTES.
- [ ] Release perf: the frames tests are unchanged within noise. The builder runs the perf script `slow-key-cause`
      adds if it has landed, else `scripts/m4-review-perf.sh`, into `docs/perf/drawn-rows-are-his.txt`, and reads only
      the summary lines.
- [ ] MAP.md for the crates touched. TESTING.md rows.

## Touches (check each, report how)
- **Graph layout and perf** (an inclusion change): checked by the perf run above.
- **The stash overlay (`S`), stash badges, `H`:** unchanged except these rows. Check: the stash tests and captures.
- **Mistake Radar counts:** one finding fewer per stash. Check: the radar tests.
- **The push flow (`P`, force with lease):** unchanged behaviour. Check: the m4 push tests.
- **Journal/undo, keymap:** none.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~55k).
