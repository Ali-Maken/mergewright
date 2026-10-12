# Review: m8-polish-and-ship ticket 48 — Generation table and the move to Mergewright

Commits: `04d295a2` (Part A), `5c77dbd0` (Part B), merged at `f11ac645`. Ticket
`.scratch/m8-polish-and-ship/issues/48-mergewright-names-table-and-migration.md`, ADR-0057 Amendment 2026-10-11.

**What I verified, and how.** I read the source and tests at `f11ac645`. **I ran no cargo**: the conductor's
dispatch forbade it because the gates and a builder were running. Gate results, timings, the heavy tier and the release
perf run are therefore **not observed by this review**. The builder reports nextest 6400/6400 for Part B, and I have
not checked that. Every finding below comes from reading the code. None was reproduced.

**Focus asked:** can any path lose a ref, backup, journal entry or kept-aside edit?
**Answer:** not on any single-process path I traced. The order of copy, read back, switch and delete holds. Each step
fails safe on its own (`rename.rs:274-299`, `refs.rs:96-131`, `trees.rs:117-154`, `trees.rs:195-234`). Two paths can
delete data **unread**, and both need an older build running at the same moment (Major 1). Two windows the ticket
said to inject at have no test (Major 2).

## Spot-check of acceptance ticks

- **Part A: checked 5, held 5.**
  - The three `on_disk::tests` named exist (`on_disk/tests.rs:8,61,85`).
  - `every_prefix_internal` exists (`ref_name/tests.rs:143`).
  - The grep `OnDiskNames|Names::(New|Old)|init_old_names` gives 0 at `f11ac645`. I re-ran it.
- **Part B: checked 14 (every box), held 14, two with a caveat.** No false tick.
  - The 6 `generations.rs` tests each assert what their tick says, including `fsck` and `u` byte-exact through
    `setup.rs:85-92`.
  - The other 8 also hold: `clean_marker_spawns_nothing` (`it/rename.rs:284-307`), `every_generations_trailer_reads`,
    the m5 golden, `an_old_migrate_row_keeps_its_target`, the config test, the status-words test, the literal test and
    the fsck box.
  - Caveat 1: the config tick names `every_earlier_folder_moves_once`, which covers only the both-present case. The
    "alone" cases are in `each_earlier_folder_alone_moves` (`config/folder/tests.rs:155`), named in NOTES but not in
    the tick. Minor 6.
  - Caveat 2: the literal tick says "no **test** literal", but `no_old_dir_literal` scans production files only
    (`rename_opens.rs:19-39` skips `tests/`, `tests.rs`, `*_tests.rs`). I grepped the test literals myself. What is
    left is the stated exceptions plus doc comments (Minor 5), so the claim holds in substance.

## Critical

None.

## Major

### 1. Scratch is decided by inferring from markers and never re-checked, so a live older build's writes can be removed unread

The amendment's §5 exception deletes "scratch" (a generation newer than the one in force, with no marker) **without
comparing it**. Which generation counts as scratch is not recorded anywhere. It is worked out again from which markers
happen to exist on disk, at a moment when the move itself has already removed some of them.

**Trigger A: a stray pass that dies part-way after the switch.**
- `after_the_switch` puts the strays in `CURRENT.older()` order, newest first: `[Tributary, Graft]`
  (`scratch.rs:25-29`, `on_disk.rs:127-133`).
- `settle_strays` then removes each generation's machinery inside one loop (`strays.rs:75-80`). Tributary's
  `migrated` marker goes **before** Graft's `remove_empty(refs/graft)` and `remove_machinery(.git/graft)`, and before
  `marker::write(Clean)` (`rename.rs:297`, `rename.rs:395`).
- The doc at `strays.rs:39-41` says "the marker last of all, so a pass cut short leaves the marker that says which
  generation the move came from". That holds for one generation, not across the loop.
- If anything fails or is killed in that window, the next open finds `switched`. `was_in_force` then finds no
  Tributary marker and answers **Graft** (`scratch.rs:35-49`).
- `after_the_switch(Graft)` then classes Tributary as scratch, and `remove` deletes `refs/tributary/**` and every
  `.git/…/tributary/` whole, with no compare (`scratch.rs:84-105`).
- Nothing of the move's own is lost: the pass already settled every Tributary ref and file.
- What is lost: §6 says a Tributary build open on the names that were in force holds no lock and keeps writing.
  Anything it wrote in that interval is removed unread, where §6 promises "caught by the stray pass on the next open":
  - backup refs, which are his kept-aside edits;
  - journal lines.

**Trigger B: an older build that starts its own move during this one.**
- `lock_alone` skips any earlier generation whose lock file does not exist yet (`locks.rs:27-31`, "no build of it is
  open on its names").
- Take a Graft repository that no Tributary build has opened. A Tributary build launched while this move runs creates
  `tributary/lock`, takes it, and moves Graft → Tributary alongside us. That build's code is fixed: it knows nothing of
  Mergewright's lock.
- Our scratch list was fixed at copy time from `copied.from` (`rename.rs:293`, computed at `rename.rs:330`).
- `remove` then deletes the Tributary build's refs and folder, including its fresh marker and anything it has written
  since its switch, unread (`scratch.rs:88-103`).
- If our stray pass then fails, the marker stays `switched`. Its Graft-ref deletes can fail when the other build has
  already deleted those refs (`refs.rs:177`, expected-value deletes). In that case every later open repeats the scratch
  removal against what the still-open Tributary window writes.

**Why it matters.** This is the one class of path where the move deletes the user's data without reading it. It
breaks the module's own rule, "never deleted unread" (`strays.rs:2`). Both triggers need a Tributary binary running at
the same moment as a crash or race, so the odds are low. The loss cannot be undone.

**Suggested fix (cause, not symptom).**
- Record the source generation in the `switched` marker (for example `switched tributary\n`). `clean_up` then reads
  `was` from the marker instead of inferring it. `marker.rs:21-26, 37-39` are the only places that spell the line, and
  no Mergewright build has shipped, so nothing older must be read.
- Before `remove` deletes a scratch generation, check again that its marker is still absent. If one appeared, send
  that generation through the stray pass instead.
- Separately, remove every generation's marker in a second loop after all the others' machinery, so the doc at
  `strays.rs:39-41` is true.

### 2. The new crash windows have no injection test, against the ticket's own build line

The ticket's `Build with` says each new window gets a test that injects at that step. The only `CleanedUp` integration
test is `switched_marker_cleans_leftovers` (`it/rename.rs:242-282`). It covers a Graft-source move with no Tributary
names present. `git grep CleanedUp` finds no other test. `was_in_force` has no test of any kind
(`git grep was_in_force` → `rename.rs`, `scratch.rs` only).

Untested:
- (a) Mergewright `switched` after a **Tributary** source: `was_in_force` → Tributary, so strays go over both.
- (b) Mergewright `switched` after a Graft source **with Tributary scratch**: the scratch is removed in `clean_up`, not
  in `migrate`.
- (c) A stray pass cut short after Tributary's machinery is gone (Major 1, trigger A).

`switched_tributary_with_graft_leftovers` is a **Tributary**-`switched` *source*, not a Mergewright `switched`
clean-up, so it does not cover these.

**Why it matters.** These are the windows where the code decides between "delete unread" and "compare". Major 1 lives
in one of them, and no test would have caught it.

**Suggested fix:** one test per window in `it/rename/generations.rs`. Each builds the on-disk state by hand, as
`as_tributary` does, writes the Mergewright marker as `switched`, and asserts on `for-each-ref`, the folders and
`fsck`. For (c), the test also writes a `refs/tributary/backup/…` ref after Tributary's marker is gone, and asserts it
is carried, not deleted.

## Minor

1. **The move unlinks lock files it holds.**
   - Where: `remove_machinery` deletes each earlier generation's `lock` (`scratch.rs:55-62, 67-79`, called from
     `strays.rs:79`), and `remove` deletes `.git/tributary/` whole (`scratch.rs:97`). Both happen while `lock_alone`
     holds those files exclusive (`locks.rs:33-34`).
   - Why it matters: a Tributary build that blocked in its `stay_old` shared wait keeps an orphaned inode. Any later
     Tributary build makes a fresh `lock` and is not excluded by it. I found no loss, because whatever they write
     becomes strays, but the lock no longer means what §6 says.
   - Fix: delete a lock file only after its `FileLock` is dropped, or leave the lock files alone. They are machinery,
     never data.

2. **The stray pass can get stuck for good on two generations' refs with the same rest.**
   - Where: `carry_or_set_aside` maps each old ref to `wanted` (`refs.rs:156`). When the new name is absent, it adds
     `create {wanted}` for **each** old ref (`refs.rs:172-175`).
   - When it happens: `refs/tributary/x` and `refs/graft/x` both exist with no `refs/mergewright/x`. For example, a
     Tributary binary re-moves Graft leftovers on a `clean` repository and dies between its switch and its deletes.
   - What follows: the transaction gets two updates for one ref, and git refuses the whole transaction. Every open
     after that is `LeftOld` with the same error. Nothing is lost (it is atomic), but nothing recovers.
   - Fix: when a target repeats, carry the first and delete the second if its value is equal. Otherwise keep it aside
     under `<gen dir>/`.
   - The ticket's tests never put both generations' refs under one name in a `clean` pass.

3. **Question: should `check_journals` read kept-aside journals?**
   - A Tributary repository's `.git/tributary/older-version/<n>/graft/undo-journal.jsonl` (26r's kept-aside file) is
     copied into `.git/mergewright/older-version/…`.
   - `check_journals` filters by file name anywhere in the tree (`journals.rs:16-18`), so it parses that file and
     requires every backup it names to exist under `refs/mergewright/` (`journals.rs:30-36`).
   - A kept-aside journal naming a backup that was itself kept aside under `older-version/` would fail `ReadBack`
     **before the switch, on every launch**: the repository is stuck `KeptOld` for good. That is safe, but there is no
     way out.
   - I could not confirm such a journal can exist, so this is filed as a question (rule 7).

4. **`stay_old` reports the names from the handle, not from disk** (`rename.rs:235`).
   - On the copy-failure path (`rename.rs:276-283`), `copy_everything` has just re-read `from` under the lock
     (`rename.rs:330`) because "the handle's answer from open may be stale". `stay_old` then answers with the stale one.
   - Only an older build moving names in between makes them differ, and its writes become strays.
   - Fix: return the `from` already read, or call `names_in_force` again.

5. **Stale doc comments still name only `refs/graft/` and `refs/tributary/` as the prefixes.**
   - In production code:
     - `graft-ops/src/snapshot.rs:171-172` ("`refs/graft/` or `refs/tributary/`");
     - `graft-git/src/history_reads/unreached.rs:26-27`.
   - In tests:
     - `it/aborted_fetch.rs:237`;
     - `it/journal_list_oracle.rs:301`;
     - `it/rename/review.rs:16`;
     - `m3_walk.rs:207,364`;
     - `m7_walk/rewrite_editor.rs:85`;
     - `radar_bookkeeping.rs:3`;
     - `repo_truth.rs:17`.
   - Fix: "the prefix in force (`Generation::ref_prefix`)".

6. **Two tick texts name less than they claim** (see the spot-check caveats).
   - The config tick should also name `each_earlier_folder_alone_moves`.
   - The literal tick should say the test covers production code, and that test literals were checked by grep.

7. **`<config_root>/tributary.lock` is left in `~/.config` for good.** The config move locks `mergewright.lock`
   (`config/folder.rs:93`), and nothing removes the earlier generations' `<dir>.lock` files. This is cosmetic: a stray
   file in his config root. Fix: remove each earlier `<dir>.lock` after its folder is settled, without taking it.

## Other checks

- **Rule 3:** no new swallowed errors.
  - Each `NotFound` pass-over carries its "safe because" comment (`scratch.rs:73-74, 99-100`, `strays.rs:130`).
  - `load.rs`'s `try_exists` guard treats an error as "not there", with the reason written beside it.
  - `journals.rs:28` `Err(_) => continue` dates from before this ticket and is commented.
- **unwrap/expect:** none added outside tests (grep of the added lines).
- **Architecture:** graft-core stays pure: the table is const fns with no free fns, as ADR-0007 requires. graft-git
  reads the markers with `stat` only. The move stays in graft-ops. graft-fixtures does not depend on graft-core; its
  literal is pinned by `rename_status.rs`.
- **Readability:**
  - `rename.rs` is at 400 lines and `trees.rs` at 395, both right at the cap.
  - `settle_strays` (`strays.rs:42-93`) mixes the choice of pass number with the pass itself. Extracting
    `next_pass(kept_dir, new_refs)` would make it read top to bottom.
  - Not filed.
- **Feature pipeline:** this is a rename and migration ticket with no user-facing screen or journey. The prior-art
  and journeys steps do not apply.
