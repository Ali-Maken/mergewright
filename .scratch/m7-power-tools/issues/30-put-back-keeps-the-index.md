# 30 put-back-keeps-the-index — every set-aside put-back keeps what he staged

Status: blocked (needs graft: 5 app tests assert the old clause/status; see NOTES "Blocked"). graft-ops side done.
Blocked by: none for the put-back (06c, 28c merged); 31 only for the absorb C1 test box.
(`set_aside.rs` `put_back` / `from_index_base`). Any finding it files on the put-back lands before this ticket or is
folded into it, by the conductor's order. **Don't write 06b's absorb-only amendment of ADR-0045:131.** This ticket's
amendment, below, replaces that line for every kind.
Crates: graft-ops only (`apply/set_aside.rs`, `apply/error.rs` `AfterState`, `operation.rs` `SetAside.staged`, tests),
plus docs: the ADR-0045 amendment below, and graft-ops MAP.md. Another crate needed → commit at the seam, report
`blocked: needs <crate>` with the signature, stop.
Build with: Opus (the set-aside protocol, ADR-0045; undo must hold; git behaviour probed before it is built on).
Scout: done (`.scratch/m7-power-tools/scouts/rmcached.md`). No second scout. Re-read each cited site before editing:
06b, 06c and 28c all touch graft-ops.
Split by: planner, 2026-09-26, from KNOWN_GAPS M7/rmcached (review `docs/reviews/m7-28-ignored-guard.md` Q1). It also
closes M7/rR-index's `r`/`R`/`a` index loss (see Decided 1).

## Read first
- The scout's report (whole, 1 page).
- ADR-0045 §4, §5, §6, §7, Consequences :129-131, and the 2026-09-26 amendment's §5 (`EditsAt`).
- ADR-0049 Decision 1-3 and Why (for Decided 4).
- 06b NOTES "Mechanism" and "Touches" (`.scratch/m7-power-tools/issues/06b-absorb-staged-and-replay.md:35-43`).
- graft-ops MAP.md: `SetAside` (:32), `apply` set-aside (:200), `ApplyError`/`AfterState` (:206-209), `AbsorbMap` (:279).
- `docs/research/commit-stash.md:226-231` and `:324` (why M6 rejected `--index`).

## Decided (planner)
1. **Every put-back keeps the index.** `git stash apply --index <S>` for every kind that sets aside, not just
   absorb. Why:
   - Plain `git switch` keeps a staged `rm --cached` (scout probe: `D  .env` after). Graft's switch turns it into
     ` M .env`: he asked to stop tracking a file, and Graft silently tracks it again. It is usually `.env`, and
     `stage_all` would then commit his secret.
   - With plain apply, content that exists **only in the index** is lost from the tree. Example: he stages v2, then
     edits the file to v3; plain apply brings back v3 and v2 is gone. `S` is dropped on the clean path, so v2 then
     survives only in the entry's backup, and nothing tells him. That breaks "you cannot lose work", with no words.
   - The instant switch has no preview (ADR-0048 §1), so the preview's `· N staged comes back unstaged` never shows
     where it matters most.
   - M6 rejected `--index` because it "refuses on conflict" (study :324, Magit #4993). Decided 2 removes that
     refusal. Prior art: **take** `git switch`'s rule (staging carried). **Invent** the fallback, because nothing
     found fits.
2. **When `--index` can't reinstate the index, fall back to a plain apply, and say so.** Don't refuse before starting.
   - A refusal would be stricter than today and a dead end: he must leave for the shell. That is a "Time I left",
     and it is against pick 16.
   - Detect the case **by reading, never by git's message** (rule 3; A1.5). After `stash apply --index S` fails, read
     the status once:
     - something is unmerged → a clash, handled as today;
     - nothing is unmerged **and no tracked file changed** (git wrote nothing) → run `git stash apply S` once, then
       read the status again, with today's three outcomes;
     - anything else → today's other-failure path (`S` kept, `EditsAt::Stash`).
   - **Never run a second apply over a tree the first one changed.** That would apply the edits twice.
   - If the plain apply lands clean: the entry is `Committed` and `S` is dropped as usual. `apply` returns
     `ApplyError::AfterState { unread: AfterState::StagingNotKept { paths } }`, which the app renders as `Incomplete`
     (a completed write). Words, exact:
     `your edits are back · staging not kept: <path>[ +N more] · u undoes it, staging included`.
   - Nothing is lost. The exact split is in the entry's backup, and `u` restores it (ADR-0045 §6).
   - **Rejected:** keep `S` on fallback. It is a stash he would forget (ADR-0045 A2), and `S p` would not reinstate
     the index either.
   - **Rejected:** Graft rebuilds the index by its own 3-way merge. That is new merge logic where git has a verb.
3. **Undo holds, unchanged.** The generic inverse restores the index from the backup, so it restores the staged split
   (§6). `index_after` is read after the put-back as today, so it now records the kept staging, and redo restores
   it. The §6 `StashDrop` rule is unchanged: both a clean put-back and a clean fallback drop `S`.
   - A clash is the same under `--index`. That is expected, not verified: git resets the index before its merge,
     then says `Index was not unstashed`. The probe (P4) verifies it. Consequences :129 stands.
4. **The second finding is not ADR-0049's missing case.** The `reset --hard` at `set_aside.rs:118` overwrites
   `.env`, a file HEAD tracks and he `rm --cached`'d. His bytes are still safe:
   - `stash create` put his bytes into `S` at step 1, and `S` is listed at step 3, **before** that reset (step 4). The
     scout's probe shows it: the put-back wrote `MINE` back, and `S` was the only place `MINE` could come from.
   - ADR-0049 refuses a write over bytes that **nothing else holds**. These are held by a listed stash, which is
     ADR-0045 §5's invariant ("in the tree or in the stash list at every instant").
   - In scope as **tests only** (T10), plus a §5 clarification in the amendment. If a test shows `S` does not hold
     the bytes, stop and report it: then ADR-0049's guard does need the case.
5. **`SetAside.staged` is always 0** (as for absorb today). Its words would now be false in the normal case. The
   preview does not forecast the fallback. The builder puts that in NOTES as a gap for the conductor.

## Scope
**In:**
- The probe, first, before any code (below).
- One put-back function for every kind: `--index`, then the fallback. Absorb's rebuilt stash (`from_index_base`) is one
  input to the same apply. It never falls back in practice, because `HEAD^{tree}` equals the base's tree.
- `AfterState::StagingNotKept`.
- `SetAside.staged` → 0.
- Red-first tests T1-T11.
- The ADR-0045 amendment.
- graft-ops MAP.md: lines :32, :200, :206-209.

**Out:**
- The graft side: its preview clause `· N staged comes back unstaged` becomes dead code, and its `Incomplete`
  rendering is untouched. Say in NOTES what the follow-up is.
- A preview forecast of the fallback.
- The staging lost on a clash (Consequences :129, existing gap).
- ADR-0049's text.

## Probe (red-first step 0; temp repo in the scratchpad, never in the tree; commands + observed status in NOTES)
- P1: the scout's state, with `other`'s `.env` the **same** blob as `main`'s. Run Graft's sequence with
  `stash apply --index S` on `other`. Expected, unconfirmed: `D  .env` and `.env` = `MINE`.
- P2: the same with `other`'s `.env` **different**. First, what plain `git switch` does. Then the sequence with
  `--index`: exit code, stderr, and the status before and after. Record whether anything was written.
- P3: the fallback case. A 20-line file: stage an edit to line 5; `other` changes line 7. Does `--index` refuse with
  nothing written, and does a plain apply then land clean?
- P4: a worktree clash under `--index`. Are the unmerged set and the stages the same as a plain apply's?
- P5: MM round trip (staged v2, worktree v3) with `--index`.
- **P1 not `D  .env` → stop and report.** The design rests on it. P3 writing something → the fallback is never taken.
  That is still safe (the other-failure path), so build on, and say so in NOTES.

## Signatures (match graft-ops' existing `Git`/`Context`/error types in `set_aside.rs`; don't invent new ones)
```rust
// graft-ops/src/apply/error.rs
pub enum AfterState {
    // …existing variants unchanged…
    /// The put-back brought the edits back, but git could not reinstate the index on the landed tree
    /// (`stash apply --index` refused and wrote nothing), so it ran without `--index`. `paths`: the staged
    /// paths whose staging did not come back. `u` restores the split exactly (ADR-0045 §6, amended 2026-09-26 b).
    StagingNotKept { paths: Vec<PathBuf> },
}
// Display: "your edits are back · staging not kept: <first>[ +N more] · u undoes it, staging included"

// graft-ops/src/apply/set_aside.rs (crate-private)
/// How a put-back ran: read from the status, never from git's exit code or message.
pub(crate) enum PutBackRan {
    Whole,                                // --index applied clean
    Unstaged { paths: Vec<PathBuf> },     // --index refused, nothing written; the plain apply ran clean
    Clashed { paths: Vec<PathBuf> },      // unmerged after either apply (ADR-0045 §2)
    Failed { error: /* existing git error type */ },  // S kept, EditsAt::Stash
}
fn run_put_back(ctx: &Context, stash: Oid) -> Result<PutBackRan, ApplyError>;
/// Staged paths the plain apply loses: `diff-tree -r S^1 S^2` minus added and copied (the m6-conflicts/83 rule,
/// moved here from the `SetAside.staged` count).
fn staging_lost(ctx: &Context, stash: Oid) -> Result<Vec<PathBuf>, ApplyError>;
```
- If the drop fails after a fallback, `AfterState::SetAsideKept.words` ends with ` · staging not kept: <paths>`. Only
  one `unread` is returned, and neither is dropped.

## Tests (red first unless marked; each named at tick time)
- T1 `switch_keeps_a_staged_rm_cached`: the Q1 fixture. `D  .env`, `.env` bytes `MINE`. Red today (` M`).
- T2 `switch_keeps_index_only_content`: staged v2, worktree v3. After: index v2, worktree v3. Red today.
- T3 `every_set_aside_kind_keeps_the_index`: a table over every kind `sets_aside` returns `Some` for (grep: rebase,
  checkout, recover `MoveRef`, `r`, `R`, `a`, pull if 03b has merged). T2's fixture for each. Red for each kind today.
- T4: absorb's `absorb_staged::*` stay green (not red first).
- T5 `put_back_falls_back_when_the_index_cannot_go_back`: P3's fixture. The edit is in the worktree, entry
  `Committed`, `S` not listed, `StagingNotKept { paths: [f] }`, Display words exact. Red today (`Ok`).
- T6 `undo_after_a_fallback_restores_the_split`: P3 plus index-only content. `u` gives the index and worktree exactly
  as before.
- T7: a clash under `--index` gives the same `Clashed { SetAside }`, paths and a tied `live_clash`. Name one existing
  `stash_clash` test that stays green, plus the new clash row.
- T8: an other failure (tree changed, or both applies failed) keeps `S` and never runs the plain apply over a
  changed tree. Assert the Command Log. No fixture reaches "tree changed" → say so and mark it unverified.
- T9: ADR-0045 §7's round-trip table gains these rows: `rm --cached`, index-only content, staged mode-only, staged
  rename, staged new file. apply → undo equals before; redo equals after.
- T10 `the_rm_cached_bytes_are_never_only_on_disk`: the existing `Stop` at each step 4-6. At each stop, `MINE` is
  readable at `<listed S>:.env` or on disk. After `u` of the switch, `.env` is `MINE` on disk and `D  .env`. **A red
  whose cause is outside the set-aside (the undo's restore) → stop and report it (rule 2). Don't fix it here.**
- T11: `SetAside.staged == 0` for every kind. Name the m6-conflicts/83 round-trip tests that flip, and why.

## Acceptance
- [x] P1-P5 recorded in NOTES (commands, observed status). Probe script, NOTES "Probe".
- [x] T1-T11, each named, red run quoted where red first. NOTES "Tests"; all in
  `crates/graft-ops/tests/it/put_back_index.rs`, `set_aside.rs`, `absorb_staged.rs`, and
  `apply/set_aside/put_back.rs` unit tests.
- [ ] Every set-aside test in both crates green: **graft-ops side green; 3 `crates/graft` tests red** (they assert
  the old `· 1 staged comes back unstaged` clause) — `blocked: needs graft`, NOTES "Blocked". Filter run: 404 run,
  401 passed, 3 failed (all `graft::it`).
- [x] ADR-0045 amendment appended verbatim (below), with test names filled in. Plus the review 06b M3 seven points
  (carry). `docs/adr/0045-…` "Amendment 2026-09-26 (b)".
- [x] MAP.md graft-ops updated: `SetAside` (:32), `apply` set-aside (:200), new `AfterState::StagingNotKept` row.
- [ ] Gates, workspace-wide: `cargo fmt --check` pass, `cargo clippy --all-targets -- -D warnings` pass,
  `cargo test --doc` pass; `cargo nextest run` **fails on 5 `crates/graft` tests** (NOTES "Blocked"). graft-ops
  749/749.
- [ ] Carry, absorb C1 test `a_staged_file_deletion_drops_the_commit_that_made_it`: **open, ticket 31 has not
  landed** (no 31 commit on this branch's history at 3cfb8fe6).

## Touches (check each, say how)
- Put-back callers, found with `grep set_aside::close`:
  - `apply.rs` `closed`;
  - `resume.rs` `close_resume` (a rebase that stopped, then `c`/`a`).
  - Both go through `run_put_back`.
- Every `sets_aside` kind: see T3.
- **Pull 03b.** Merged → T3's pull row. Not merged → report it, and the conductor adds that row to 03b.
- Undo/redo: T6, T9, and `undo_after_absorb_*`, all green.
- `clash::live_clash`: T7.
- Preview: `SetAside.staged` → 0.
  - The graft preview clause goes dead. Grep `staged` in `crates/graft` and list the site for the follow-up.
  - `preview/put_back.rs` is unchanged: the forecast models the worktree merge, and `--index` doesn't change that.
- `AfterState`:
  - An exhaustive `match` in `crates/graft` (grep `AfterState::`) → `blocked: needs graft`, naming the arm.
  - Quote in NOTES how `Operated::Incomplete`'s `error` string is built from it, so the conductor sees the line he
    will read.
- Command Log: the new line `stash apply --index`.
  - Grep `stash apply` in `crates/graft/tests` and `docs/shots`.
  - An exact-line assert or capture that changes → list it (stale captures go to the conductor).
- Journal serde: no new line field. Show it with a grep of `Serialize` on `AfterState`.
- Docs:
  - ADR-0045 (amended).
  - ADR-0049 (unchanged; the amendment says why).
  - `docs/design-notes/m6-stash-put-back.md:33,118` goes stale; name it for the conductor.
  - KNOWN_GAPS rows M7/rmcached, M7/rR-index, and M6's "put-back loses the staged split": the conductor updates them
    from the report.

## ADR-0045 amendment text (append verbatim; fill the test names)

```
## Amendment 2026-09-26 (b) — every put-back keeps the index (ticket m7-power-tools/30; review m7-28 Q1;
scout `.scratch/m7-power-tools/scouts/rmcached.md`)

### §4 step 6 amended
Put back with `git stash apply --index <S>`, then read the status whatever the exit code:
- nothing unmerged → clean: drop `S` by id, `Committed`;
- unmerged → the clash (§2). Git resets the index before its merge (`Index was not unstashed`), so it is the clash a
  plain apply gives;
- nothing unmerged and no tracked file changed → git could not reinstate the index on the landed tree and wrote
  nothing. Run `git stash apply <S>` once, and read again with the same outcomes. A clean result is `Committed`, `S`
  dropped, and `AfterState::StagingNotKept { paths }`:
  `your edits are back · staging not kept: <path>[ +N more] · u undoes it, staging included`;
- anything else → as before (`S` kept, `EditsAt::Stash`). A second apply never runs over a tree the first changed.
Why: `git switch` carries what he staged (a `git rm --cached` stays `D `). A plain apply turned it into ` M`, and it
dropped content that existed only in the index, with no words. M6 rejected `--index` because it refuses on conflict
(study `commit-stash.md:324`, Magit #4993). The fallback removes that refusal. Absorb's rebuilt stash (06b) is one
input to the same apply.

### Consequences :131 replaced
A put-back keeps the staged/unstaged split, as `git switch` does, except (a) on a clash, where every file ends staged
(:129), and (b) when git cannot reinstate the index, where the line names the files. In both, `u` restores the split
exactly (§6). `SetAside.staged` is always 0, and the preview no longer says `N staged come back unstaged`. The
preview does not forecast (b). This replaces 06b's absorb-only supersession of this line.

### §5 clarified — a path HEAD tracks and the index doesn't
A `git rm --cached` file, ignored or not, is set aside with the rest. `stash create` writes its bytes into `S`
(step 1), and `S` is listed (step 3) before the reset (step 4) writes HEAD's copy over it. So its bytes are in the
tree or the stash list at every instant, and ADR-0049's refusal, which is for bytes nothing else holds, does not
apply. Pinned by `<T10 name>`; the round trips by `<T9 names>`.
```

## Gates
Workspace-wide, quiet: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`.

## Size
About 55k: the probe about 8k, the put-back and error about 15k, tests about 25k, docs about 5k. Over budget → hand
off after T5, with T6-T11 and the ADR left.

## NOTES (builder)

Scout: used the ticket's (`scouts/rmcached.md`); no second one. Re-read each cited site before editing. No compaction.

### Probe (git 2.54.0; throwaway repos in the session scratchpad; Graft's sequence = `stash create`, `stash store`,
`reset --hard`, `checkout <b>`, then the apply)
- **P1** (`.env` same blob on `other`): before `D  .env` / `!! .env`. `stash apply --index S` exit 0, stderr empty;
  after `D  .env` / `?? .env`, `.env` = `MINE`. `S:.env` = `MINE`; `S^2` has no `.env`; after `reset --hard` and
  after `checkout`, `.env` = `base` until the apply. **Design premise holds.**
- **P2** (`.env` differs on `other`): plain `git switch other` refuses (exit 1, "Your local changes … would be
  overwritten"), nothing changes. Graft's sequence with `--index`: exit 1, stderr `error: patch failed: .env:1` /
  `error: .env: patch does not apply` / `error: conflicts in index. Try without --index.`; status after: empty
  (nothing written; `.env` = `theirs`, HEAD's). Then plain apply: exit 1, `UU .env` with markers — a clash, `S`
  kept (§2).
- **P3** (20 lines; staged line 5, `other` changes line 7): `--index` exit 1, same three stderr lines; status after
  empty, `git diff HEAD --stat` empty (nothing written). Plain apply: exit 0, ` M f`, `five` and `seven` both in.
  **The fallback is taken.**
- **P4** (staged `g`, unstaged `f` clashing): `--index` → exit 1, `Index was not unstashed.`, `UU f`, `M  g`,
  stages 1/2/3 for `f`; plain → exit 1, `UU f`, `M  g`, the same three stage blobs. **Same clash.**
- **P5** (MM: staged v2, worktree v3): `--index` exit 0, `MM f`, index v2, worktree v3.

### Mechanism
- `apply/set_aside/put_back.rs` (new, crate-private): `run_put_back` = `stash apply --index`, `judge` the status
  (unmerged → clash; exit 0 → whole; failed + no staged/unstaged entry → wrote nothing; else wrote), fallback once,
  `staging_lost` (`diff-tree -r --no-renames --name-status -z X^1 X^2`, less `A`/`C`) read **before** the plain
  apply — if it fails, nothing more runs and `S` is kept with that error. `from_index_base`, `resolve`,
  `commit_tree` moved here from `set_aside.rs` (review 06b M2's split, partly: `set_aside.rs` 690 → 671 lines, still over 6.2's ~400).
- "Wrote nothing" is judged as "every tracked file equals `HEAD`". The put-back starts on the tree the reset and the
  operation left, so a tree still equal to `HEAD` holds none of the edits: a second apply cannot double them.
- `Returned::Back`/`Outcome::Back` carry `unstaged`; `Kept::Git`/`Kept::Unlisted` carry it for the words. Drop
  failing after a fallback: the entry reason and `SetAsideKept.words` end ` · staging not kept: <paths>`; one unread.
  Operation failed + fallback clean: `ApplyError::Git`, the entry's `Aborted` reason ends with the same clause.
- C2 is closed by the same rule: absorb aborted/failed passes `index_base: None`, so `S` goes back with `--index`
  over `HEAD^{tree}` = `S^1^{tree}` — exact.

### Tests (red run quoted; `cargo nextest … -E 'test(/put_back_index/)'` before the fix, 3 passed / 9 failed)
- T1 `switch_keeps_a_staged_rm_cached` — red: `left: " M .env"`, `right: "D  .env\n?? .env"`.
- T2 `switch_keeps_index_only_content` — red: `checkout: index keeps v2` (`left: "v1"`).
- T3 `every_set_aside_kind_keeps_the_index` (Checkout, `r` Rebase, `R` Rewrite dropping a row, recover `MoveRef`;
  undo + redo exact each) — red: `Checkout: index keeps v2` (`left: "v1"`). `a` row:
  `amend_into_commits_the_index_and_puts_the_rest_back` — **not red first** (`a`'s index goes into its commit, the
  index after equals `HEAD`; plain apply gave the same). Pull: **03b not merged** — `sets_aside` has no pull kind;
  the conductor adds that row to 03b. (The app's `pull-rebase-dirty` runs `Operation::Rebase`, covered by the `r` row.)
- T4 `absorb_staged::*`: 17/17 green, the 13 pre-existing ones unchanged.
- T5 `put_back_falls_back_when_the_index_cannot_go_back` (+ `staging_not_kept_counts_the_rest` for `+N more`) — red:
  `the staging should not be kept: Ok(Applied { … "git stash apply 7d9f…", "git stash drop 'stash@{0}'" … })`.
- T6 `undo_after_a_fallback_restores_the_split` — red: `Ok(Applied …)` where `AfterState` was expected.
- T7 new `a_clash_with_staging_is_the_same_clash` (P4 fixture: `UU f.txt` / `M  g.txt`, `Clashed { SetAside }`,
  `live_clash` ties) — red only on its Command Log assert (no `--index` line). Existing
  `set_aside::a_clashing_put_back_is_clashed_and_keeps_the_stash` green.
- T8 `both_applies_refused_keeps_the_stash` (both applies refused, `S` kept, `EditsAt::Stash{0}`, Log = two applies,
  no drop) — red: one apply. **"Tree changed" is unverified by a git fixture**: none found that makes
  `stash apply --index` fail after writing a tracked file. Its branch is pinned by the unit test
  `a_failed_apply_that_changed_a_tracked_file_wrote` on `judge` (plus `…_only_untracked_files_wrote_nothing`,
  `unmerged_is_a_clash_whatever_the_exit_code`), and by reading `run_put_back`: `Read::Wrote` returns before any
  second apply.
- T9 rows in `set_aside.rs`: `{rebase,checkout}_round_trips_an_rm_cached`, `…_index_only_content`,
  `…_a_staged_mode_only_change` (new), `…_a_rename`, `…_a_staged_new_file` (existing). Each now asserts status and
  `git diff --cached`/`git diff` equal to before, `staged` 0, undo = before, redo = after. Red against the old
  put-back (old `set_aside.rs` restored for one run): all 6 new rows red, e.g. `checkout_round_trips_an_rm_cached`
  `right: ["D  a.txt", "?? a.txt"]`.
- T10 `the_rm_cached_bytes_are_never_only_on_disk` (stops before 4, 5, 6: `MINE` on disk or at a listed `S:.env`)
  and `undo_of_the_switch_keeps_the_rm_cached` (`D  .env` / `!! .env`, `MINE`, whole state = before). Green first:
  they pin §5, not a change. `S` does hold the bytes (P1: `S:.env` = `MINE`), so ADR-0049 needs no case.
- T11 `set_aside_staged_is_zero_for_every_kind` — red: `Checkout` `left: 2, right: 0`. Flipped by it: the
  m6-conflicts/83 round trips `{rebase,checkout}_round_trips_{staged_unstaged_untracked,a_deletion,a_rename}` (their
  table was git's no-`--index` shape, ` M`/` D`/` D`+`A `; now `MM`/`D `/`R `, as before the operation) and the unit
  `operation::history::tests::sets_aside_table` (`staged: 3` → `0`).
- Carry C2: `abort_after_absorb_stops_keeps_index_only_lines` — red against the old put-back: `left: "o1\no2\no3…"`,
  `right: "o1\no2\nO3…"`. `r` row with an index-only line: `rebase_round_trips_index_only_content` (red, as T9).
- Carry M1 (stop from an `Edit` on `f3`): `absorb_stopped_then_continued_as_planned_keeps_the_stay` (green before:
  06b's path), `absorb_stopped_then_continued_elsewhere_says_the_staging_went` (red before: `Ok`; now
  `StagingNotKept [b.txt, old.txt]`, undo exact), abort = the C2 test + `undo_after_absorb_stops_then_aborts` (an
  `Aborted` entry has nothing to undo: `NothingToUndo`, state = before).

### Blocked: needs graft (the app crate; not in this ticket's crate set)
Failing workspace tests, all from the intended behaviour change:
- `m6_stash_put_back::set_aside::a_dirty_pull_rebase_is_three_keys` — `crates/graft/tests/it/m6_stash_put_back/set_aside.rs:744,753`
  expect `· 1 staged comes back unstaged`; its capture compare at :758.
- `m5_walk::every_scenario::journey_pull_rebase_dirty`, `…_pull_rebase_dirty_preview`,
  `…_rejected_push_dirty_to_pushed` — `crates/graft/tests/it/m5_walk/stash_journeys.rs:520` (the clause); after it,
  :542-547 `three_edits_back` expects `[" M one.txt", …]`, now `M  one.txt`.
- `m5_walk::every_scenario::journey_switch_dirty` — `stash_journeys.rs:775` expects `[" M base.txt", " M notes.txt"]`;
  now `["M  base.txt", " M notes.txt"]` (the staging kept).
Follow-up for the graft side: drop the clause at `crates/graft/src/update/preview.rs:956`
(`set_aside_row(aside.files, aside.staged)`; `graft_ui::preview_box::set_aside_row`'s staged clause is dead), and
the unit test `crates/graft/src/update/preview/tests.rs:1118` that builds it by hand. Stale captures:
`docs/shots/m6/m6-22-set-aside-preview-{80x24,120x40,187x50}.txt` (clause), `docs/shots/m6/set-aside-preview-120x40.txt`,
`docs/shots/m6/m6-24-set-aside-done-*` (`3 unstaged` likely now shows one staged; not re-captured — unverified).

### Gaps for the conductor
- The preview does not forecast the fallback (Decided 5, out of scope).
- The line he reads on a fallback, built in `crates/graft/src/jobs/operate.rs:333-339` (`unread` joined with `; `)
  and `crates/graft/src/update/operate.rs:845-848`:
  `checkout other — could not confirm it fully: your edits are back · staging not kept: f.txt · u undoes it, staging included`.
  "could not confirm it fully" misreads a completed write (CLAUDE.md status rule); the graft follow-up should word it.
- `docs/design-notes/m6-stash-put-back.md:33,118` stale (no `--index`).
- KNOWN_GAPS M7/rmcached, M7/rR-index, M6 "put-back loses the staged split", M7/06b-resume: closable from this report.

### Touches
- Put-back callers (`grep set_aside::close`): `apply.rs:368` (`closed`) and `apply/resume.rs:419` (`close_resume`);
  both reach `put_back` → `run_put_back`. Tests: T3 (apply), the M1/C2 resume tests.
- Every `sets_aside` kind (`operation.rs` `sets_aside`: Rebase, Rewrite, Checkout, Recover MoveRef): T3 + `a` row.
- Undo/redo: T3, T6, T9, `undo_after_absorb_*` — green.
- `clash::live_clash`: T7.
- `AfterState`: no exhaustive `match` in `crates/graft` (`grep -rn "AfterState::" crates/graft/src` → none); no
  `Serialize` on it (`grep Serialize crates/graft-ops/src/apply/error.rs` → none), so no journal field.
- Command Log: new `git stash apply --index <S>` line (and a second plain one on fallback). `grep "stash apply"
  crates/graft/tests docs/shots`: no exact-line assert of the set-aside's apply (`stash_journeys.rs:542` is a comment).
- `preview/put_back.rs` unchanged (models the worktree merge).
- ADR-0049 unchanged; the amendment says why.

## Carry (conductor, 2026-09-26): review 06b folds in here
Source: `docs/reviews/m7-06b-absorb-staged.md`.
- **C2 (Critical):** an absorb abort loses index-only lines (`apply/resume.rs:414`; `apply.rs:358-361` on failure), and so do `r`/`R` (`rewrite/plan.rs:150`, `index_base: None`). This is the same `put_back` arm as this ticket. Tests: `abort_after_absorb_stops_keeps_index_only_lines`, and an `r` row with an index-only line in the set_aside round trips, red first. When `HEAD^{tree}` equals `S^1^{tree}`, `--index` is exact (review §C2 fix).
- **M1:** resume tests: continue as planned, continue resolved elsewhere, and abort, each with an undo row. Stops come from an `Edit` row (`tests/it/rewrite_run.rs:366`) or the q3 fixture. These close M7/06b-resume.
- **M3 + the amendment:** the ADR-0045 amendment this ticket writes must also carry the review's seven points (§"What the ADR-0045 amendment must state"). Absorb's went-through path is one case of your rule.
- **Absorb C1 test** (after 31 lands): `a_staged_file_deletion_drops_the_commit_that_made_it`.
- Blocked by adds 31 for that test only; if 31 has not landed, leave that box open and say so.
