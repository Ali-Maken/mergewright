# Review: M8 ticket 26, rename on-disk migration (Parts A and B)

Range `1940990d..26175b58`, read-only at fixed hash `26175b58` (HEAD `8076b8d3` differs only by `docs/log/m8.md`).
Ticket `.scratch/m8-polish-and-ship/issues/26-rename-on-disk-migration.md`. Skill: `code-review`.

**Counts: Critical 0 · Major 3 · Minor 11 · Questions 3.**

## What I verified and how

- Read the ticket, the design note's rename rows (Lands table, Hard-to-reverse choice 3), the four MAP diffs, then the
  diff crate by crate: `graft-ops/src/rename{,/marker,/refs,/trees,/record}.rs`, `graft-git/src/file_lock.rs`,
  `repo/{mod,shared}.rs`, journal readers, `graft/src/{rename,repos}.rs`, `config/{folder,load}.rs`, fixtures.
- **Reproduced Major 1 against real git**, using a scratch crate outside the repo (path dependencies on graft-ops
  and graft-git, with `settle_names` on a real repository). Output is quoted under Major 1.
- Crash and lock order: read line by line against the ticket's order. The kill after the create transaction is
  covered by a real SIGKILL (`tests/it/rename/shim.rs`). **Not observed by me:** power loss, two real processes
  racing, and a kill at any other step.
- Colour: none involved. Captures: none changed (`docs/shots/` untouched). The status-row words have no capture;
  they are judged only through `RepoStore::rename_words`.

## Gates (own `CARGO_TARGET_DIR`, run one at a time)

**Load warning.** Another agent's `cargo nextest` was running in `.claude/worktrees/agent-aa30ed…` during my runs.
Load average was 13–14. Every time below is inflated by that, and so is every perf verdict.

| Gate | Result | Wall | Peak RSS |
|---|---|---|---|
| `cargo fmt --check` | pass | — | — |
| `cargo clippy --all-targets -- -D warnings` | pass | 31 s (cold check) | 1.0 GB |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass | 13 s | — |
| `cargo nextest run --workspace` | **6273/6273**, 15 skipped | 847 s (rerun with per-test times: 1048 s) | 249 MiB (nextest) |
| `cargo test --doc --workspace` | pass (3 doctests) | 3.4 s | — |
| heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **4 red**: `held_key::held_j_stops_on_release_pixel{,_free}`, `…pixel_deflate`, `ghostty_shot::fix_scroll_ghostty_png` | 903 s | 1.13 GB |
| release perf `… frames:: heavy::budgets_hold --test-threads=1` | **14 of 21 red** | 283 s | 1.0 GB |
| `target/` (mine, deleted after) | 9.9 GB | | |

- I used `nextest` for the fast suite instead of `cargo test`: it is the CLAUDE.md gate, and it is the only one that
  gives per-test times.
- **The heavy reds are all known.** `KNOWN_GAPS.md` has M8/heavy-tier-reds (ghostty png and held_key) and
  M7/46-kitty-no-deflate. None of them touches 26's code.
- **The release perf reds cannot be judged at load 14.** Frame p99 was 19–30 ms against the 16 ms budget, with p50
  at 6–13 ms. First paint was 150–289 ms against 150 ms. Pixel held-`j` peak RSS was 308 MiB against 300 MiB.
  - 26's only hot-path change is `is_graft_internal` checking two prefixes instead of one. That is not plausibly
    milliseconds.
  - The synthetic fixtures already carried the marker before the perf run (the heavy tier had written it at 14:51;
    see Minor 9), so no settle was inside a first-paint measurement.
  - Rerun on a quiet machine before M8 closes, as the gap already says.
- **Five slowest tests (nextest, loaded).** All are outside 26, and all are far over 7.1's per-test expectation:
  - `graft-ui graph_screen::hints::tests::the_bar_never_draws_a_fragment_of_itself`: 117 s
  - `…::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn`: 115 s
  - `…::no_hint_loses_ground_as_the_room_for_it_grows`: 110 s
  - `…::the_bare_keys_are_a_tail_not_a_scatter`: 103 s
  - `…::a_words_only_bar_draws_no_bare_key_and_ends_with_its_doorway`: 63 s
- No fuzz targets or benchmarks touch this group, so none were run.

## Acceptance ticks: 17 of 19 checked, 17 held

- **Checked by reading the test body** (16): boxes 1–11, 14, 15, 17, 18 and 19. Box 16 (config) was checked by
  reading its asserts. All 17 held, with two caveats, both under Minor 6:
  - Box 1 says "M7 walk fixtures", but the test builds its own: `old_repo(17, 2)` plus a stage.
  - Box 3 names `kept_old_words`, which asserts the **Busy** words. The read-back words are asserted by
    `rename_status.rs::every_outcome_has_its_words_or_none`. The behaviour holds; the named test is the wrong one.
- **Not checked** (2): box 12 (trailer) and box 13 (both prefixes). I confirmed only that the tests exist and the
  suite passes.
- **No false tick.**

## Critical

None.

## Major

### M1. A move that fails before the switch leaves its copies behind, and they block every later retry

**Where**
- `crates/graft-ops/src/rename.rs:281-287`: a copy failure goes to `stay_old`, and nothing removes the files or refs
  this attempt already created.
- `crates/graft-ops/src/rename/trees.rs:141-148`: a new file already there with different bytes is refused.
- `crates/graft-ops/src/rename/refs.rs:62-66`: the same rule for refs.
- `rename.rs:95-97` promises "the next open tries again".

**What happens.** The run then continues on the old names. It writes `.git/graft/layout` on a pane resize and
appends `.git/graft/undo-journal.jsonl` on any operation. From then on, the leftover copy differs from its original,
so every later settle stops at it.

**Reproduced** (real git, scratch crate):
1. Setup: a linked worktree whose `graft/` holds one symlink, and a main `graft/layout`.
2. The first settle fails on the symlink. The copy of `.git/tributary/layout` exists.
3. I removed the symlink and rewrote `layout`.
4. Settles 2 and 3 both returned:

```
second: KeptOld(Files { path: ".../.git/tributary/layout", words: "already there with other bytes; left for you to compare" }) names=Old
third:  KeptOld(Files { path: ".../.git/tributary/layout", words: "already there with other bytes; left for you to compare" }) names=Old
```

**Why it matters.**
- Any transient failure becomes permanent: EIO, a permission problem on one worktree's folder, disk full, or a
  file the walk will not copy.
- The app stays on the old names for good.
- Every launch's status row tells him to compare files inside `.git` that he never made.

The builder's note ("A new ref left at a different value … Safe, but sticky") saw the ref half of this but not the
file half, where it is the common case.

**Fix.** Before the switch, nothing under `<common>/tributary/` (except `lock`) and nothing under `refs/tributary/`
has ever been read as the user's: the marker is absent, so no run used the new names.
- On any failure in steps 2–4, delete what this attempt created. Record the files it wrote, and delete the refs it
  made at their created values.
- Alternatively, when the marker is absent, let `copy_one` overwrite a copy that differs. Keep "refuse" for the
  `switched` state.

**Test.** A copy that fails part-way, then the old journal grows, then the next settle gives `Migrated`.

### M2. An older build still running on the repository is not kept out, and what it writes after the move is never read

**Where**
- `rename.rs:232-237` and `242-258`: the lock is taken only by builds that have this code. A build from before 26
  takes none.
- `rename.rs:155`: with the marker at `clean`, settle returns `Done` after one read. A `.git/graft/` or
  `refs/graft/*` that the older build recreates afterwards is never looked at again.
  - Its journal entries are invisible to `u` and J.
  - Its new backups are hidden from the graph by the two-prefix filter (`ref_name.rs` `is_graft_internal`).
- Inside the move, `remove_old` (`rename.rs:306`, then `trees.rs:107-109` `remove_dir_all`) deletes whatever the
  older build appended to the old folder between step 4 (copy) and step 8 (remove).

**This applies to the user's machine now.** During this review, `ps` showed a release `graft`, started 2 days ago
(so built before 26), open on one of his work repositories. That is exactly the state in which he will first launch
the renamed build.

**Why it matters.**
- The conductor note accepted that "an older graft binary afterwards sees no undo history in a migrated repo".
- It did not decide the reverse case: the new build silently never seeing the old window's operations.
- It did not decide the deletion window either.
- The window is milliseconds and needs an operation in the old window at that moment. **I reasoned this from the
  lines above and did not reproduce it.** The stranding after the move needs no race at all.

**Fix** (needs a decision, so this is also Question 1):
- (a) On `clean`, add one more `stat` of `<common>/graft`. That is still no spawn, so the zero-spawn tick holds. If
  it exists, say on the status row `rename: an older graft wrote under the old names · close it and reopen`, and
  rerun the copy, refusing on conflict.
- (b) And/or: before step 8, compare each old file's length and modification time with what step 4 copied, and keep
  any that changed.

### M3. `clean_up` runs its body in an immediately-invoked closure

**Where:** `crates/graft-ops/src/rename.rs:366-390`, `let cleaned = (|| { … })();`, used only to get `?`.

**Why it matters.** This is clever code where plain code would do (PROMPT.md 6.2). A reader has to work out that
the closure exists only for early return. Readability findings are Major per the review brief.

**Fix.** Write a plain `fn clean_up_steps(repo) -> Result<(usize, usize), MigrateError>` and have `clean_up` match
on its result, the same shape as `copy_everything`.

## Minor

1. **`clean_up` deletes old folders without checking that each file has a new copy.**
   - Where: `rename.rs:374-379`.
   - The ticket says: "Never deletes an old ref or file before its new copy reads back the same". Under `switched`,
     only an older build (M2) can have written there, and those writes are deleted unread.
   - Fix: before `remove`, require each old file's new path to exist. Leave the ones that don't, and say so.
2. **Directory entries are never fsynced.**
   - Where: `trees.rs:152-157` syncs file data but not the folders `create_dir_all` makes; `marker.rs:60-64` syncs
     only `tributary/`.
   - After a power loss (not a process kill), the marker can survive while a copied file's directory entry does
     not, for example `worktrees/X/tributary/…` or `rewrite/<id>/plan.json`, and step 8 has already removed the
     original.
   - This is reasoning, not reproduced. APFS and ext4 usually order it safely.
   - Fix: fsync each created folder and its parent before writing `switched`.
3. **A failed removal of the old config folder loses its reason.**
   - Where: `config/folder.rs:74-79`, `Err(_) => BothPresent`.
   - He reads "still there beside the new folder" with no why. That is rule 3 (the comment explains the outcome,
     not why dropping the error is safe).
   - Fix: carry the `io::Error` words into the warning.
4. **Two launches at once race the config move.**
   - Where: `config/folder.rs:67-71`. The second launch deletes the first's `tributary.moving` mid-copy.
   - The loser runs on `graft/` for its life. Its `recent`/`launches` writes recreate the old folder, which gives a
     permanent BothPresent warning (Question 2).
   - Fix: hold an flock on `<config_root>/tributary.lock` around the move.
5. **The "nothing to say" journal check can never fail.**
   - Where: `trees.rs:164-170`. It parses the journal and compares entries only after a byte-equality check has
     passed, so it is tautological.
   - `.ok()` turns a parse error into `None` on both sides, so an unparseable copy passes too.
   - The ticket's "parsed through the real reader and compared entry by entry" is met in letter only.
   - Fix: parse the copy with `map_entries(New)` and check that each `backup_ref` it names exists under
     `refs/tributary/` (the refs were read back in step 3). Or delete the check and say why bytes suffice.
6. **Tick wording.** Box 3 names `kept_old_words` for the read-back words; it should name
   `every_outcome_has_its_words_or_none`. Box 1 says "M7 walk fixtures" but builds its own. Both behaviours hold.
7. **The ticket's toolchain stop was not taken.**
   - The ticket said: "toolchain predates 1.89 → stop and report, no new dependency".
   - The builder enabled nix's `fs` feature instead (`graft-git/Cargo.toml`, the `nix` line) and wrote
     `file_lock.rs`.
   - I checked: `fs = []` in nix 0.29, so no new crate. The intent is kept but the instruction was not. The
     conductor should ratify it or not.
8. **Old paths left in docs.**
   - `crates/graft/MAP.md`, the `edit_message_in` line: `<git_dir>/graft/COMMIT_EDITMSG`.
   - Doc comments: `graft/src/update/detail/state.rs:312`, `update/events/command.rs:90`, `:335`.
   - `docs/design/06-undo-journal-design.md:21-31`.
   - Confirm all of these are on ticket 27's list. If they are not, they are 26's leftovers.
9. **Heavy-tier fixtures are not built with the marker, so tests still write into shared caches.**
   - Where: `graft-fixtures/src/synthetic.rs:201` (`build`) and `team.rs:147` have no `names_settled`.
   - Observed in my target dir: `fixtures/synthetic-100000-*/.git/tributary/{lock,migrated}`, written at 14:51 by
     the heavy run, and the same for `team-17700`.
   - This is the copy-versus-write race the builder fixed for the corpus, still open for these caches. The first
     heavy test on each fixture also pays a settle (one spawn and two `sync_all`) inside its first-paint
     measurement, though only on a fresh target dir.
   - Fix: call the same marker writer in `synthetic::build` and `team::build`. Their cache keys hash their sources,
     so they rebuild once.
10. **Unverified items are not in `KNOWN_GAPS.md`.** The builder's "Not verified" list has no gap rows:
    - two real processes racing
    - kill points other than after the create transaction
    - the status row on a real first frame (no capture)

    The conductor should file them as **unverified**.
11. **The Part B NOTES header says `builder-opus`.** The dispatch and log lines say `builder-fable`, and the commit
    trailer says Opus. Correct the NOTES so the ticket agrees with the log, which `AUTHORSHIP.md` is generated from.

## Questions (no line proves a defect)

1. **M2 policy.** Should the renamed build detect an older build's writes under the old names after the move, and
   what does the user see? (a) a status row plus a re-run of the copy; (b) a release note only; (c) accepted as is.
2. **Config "warned once".** `folder.rs:64-66` warns on every launch while both folders exist, and the old one is
   never removed for him. Is it once per launch or once ever? If an older build recreates `~/.config/graft/`, every
   start shows it.
3. **Pipeline.** The design note has no Prior-art row for the on-disk and config-folder move. There is no journey
   (it is invisible when it works), so I have not filed it as Major. Precedent exists, for example config-folder
   moves in other tools and how they kept the old path readable. Does the conductor want a row?

The builder's own questions are left to the usability walk: the silent `Migrate` J row with no `not undoable` mark,
a pending `Migrate` after a crash in `record.rs:71-80` reading as an interrupted operation, and the wall-clock time
under `GRAFT_NOW`. I agree they belong there.

## Checked and fine

- Order of steps, `rename.rs:270-317`: lock, then create, then read back, then copy and read back, then `switched`,
  then the journal entry, then delete at the expected old values, then remove, then `clean`. It matches the ticket.
  Nothing old is deleted before `switched`, and the marker write is atomic (temp file, sync, rename, folder sync).
- `stay_old` re-reads the marker after waiting on the shared lock, and `open_settled` reopens when the names change.
  Exactly one production open site exists (`rename_opens::every_open_settles_first`).
- Journal mapping happens at read in `open_as` and in the three raw readers. The golden file is byte-identical
  (`rename/tests.rs`).
- `undoable_kind` excludes `Migrate`. `next_redo` and `plan_revert_to` step over it
  (`journal/tests.rs::migrate_is_transparent_to_undo`).
- Architecture: `on_disk` lives in core, which stays pure. `file_lock` is in git, and ops depends on git, which is
  allowed. graft-fixtures pins the literal by test rather than depending on core.
- No `unwrap` or `expect` outside tests. One `.ok()`, which is Minor 5.
