# radar-skips-bookkeeping-commits: no duplicate mark from Graft's backups or the stash, no evil-merge mark on a stash

Status: draft
From: `docs/reviews/m6-usability-final.md` N1 (:89-110); the radar half of n5 (:246-252).
Build with: (conductor, after the scout)
Scout: `.scratch/m6-conflicts/scouts/radar-skips-bookkeeping-commits.md` — brief below.
Needs: none.
Crates: graft-ops (`mistake_radar/probes.rs`), graft (`update/detail/ops.rs` evil probe, `update/radar*`, `history` ref badges,
tests, walks).
Group: **A** (radar), alone. Runs first of the four: it changes which rows carry `ⓘ`, and so which `.` menus lead with
`Explain` (review N7), which every later group's captures show.

## Read first
- `docs/reviews/m6-usability-final.md` N1 (:89-110), n5 (:246-252), m9 row (:48).
- PROMPT.md §1.4 table, "Duplicate commits" row (:152): "computed … **only across visible refs**". This spec makes the
  probe match that line.
- `crates/graft-ops/MAP.md` §`mistake_radar::probes` (:235-244); `crates/graft-git/MAP.md` :163, :166
  (`references` includes `refs/graft/**`; `ref_snapshot` is "the repository as Graft draws it").
- `crates/graft/MAP.md` :76-88 (`RefBadges`, `stashes`), :473-491 (`RadarState`, duplicates probe), :520-524
  (`DetailState::evil*`, `probe_evil`).
- `.scratch/m6-conflicts/usability-final-plan.md`.

## The cause (rule 7, lines from the review at `1b1995cc`; the scout re-reads them on main)
1. **Duplicates.** `probe_duplicates` walks `git log --all` in both spawns (`graft-ops/src/mistake_radar/probes.rs:410-415`,
   `:424-430`). `--all` includes `refs/graft/backup/**`, which keep every pre-rebase commit for `u`, and `refs/stash`,
   whose base commit is the pre-rebase one. So each rebased commit's old self is a "copy". PROMPT §1.4 says visible refs.
2. **Evil merge.** The evil-merge probe runs on whatever merge the selection follows; nothing excludes a stash commit
   (`graft/src/update/detail/ops.rs:125-133`, `:157`). A stash commit is a merge whose tree differs from both parents by
   construction, so it is always "evil".

## Decided here
- **The duplicate walk is an allowlist: `--branches --remotes --tags HEAD`**, one constant both spawns read. Not a
  denylist of `refs/graft/*` and `refs/stash`. Why: a copy matters only where it can be merged (PROMPT :152, "on several
  branches"); every tool that writes refs adds a namespace a denylist must learn (git maintenance's `refs/prefetch/**`
  is a second one `--all` walks today). Trade-off: a copy reachable only from, e.g., `refs/pull/*` is not flagged.
- **A stash commit is never probed for an evil merge.** The app knows every stash commit (`RefBadges::stashes`); the
  selection's merge is filtered through it before `probe_evil`. While the refs are not yet read, no evil probe is
  asked (unknown is not "not a stash").
- **Out:** m9 (every branch with no upstream is `ⓘ`), the stash internals' rows in the graph (n5's drawing half,
  KNOWN_GAPS :208). Both stay open; the plan lists them.

## Scout brief (≤1 page, code not intent)
1. Re-read `probes.rs:410-430` on main: the exact args of both `log` spawns; is `--all` in both; anything else
   (`--exclude`, `--no-walk`) there. List every test in `graft-ops` that builds a duplicate group, and which refs its
   copies are on (a test whose copy is only on a non-branch ref breaks under the allowlist).
2. `update/detail/ops.rs:120-160`: who calls `probe_evil`, with what `selected`; where `evil_at` is set and cleared;
   whether `view::radar` or the `.` menu (`context_menu.rs:139`) reads `evil`/`evil_cache` by any other path.
3. `RefBadges::stashes` — its cap (50, from `graft_git::stash::commits`) and whether `AppState::refs` is `None` before
   the refs land.
4. The `.` menu: which rows lead with `Explain` because of a duplicate or evil mark in any `tests/it` fixture or capture
   (`grep -rln "Explain" docs/shots`), so the builder knows which captures move.
5. The app-level harness that drives `p r r` against a real origin (m4 walks, `remote_by_key`), for the walk below.

## Scope
**In:** the allowlist constant and both spawns; the stash filter before `probe_evil`; the "no evil probe before the refs"
rule; tests and captures below.
**Out:** `probe_evil_merge` itself, the window size, m9, n5's drawing, the `Explain` item's position (N7).

## Signatures
```rust
// graft-ops, mistake_radar/probes.rs — the one rev set both `log` spawns take (was `--all`).
/// What the duplicate probe walks: every ref the user can merge (PROMPT §1.4, "visible refs").
/// Never Graft's `refs/graft/**`, never `refs/stash`, never another tool's bookkeeping.
pub(crate) const DUPLICATE_WALK: &[&str] = &["--branches", "--remotes", "--tags", "HEAD"];
// `probe_duplicates(repo, window, stop) -> Result<Duplicates, RunError>` — signature unchanged.

// graft, history ref badges (beside `RefBadges::stashes`)
impl RefBadges {
    /// Whether `id` is any stash entry's own commit (`stash@{n}`, n < the list's cap).
    #[must_use] pub fn is_stash(&self, id: Oid) -> bool;
}
// graft, where the scout finds `selected` built for `DetailState::probe_evil`:
// the merge handed over is `selected.filter(|id| refs.is_some_and(|r| !r.is_stash(*id)))`.
// `probe_evil`'s own signature is unchanged.
```

## The failing tests first
- graft-ops `probes` tests, each red today:
  - `duplicates_skip_graft_backup_refs`: A on `main`; a backup ref `refs/graft/backup/main/<ulid>` at A; `main` reset
    to A′ (same patch, new parent). Assert no group holds A′.
  - `duplicates_skip_the_stash`: a stash made on A, then `main` rebased to A′. No group.
  - `duplicates_skip_other_bookkeeping_refs`: a copy only under `refs/prefetch/remotes/origin/main`. No group.
- graft `update` tests: `a_stash_commit_is_never_probed_for_an_evil_merge` — refs with a stash at X (two parents),
  selection on X: no `Command::EvilProbe`, `evil` not `Ready(Evil)`, the row draws no `⚠`. Red today.

## Touches (each entry, and how the builder checks it)
- **`jobs::radar::run_duplicates_probe`** (the only caller): unchanged signature. Check: build.
- **`RadarState::duplicate_findings`** and the rows' `ⓘ`: fewer findings. Check: `grep -rn "DuplicateCommit" crates` —
  every test that expects a finding still gets it, or NOTES says why its copy was on a non-visible ref.
- **The `.` menu's `Explain` item** leads on fewer rows. Check: menu tests and captures from scout Q4; each regenerated
  by its own test, listed in NOTES.
- **The detail pane's radar line** (`RadarLine`) on a stash row: none drawn. Check: the evil test above draws the pane.
- **Captures** with `ⓘ`/`⚠` on a rebased or stash row: `grep -rln "ⓘ\|⚠" docs/shots`; each hit either unchanged
  (`git diff --exit-code`) or regenerated by its test.
- **TESTING.md** lines expecting a duplicate mark: `grep -n "DUPLICATE\|duplicate" TESTING.md`; listed in NOTES for the
  acceptance ticket.
- **KNOWN_GAPS :208** (n5): its radar half is closed by this; the builder says so in NOTES, the conductor edits the row.
- **Journal/undo:** none; backup refs are only read less. **Keymap:** none.
- **MAP.md:** graft-ops §`mistake_radar::probes` (`DUPLICATE_WALK`, the allowlist); graft `RefBadges::is_stash`,
  `probe_evil`'s caller.

## Acceptance (name the verifying test at tick time)
- [ ] The three `probes` tests above, red then green.
- [ ] Control, green before and after: `duplicates_across_two_branches_are_still_found` (A on `feature`, its
      cherry-pick on `main`: one group of two).
- [ ] `a_stash_commit_is_never_probed_for_an_evil_merge`, red then green.
- [ ] Control: `an_evil_merge_on_a_branch_is_still_marked` (a real evil-merge fixture keeps `⚠`).
- [ ] `no_evil_probe_before_the_refs_are_read`.
- [ ] Walk, real repo (`tests/it`): `a_pull_rebase_marks_no_duplicates` — origin moved, a stash on the pre-rebase tip,
      `p r r` through the app: `main`'s rebased rows carry no `ⓘ`; with the cursor on `stash@{0}` that row carries
      no `⚠`.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-radar-bookkeeping-120x40.txt`, from the walk.
  - **What right looks like:** frame 1, after `p r r`: no `ⓘ` in the marker column of any `main` row. Frame 2, cursor
    on `stash@{0}`: no `⚠` on that row, and the detail pane (if open) has no `EVIL MERGE` line. Frame 3, `.` on
    `main`'s tip: the menu's first item is not `Explain`. Every branch in the fixture has an upstream, so m9's `ⓘ`
    does not appear.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small–medium: one constant, one filter, five tests, one walk, one capture. One context.
