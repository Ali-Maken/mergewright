# 30 put-back-keeps-the-index — every set-aside put-back keeps what he staged

Status: ready
Blocked by: 06c, 28c (both hold graft-ops; serial). Sequencing note: review 06b is running on the same function
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
- [ ] P1-P5 recorded in NOTES (commands, observed status).
- [ ] T1-T11, each named, red run quoted where red first.
- [ ] Every set-aside test in both crates green: `cargo nextest run --workspace -E 'test(/set_aside|put_back|stash|absorb|recover|pull/)'`, count quoted.
- [ ] ADR-0045 amendment appended verbatim (below), with test names filled in.
- [ ] MAP.md graft-ops updated.
- [ ] Gates, workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
  `cargo test --doc`.

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

## Carry (conductor, 2026-09-26): review 06b folds in here
Source: `docs/reviews/m7-06b-absorb-staged.md`.
- **C2 (Critical):** an absorb abort loses index-only lines (`apply/resume.rs:414`; `apply.rs:358-361` on failure), and so do `r`/`R` (`rewrite/plan.rs:150`, `index_base: None`). This is the same `put_back` arm as this ticket. Tests: `abort_after_absorb_stops_keeps_index_only_lines`, and an `r` row with an index-only line in the set_aside round trips, red first. When `HEAD^{tree}` equals `S^1^{tree}`, `--index` is exact (review §C2 fix).
- **M1:** resume tests: continue as planned, continue resolved elsewhere, and abort, each with an undo row. Stops come from an `Edit` row (`tests/it/rewrite_run.rs:366`) or the q3 fixture. These close M7/06b-resume.
- **M3 + the amendment:** the ADR-0045 amendment this ticket writes must also carry the review's seven points (§"What the ADR-0045 amendment must state"). Absorb's went-through path is one case of your rule.
- **Absorb C1 test** (after 31 lands): `a_staged_file_deletion_drops_the_commit_that_made_it`.
- Blocked by adds 31 for that test only; if 31 has not landed, leave that box open and say so.
