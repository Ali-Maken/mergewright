# Scout 14d — fork follows safety (branch m8-09b @ 4ce84e29)

## forks.rs (`crates/graft-ops/src/preview/forks.rs`, 467 lines)
- Loop `forks()` :127-145: skips `fates.claimed` (:129-131), `after_row` (:133), `fate()` (:134), and only on
  `Follows` calls `fates.follow()` (:135-137), which claims carried branches. So carried branches are discovered
  **after** the fate is decided — a §6.1 gate needs the `inside` set (:231-237: other unclaimed candidates whose tip
  is in F's own commits) computed before/inside `fate()`.
- `inside` holds only **candidates** (`candidates_of` :162; backups with 0 commits at a tip excluded per its doc :159-161).
  F's tip itself is not in `inside` (filter `*other != index`).
- `Weigh::read` :288-316: reads `default` (`rewrite::default_branch_of`, :308) and `upstreams` (one `for-each-ref
  %(refname)%00%(upstream)`, :427-448) for candidates only. No `refs/remotes/*` scan exists.
- `after_row` :323-331: `row_of[at]`, then last row of its `squash_groups` group (kept or melded) — already "last
  row of group". `None` when `at` is not a row (→ `NotChecked`, :357-362).
- `fate()` order :334-375: DefaultBranch :343 → Elsewhere (`off.elsewhere`) :346 → HasMerge :351-354 → after_row None
  → Pushed :364-370 → Nested :371-373 → `replayed_fate` (forecast) :374/:381-424. All gates already precede the
  forecast; a §6.3 base check plugs in before :374.
- Pushed: `holds_any(repo, upstream, off.at, commits)` (`rewrite.rs`:359-385): `rev-list --ignore-missing <up> ^at`,
  true if any listed oid is in F's own commits. `^at` is why `at` and below are blind (review M2).
- `elsewhere` comes from `rewrite/dependents.rs` :175, :204-211 (for-each-ref `%(worktreepath)` :327, minus this one).

## effects.rs
- `WhyStays` :210ff; `Elsewhere` doc :232-233 says "git would refuse to move under it" — contradicts ADR §6.1.
- `words()` :258ff — the only exhaustive match in non-test code.
- `rewrite/dependents.rs` :28, :65-67 mention refuse — :67 is Graft's own refusal; :28/:43 worktree check, no "git refuses" claim seen.

## graft (app)
- `line_rows.rs`:129 matches `WouldConflict` → `!`/Warning; :133 catch-all `Stays(why)` → `muted`. New variants
  land muted **without code change**. No exhaustive `WhyStays` match in graft (grep `WhyStays::` in crates/graft:
  line_rows.rs:129, tests.rs:1889/2007/2048, rewrite_editor/right/tests.rs:40/96 only).

## Tests / helpers
- `graft-ops/tests/it/fork_follows.rs`: `forked()` :56, `reword_c1` :107, `amend_into` :94, `the_fate` :459,
  `stays` :157, `heads` :41, `own_diff` :50. Remote: `a_pushed_fork_stays` :465-484 uses config `remote.origin.url "."`
  + `update-ref refs/remotes/origin/...` — **no bare remote helper**. Worktree: `a_fork_checked_out_elsewhere_stays`
  :486-497 (`worktree add -q linked feature-x`; asserts fate only, no post-run status). Default: :534-543.
- graft walks: `m7_walk/fork_rows.rs` `two_forks_repo` :27, three walks :75, :104, :138.

## Contradictions with ticket
1. Ticket "graft only where it matches `WhyStays` exhaustively" — none exists; catch-all at line_rows.rs:133.
2. Ticket line refs (:128-131, :231-248, :323-331, :363-370) all match the code.
3. `after_row` already ends at the group's last row (:325-330); ticket lists it as scope ("the `after_row` rule") — could not confirm what changes beyond the base check.
4. Ticket asks for "bare remote" fixtures; existing helper uses `url "."` not bare.
