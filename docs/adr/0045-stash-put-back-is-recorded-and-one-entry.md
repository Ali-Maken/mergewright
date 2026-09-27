# ADR-0045 — A stash put back is recorded by Graft; setting edits aside around a rebase or switch is one entry

Date: 2026-09-25 (amended 2026-09-25: §2 tie to the index, §7 table rows — review m6-final-fixes-72-72b M2; amended
2026-09-26: §3 set-aside close words, §5 where the edits are, new §8 the preview forecasts the put-back — review
m6-final-fixes-74 M1–M3, see "Amendment 2026-09-26" at the end)
Status: accepted. Direction by the user: picks 15 and 16 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24-25,
field notes :67, "all three took the recommended option"). Mechanism by the design pass:
`docs/design-notes/m6-stash-put-back.md`. Study: `docs/research/commit-stash.md` addendum A1–A4.
Supersedes: `docs/design-notes/m5-history-and-ghost-preview.md` hard choice 4 ("rebase on a dirty tree: refused, never
`--autostash`") for **rebase and checkout**. Other kinds keep their refusals.

## Context
- A pop that clashes keeps the stash, leaves markers and **no operation state** (A1.1, A1.3). Nothing in git says "a
  stash is half put back", so neither the resolver's close nor a relaunch can name the stash or finish the job.
- M5 refused a dirty rebase because git's `--autostash` put-back "can conflict outside any journal entry" (hard choice
  4). His pick 16 asks for the put-back anyway. The reason stands; the answer is to put it **inside** the entry.
- git exits 0 when an autostash put-back clashes (A1.5). A tool that trusts the exit code hides the clash (VS Code
  #262360, Sublime Merge #1199).

## Decision

### 1. Graft records every put-back it starts, on the journal's Pending line
`PutBack { stash: Oid, why: Pop | Apply | SetAside }`, on `PendingLine` and `JournalEntry` as
`#[serde(default, skip_serializing_if = "Option::is_none")]` — the same additive rule as `merge_before`: every line
written before reads as `None` and re-renders byte-identically. For a pop or apply, `stash` is the entry's commit. For
a set-aside it is the commit `git stash create` printed **before** the Pending line was written (§4).

### 2. A clash is live by one pure rule over the journal and the index now (`graft_ops::clash::live_clash`)
- No clash while git has an operation in progress, or while nothing is unmerged.
- Walk the in-force entries newest first. Skip an entry whose `index_before` is unmerged or not recorded (a step taken
  inside the clash: stage, a whole-file choice, start over; or an entry that never touched the index), and an Undo or
  Redo whose `index_after` is unmerged (walking back into the clash). The first entry left must have `index_before`
  merged, `index_after` unmerged and `put_back: Some` — that is the **candidate**. Anything else: no clash.
- **Tied to the index now** (amended 2026-09-25). The newest in-force entry that recorded an `index_after` — the
  candidate itself, or a step Graft took inside the clash since — must record **exactly the index now**, compared as
  `IndexState` (the stage tree: every path's stages, modes and blob ids). Anything that changed the index without a
  journal line since — a shell `add`, a shell pop, a reset, an editor staging a file — breaks the tie: no clash.
  The index now is read by the same `index_state` the recorder uses (one encoding, so equal content is an equal id),
  and only while a candidate exists: no read, no cost, when nothing Graft put back is in question. The read failing is
  no clash, and the status says it could not tell (rule 3: worded, never a silent "no").
- The stash must still be in the list, found **by commit id**; its position is looked up, never assumed.
- A pop made in the shell has no record: the resolver works as today and names no stash. **Residual: none that
  drops.** The tie fails only towards "no clash", where the close names no stash and drops nothing, and every stash
  stays in the list for `S d`. A shell act that leaves the index exactly as Graft last recorded it is, stage for stage
  and blob for blob, the same conflict.
- **Replaces** (2026-09-25) the first residual clause: "a shell pop that clashes straight after a Graft
  **apply**-clash was finished, with no Graft write between, is named as that apply's stash. Apply never drops, so
  nothing is lost." That held for apply only. Review m6-final-fixes-72-72b M2 reproduced the pop case in git: Graft
  pops A and it clashes; in the shell `git checkout --ours a.txt && git add a.txt`, then `git stash pop stash@{1}`
  (B) clashes on `b.txt`. The old rule named A and the close dropped A, a stash he had kept on purpose. Under the tie,
  the shell `add` and the shell pop each changed the index outside the journal, so no clash: B's conflict resolves as
  a shell pop, and both stashes stay listed.
- Signatures (graft-ops `clash`): `clash_candidate(entries, in_progress, any_unmerged, stashes) ->
  Option<ClashCandidate>` (the walk, plus `expect`: the index the tie needs); `live_clash(entries, in_progress,
  index_now: &IndexState, stashes) -> Option<StashClash>` (the whole rule, pure; "anything unmerged" is
  `index_now.is_unmerged()`). The app asks for the index only when `clash_candidate` is `Some`.

### 3. Finishing drops the stash by commit id, without a box
When the last clashed file is marked done, a **pop** or **set-aside** clash ends in
`Operation::DropPutBack { stash }`: `stash drop stash@{n}`, `n` looked up in the facts `apply` reads, refused
`NoSuchStash` when it is gone, journal kind `StashDrop` with `pins: [stash]`, so `u` puts it back (the existing
`StashStore` inverse). An **apply** clash never drops. Undo is the guard, not a confirm (GitUp; lazygit #4491).
(Amended 2026-09-26: what the set-aside close says, and what taking the branch side everywhere costs — see the end.)

### 4. Setting aside is one entry, in this order
For `Rebase` and `Checkout` when tracked files are changed (`Operation::sets_aside`); untracked files are left where
they are and covered by the entry's backup, as every write is (ADR-0009).
1. `git stash create "<message>"` → `S`. Writes objects only: no ref, index or file changes.
2. The before-snapshot and the Pending line, with `put_back = SetAside(S)`, `pins ∋ S`, `touches ∋ refs/stash`.
3. `git stash store -m "graft: your edits, set aside for <description>" S` — from here `S` is in the stash list.
4. `git reset --hard` (tracked files only; untracked untouched).
5. The operation's own plan.
6. Put back: `git stash apply <S>` (no `--index`), then **read the index whatever the exit code**. Nothing unmerged →
   `stash drop` of `S` by id, entry `Committed`. Unmerged → `S` kept, entry `Committed` with an unmerged
   `index_after`, outcome `ApplyError::Clashed` (§2 finds it). Another failure → `S` kept, words name it.
- The operation **stops on a conflict** at 5: the entry stays `Pending` with `put_back` on it; `S` waits in the list.
  `resume` runs step 6 after `--continue` finishes and after `--abort` (onto the original tip, where `S` was made).
- The operation **fails** at 5 (git refused, e.g. an untracked file in the way): step 6 onto the unchanged tip, then
  the failure's own words.

### 5. The edits survive every failure
| Stops at | Working tree | `S` | What he is told |
|---|---|---|---|
| 1 or 2 | untouched | not stored | the error; nothing changed |
| 3 | untouched | objects only, pinned by the backup | the error; nothing changed |
| 4 or later | reset or part-way | in the stash list | the error names `stash@{n}`; `u` restores the backup |
| a kill anywhere after 3 | as git left it | in the stash list | relaunch: the interrupted line names `stash@{n}` |
| 6, clash | markers | in the list | the resolver (§2, §3) |
| 6, other failure | the operation's result | in the list | `your edits are in stash@{n} · S p puts them back` |

(Amended 2026-09-26: the words come from where `apply` says the edits are, never from a second read of the list —
see the end.)

### 6. Undo and redo
- **Undo** of a set-aside entry: the generic inverse (branch back by compare-and-swap, `HEAD`, index and working tree
  from the backup — the edits come back exactly, staged split included), plus `StashDrop(S)` **only when the entry's
  `refs/stash` moved to `S`** (a clash left it there): the same rule as a push's undo. A clean put-back already
  dropped `S`, so undo stores nothing. `S` stays pinned by the undo entry's backup.
- **Redo**: the after-state, plus `StashStore(S)` only when the undo's own `refs/stash` moved (the existing pop/drop
  rule, m6-conflicts/07 F2).
- **Undo during a clash** (`esc u`) is the "abort" A4 asked for: pop or apply → the tree before the pop, stash still
  listed (git kept it); set-aside → the tree and branch before the rebase or switch, edits back, `S` dropped as a
  duplicate of what came back. With a take made since, the drift box asks first, as for any undo.
- A set-aside entry that is still `Pending` (a rebase stopped) is not undoable, as today; `a` is its way back.

### 7. What must be proven before it ships
- Round trip on a table of trees (staged + unstaged + untracked, a staged new file, a deletion, a rename): apply →
  undo leaves the index tree, the working-tree tree and the stash list equal to before; redo equal to after.
- A stop injected before each step 1–6 (the existing `Stop`) leaves the edits in the tree or in the stash list, never
  in neither — asserted by reading both.
- The put-back's outcome comes from reading the index, never the exit code: a failed `stash apply` with nothing
  unmerged (an untracked file in the way) is "kept", one with unmerged paths is a clash. `stash apply` itself exits 1
  on a clash, so git's exit-0 case (A1.5, `--autostash`) is excluded by not using `--autostash`, not by a test.
- `live_clash`'s rule as a table test, including the undo-back-into-a-clash row and the shell-pop row, and (amended
  2026-09-25) the tie's rows:
  - **a shell pop after an unfinished Graft pop** (review 72+72b M2): the index now differs from Graft's last
    `index_after` → no clash, so nothing is dropped;
  - the same, the shell pop clashing on the **same path** with other blobs → no clash;
  - a Graft step inside the clash (a file marked done), the index now equal to that step's `index_after` → the clash;
  - the newest index-touching entry with `index_after` not recorded → no clash;
  - a Graft apply clash finished, then a shell pop (the replaced residual) → no clash.
- The review's four steps replayed in a real repository: `read` of the index + `live_clash` → no clash; the app's
  close after `⏎` on `b.txt` leaves both stashes listed. And the plain Graft pop clash in a real repository is tied
  (the recorder's and the reader's encodings agree).

## Consequences
- `check` no longer refuses `DirtyTree` for rebase or checkout. Ticket 67's gate keeps its job for every other refusal;
  its dirty-tree tests flip in the set-aside group.
- After a clash is resolved, every file is **staged**, as git leaves it (A1.2). Not unstaged by Graft: the files he
  had staged before are indistinguishable. Named in KNOWN_GAPS.
- A put-back loses the staged/unstaged split on the clean path too (no `--index`); the preview says so before `y`.
- A fast-forward pull over overlapping edits still fails with git's words. Out of pick 16's scope; a KNOWN_GAPS row.
- (2026-09-25) While a Graft put-back clash is a candidate, each working-tree or journal read that lands asks for one
  index read (`index_state`: a copy of `.git/index`, `ls-files --stage`, `write-tree` on temp indexes). An unchanged
  index writes no new tree object after the first. No read at all in a repository with no such entry.
- (2026-09-26) A preview of a rebase or checkout with edits to set aside spawns two more git commands (three for a
  rebase) and writes loose, unreachable objects (§8). A clean tree spawns none.

## Amendment 2026-09-26 (review `docs/reviews/m6-final-fixes-74.md` M1–M3; ticket 84)

### §5 amended — `apply` says where the edits are; the app only words it
The review found three paths (`graft-ops/src/apply/set_aside.rs:444`, `:122-163`, `:445`) where the status line said
`your edits are in stash@{n} · S p puts them back` while the edits were already in the tree: the app read the stash
list again (`crates/graft/src/jobs/operate.rs:344-360`) and took "still listed" for "not back". Only `apply` knows
whether the put-back ran. So every `ApplyError` a set-aside ends in, other than `Clashed`, carries
`graft_ops::EditsAt`, and the app reads no list to guess:

| `EditsAt` | When | The line's clause |
|---|---|---|
| `Stash { n }` | step 6 failed, nothing unmerged (§5 last row) | `your edits are in stash@{n} · S p puts them back` |
| `Back { copy: Some(n) }` | the put-back ran clean; the drop failed | `your edits are back · a copy stays in stash@{n}` |
| `Back { copy: None }` | the put-back ran clean; the copy was not found to drop | `your edits are back · a copy may still be in S` |
| `Backup { copy }` | stored (3), then the reset (4) failed or stopped | `u restores the backup`, + ` · a copy is in stash@{n}` when `Some(n)` |

The operation's own failure (the error's Display) is the line's reason and comes first; the clause is the part kept
whole at narrow widths (`fit_status_line`'s keys). `S p` is named only when the edits are not in the tree.

(2026-09-27, 30c) Recovery says where an interrupted set-aside's edits are (Recovered.set_aside); the relaunch notice words it. A later launch keeps "edits in stash@{n}".

### §3 amended — a set-aside close names what it dropped
The set-aside close reads as the pop's does: `every file done · set-aside copy dropped ✓ · u brings it back`.
Ticket 74's `every file done · your edits are back ✓` is withdrawn: after `o branch` in every block his edits are not
back, and the line said they were. Taking the branch side everywhere and pressing `⏎` still drops the copy, as a pop
taken `yours` everywhere does: the edits are then one `u` away (the drop's `StashStore` inverse puts `S` back in the
list; `S` shows it), not lost, and the close line says `u brings it back`. **Rejected:** keep `S` when any block took
the branch side — a stash he would forget (A2, the reason §3 drops at all), and a rule that depends on takes he cannot
see from the close.

### §8 (new) — the preview forecasts the put-back
M5's promise is "you see the result before you confirm". A box that said `no conflicts expected` over a put-back that
clashes (review M2: `graft-ops/src/preview/kinds.rs:351-376`, a checkout never forecasts) broke it. So `plan`, only
when `set_aside` is `Some`, forecasts step 6:
1. `git stash create` of the tree now → `S'` (objects only), run on a temporary copy of the index, as `index_state`
   does, so `.git/index` is not rewritten.
2. `landed`: a checkout's target commit; a rebase's `git merge-tree --write-tree HEAD <upstream>` tree — one spawn
   standing for the replayed tree (m5 hard choice 5 refused chaining the replay). When that merge conflicts, or the
   replay forecast is not `Clean`, the put-back is `NotChecked("the rebase may stop first")`.
3. `git merge-tree --write-tree --merge-base=HEAD <landed> <S'>`: the three-way merge `stash apply` does (base `S^1` =
   `HEAD`). Exit 0 → `Clean`; conflicts → `Likely { paths }`; anything else → `NotChecked(<reason>)` — a hole, never
   a clean bill, and never a failed preview.

Result: `PreviewPlan::put_back: Option<ConflictForecast>`, `Some` exactly when `set_aside` is. It is a forecast
(`may`), as every forecast is: the approximation for a rebase, and edits made between the preview and `y`, can
differ. Cost: two spawns for a checkout, three for a rebase, on the preview job, checked for `stop` between them; no
ref, index or file written. Words: `docs/design-notes/m6-stash-put-back.md`, amendment 2026-09-26.

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
apply. Pinned by `the_rm_cached_bytes_are_never_only_on_disk` (and `undo_of_the_switch_keeps_the_rm_cached`); the
round trips by `rebase_round_trips_an_rm_cached`, `checkout_round_trips_an_rm_cached`,
`rebase_round_trips_index_only_content`, `checkout_round_trips_index_only_content`,
`rebase_round_trips_a_staged_mode_only_change`, `checkout_round_trips_a_staged_mode_only_change`,
`rebase_round_trips_a_rename`, `checkout_round_trips_a_rename`, `rebase_round_trips_a_staged_new_file` and
`checkout_round_trips_a_staged_new_file` (`crates/graft-ops/tests/it/set_aside.rs`).

### Absorb's put-back (review m7-06b M3, "What the ADR-0045 amendment must state")
1. **Scope.** Absorb that **went through** — `Operation::Rewrite` with `index_base: Some`, and `HEAD^{tree}` equal to
   the base's tree at close — is one case of the rule above: its input to `stash apply --index` is `S` rebuilt on
   the base instead of `S`.
2. **Mechanism, as built.** `index_base` = old `HEAD` + every placed hunk, a commit with no ref. It is pinned by the
   entry (`record.rs`) and saved in `plan.json` (`SavedPlan::index_base`, `#[serde(default)]`: a plan saved before
   06b reads as `None`). At step 6, `S` is rebuilt (objects only): working tree `S^{tree}`, parents `base` and a
   commit of `S^2^{tree}` on `base` (`apply/set_aside/rebuild.rs`, `from_index_base`). Then `stash apply --index` of
   the rebuilt stash. `S` stays the listed copy and is dropped by id.
3. **The guarantee.** Index == the old index, byte for byte. The working tree is exact, including an edit made on past
   a placed hunk. It cannot clash, because ours is the merge base.
4. **Rejected: merging with the old index as base.** It removes every stay from the working tree (review m7-06b,
   §"What the absorb promise holds now", its probe table).
5. **Failed, aborted, or resumed onto a different tree** (review m7-06b C2). `S` itself goes back with `--index`, by
   the rule above — not the plain put-back. Aborted, `HEAD^{tree}` is `S^1^{tree}`, so it is exact: the index-only
   lines are still staged (`abort_after_absorb_stops_keeps_index_only_lines`). Resumed onto a tree the plan did not
   name, git usually cannot reinstate the index (a placed hunk is already in `HEAD`) and writes nothing: the plain
   apply runs and `StagingNotKept` names the files; `u` restores the split
   (`absorb_stopped_then_continued_elsewhere_says_the_staging_went`). `r` and `R`, which pass `index_base: None`,
   are the same rule (`every_set_aside_kind_keeps_the_index`).
6. **The preview.** `SetAside.staged` was 0 for every kind, so it was no longer true only on absorb's went-through
   path. (30c removed the field, and its test with it.)
7. **§7 proof rows, absorb.** An index-only line (`an_index_only_stay_survives_absorb`), an index-only mode
   (`an_index_only_mode_change_survives_absorb`), a rename (`a_staged_rename_stays_whole`), edited on
   (`absorb_after_editing_on_leaves_no_markers`), stopped then `c` (`absorb_stopped_then_continued_as_planned_keeps_the_stay`,
   `absorb_stopped_then_continued_elsewhere_says_the_staging_went`), stopped then `a`
   (`abort_after_absorb_stops_keeps_index_only_lines`, `undo_after_absorb_stops_then_aborts`), each with its undo row
   (`crates/graft-ops/tests/it/absorb_staged.rs`).

ADR-0049 is unchanged: its guard refuses a write over bytes nothing else holds, and §5 above shows a set-aside's
bytes always have a holder.

(2026-09-27, 30c) An added path is named when its staged content differs from the tree's (AM, AD). SetAside.staged removed.
AD caveat, measured: `git stash create` keeps a staged-then-deleted file's staged blob in `S`'s tree, so the plain
apply re-adds it `A ` and it is not named (`put_back_leaves_a_staged_new_file_deleted_since_unnamed`); the deletion
is what the stash itself does not keep.
