# Review: m3-working-tree-undo-and-stash/02 — ops-model (signature ticket)

Reviewer: Fable (`reviewer`), 2026-09-19. Range `25e9358..d302765` (the
conductor's `bc601d6` skipped). Read: the spec incl. its `## NOTES`, as-built,
design note, ADR-0009/0010/0031, DESIGN.md 06 §6.1/6.2/6.4/6.5/6.7, the three
MAP.md files, then the diff crate by crate (core, git, ops: operation → journal
→ undo → tests). Opened no source file whole. Claims marked *verified* were run
against git 2.54.0 in a throwaway repo under `target/review-ops-model-repo/`
(untracked). The builder's own scripts (`target/verify-ops-model*.sh`) were
read, not re-run.

**Counts: 2 Critical, 7 Major, 8 Minor.**

## Gates

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` (fast tier) | all green (467+97+72+49+36+29+… tests, 0 failed, 9 ignored); **25.6 s wall** (warm cache; group 01's 83 s included a compile), 19.7 s user + 11.2 s sys; peak RSS **343 MB** |
| `cargo test --features heavy-tests -- --test-threads=2` | **1334 passed, 0 failed, 9 ignored**; 178.2 s wall; peak RSS 591 MB. The debug `many_refs` guard that failed in the 01 review passed this run |
| release perf run (`frames:: heavy::budgets_hold`, `--test-threads=1`) | **13 passed, 0 failed**; 82.9 s wall; peak RSS 468 MB; numbers below |
| fuzz / benches | none exist in the workspace |
| `unwrap()`/`expect()`/`let _ =`/`unwrap_or_default`/empty `Err(_)` in the group's non-test code | none. `EntryId::new`'s `unwrap_or(0)` is a documented, tested clamp; `commit::subject`'s `unwrap_or("")` is the empty-subject value `check` then refuses |

Five slowest fast-tier tests (single-threaded, wall ms): `diff_fixture::parse_unified_matches_numstat_for_every_shape` 7 464; `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 5 907; `corpus::two_builds_produce_the_same_object_ids` 2 732; `diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 1 522; `corpus::a_second_build_in_the_same_place_is_a_no_op` 1 018. This group's 47 tests are pure and finish in under 10 ms together. `target/`: **16 GB** (was 12 GB at the 01 review; the release perf build is most of the growth).

Release perf: `first_paint_us` 12 454 / 14 405 / 76 199 / 93 282 (100k with/without commit-graph, plain/realistic) and 58 857 with 50k refs — budget 150 ms. Frame p99: Pixel 50k refs 6 044 us (max 9 846), Unicode 50k refs 5 367 us, detail pane 5 076 us, diff scroll 4 930 us — budget 16 ms. In-test peak RSS 30.6 / 95.1 / 35.5 / 101.4 MB, 112.4 MB with 50k refs — budget 300 MB. `bottom_ms` 51–72 with a commit-graph, 467–525 without.

## What this group is, and where it stands

The shape is right: one enum, four properties, state-based journal, generic
inverse plus a stash reflog step, cursor read off the entries. `check`/`plan`
tables are exact and every `Refusal` is reachable in a test. Round trip,
cursor, `state_at`, per-kind step vectors and drift are all tested. The
`refs/stash` reasoning (deviation 2) is correct and the tests prove the
*plan* never moves that ref.

What is not right is the error path — the one the promise rests on. Two
places quietly do the wrong thing after a failure, both in this group's own
code and both cheap to fix. Everything else is a contract gap for group 03 or
a check that lets git fail where a refusal was due.

## Critical

### C1. A failed operation that changed nothing shadows the real `next_undo`; a failed *undo* marks its target reversed, so the next `u` undoes an older operation

`crates/graft-ops/src/journal.rs:153-159` (`took_effect`) and `:163-174`
(`in_force_flags`).

`took_effect(Aborted) = worktree_before.is_some()`. Under ADR-0009 (and the
design note's choice 3) every operation that touches index or worktree takes
the full before-snapshot, so `worktree_before` is `Some` for *every* aborted
stage, unstage, discard, commit, amend, stash push/apply/pop — and for every
aborted `Undo`/`Redo`, which the executor snapshots too. Two consequences:

1. A commit whose hook rejected it, a `stash push --staged` with nothing
   staged (git exits 1 — *verified*), a `stash push` on an unborn head (exits
   1 — *verified*): each becomes an in-force entry that changed nothing.
   `next_undo` returns it; `u` restores a state equal to the current one and
   the user has to press `u` twice to undo the previous operation. The journal
   browser marks the no-op as `← u`.
2. Worse: `[Commit C, Undo(C) aborted]` — the CAS refused because the branch
   moved, or `RestoreIndex` hit an I/O error before writing anything. The
   aborted undo is in force, `in_force_flags` inserts `C` into `reversed`,
   `C` leaves the undo cursor although it is still fully in effect. The next
   `u` undoes whatever preceded `C`. That is an unexpected write to refs,
   index or working tree, with the user believing they are retrying the undo
   of `C`. Backed up, so not lost — but "u undid the wrong thing" is exactly
   the outcome ADR-0009 exists to prevent, and nothing warns.

The design note's Proposed change 2 wants the *half-applied* abort undoable
(a `stash pop` that conflicted). That is satisfiable without this: the group
already requires the executor to record after-state on abort (deviation 3).

**Fix (this group, ~10 lines + 3 tests):** `took_effect(Aborted)` = backup
present **and** something recorded differs from before — any
`refs_after[k] != refs_before[k]`, `head_after != Some(head_before)`,
`index_after != index_before`, `worktree_after != worktree_before`; an
after-field that is `None` while the op touched it counts as changed
(unknown is not "unchanged"). Tests: `[Stage, Commit aborted, after == before]`
→ `next_undo` = Stage; `[Commit, Undo(Commit) aborted, nothing moved]` →
`next_undo` = Commit and `next_redo` = None; `[StashPop aborted,
worktree_after != before]` → in force (the existing test keeps passing once
its `aborted()` builder sets a differing after-tree).

### C2. Undo of a stash push that created no stash drops the user's previous `stash@{0}`

`crates/graft-ops/src/undo.rs:220-228` (`stash_step`, `pushed()`).

`inverse(StashPush)` always ends in `StashDrop { commit: refs_after["refs/stash"] }`.
It never checks that `refs_after` differs from `refs_before`. `git stash push`
on a tree that has nothing to save prints `No local changes to save` and
**exits 0** → the entry is `Committed` with `refs_after["refs/stash"] ==
refs_before["refs/stash"]` = the stash the user already had. `u` then runs
`StashDrop` on that commit and the executor's "drop by commit" removes the
user's own earlier stash. Not pinned by anything (see M2), so gone after
`gc.pruneExpire`.

Reachable: `RepoFacts` are read once (the struct's own doc) and `check` passes
on them; the stash prompt then waits for a message while the editor
autosaves a revert — `status` was true when read, false when git ran. Also
reachable on any status/git disagreement (a change git considers
not-stashable, e.g. a submodule pointer in some configurations).

**Fix (this group, ~5 lines + 1 test):** in `stash_step`, for `StashPush` in
either direction compare `refs_after[STASH_REF]` with `refs_before[STASH_REF]`
(missing key → `Incomplete`, as today); equal → `None` (nothing was pushed;
the generic restores are still correct and are no-ops). Test: a push entry
with equal stash refs before/after produces no `StashDrop`.

## Major

### M1. `PathScope::Everything` plus "an unrecorded field cannot drift" restores the whole working tree with no warning

`crates/graft-ops/src/undo.rs:211-216` (`scope`) and `:353-371` (`drift`,
deviation 5).

An entry whose `worktree_after` is `None` (the executor could not read the
after-tree — an I/O failure hashing the tree after the op) gets
`RestoreWorktree { paths: Everything }`. `drift` then reports the working
tree as clean because there is nothing to compare. The user edits for an
hour, presses `u`: the §6.7 rail says nothing changed, the restore overwrites
every file with the before-snapshot. Recoverable through the undo's own
pre-undo snapshot (ADR-0009), but the rail — "stops, explains exactly which
of the three changed, and asks" — is silent in the one case where it should
be loudest. The conductor's question "are the rules reachable in a case
where they'd restore too much" — yes, this one.

**Fix:** `Drift` gets `unknown: Vec<&'static str>` (`"the working tree"`,
`"the index"`) filled when the entry's after-field is `None` and
`touches` says the inverse restores it; `is_clean` is false when non-empty;
`explain` says `the working tree's state after the operation was never
recorded; undo restores all of it`. Group 05's confirm box then fires. Two
lines in `drift`, one test.

### M2. `InversePlan` carries no `pins`; the redo of an undone stash push depends on gc timing

`crates/graft-ops/src/undo.rs:93-106` (`InversePlan`), `:220-247`.

Undo of a push emits `StashDrop(S)`; after it, `S` is unreachable from any
ref. Redo emits `StashStore(S)`. The push entry's backup ref does not pin `S`
(it did not exist before the push). The Undo entry could, but `InversePlan`
never says "pin this" — the executor would have to infer it from the steps,
and the "For the execution group" contract in the ticket does not mention it.
`git gc --auto` runs inside ordinary git commands and prunes unreachable
objects older than `gc.pruneExpire` (2 weeks default); a redo after that
fails, or worse, `stash store` of a missing object.

Same for undo of a pop/drop (`StashStore(pins[0])`): the original entry pins
it, fine — but the *redo* (`StashDrop(pins[0])`) then leaves the commit
reachable only through the original entry's backup ref, which §6.8's
retention can clear. Pins should travel with every plan that drops.

**Fix:** `InversePlan.pins: Vec<Oid>` = every `StashDrop { commit }` in
`steps`; the executor writes them on the new entry's pending line (deviation
2 already says pins are known before the op). Add to the contract paragraph.

### M3. No seam guard test; the seam rule is prose only

Design note "Seam rule": *"Guard test: `GitInvocation` appears under
`crates/graft/src/` only inside `jobs/` (reads)."* ARCHITECTURE.md:38-42 got
the sentence; no test exists (`grep -rn GitInvocation crates/graft/src` is
empty today, so the test is trivially green and costs nothing). The spec's
piece 5 dropped it. Once groups 03–05 wire keys, a builder writing a
`GitInvocation` in `update/` gets no gate.

**Fix:** a test beside `crates/graft/tests/it/dependency_edges.rs` (or in the
`update/` purity guard, `crates/graft/src/update/tests.rs`) that walks
`crates/graft/src`, allows `jobs/`, and fails on `GitInvocation`, `graft_ops::journal::render_`, and `Step::`. Ten lines; do it before group 03.

### M4. `Discard(Target::Patch)` cannot express a staged hunk, and `check` lets one through when the file also has an unstaged change

`crates/graft-ops/src/operation/stage.rs:25-33` (`State::admits`), `:156-164`
(`plan_discard`, patch arm).

The design note's Changes pane: the body shows *the staged diff for a STAGED
file*, and `d` discards the unit under the cursor. So group 04/05 will send
`Discard(Patch)` built from the index-vs-HEAD diff. `check`'s `Discardable`
for a patch is `entry.is_unstaged()`: refused when the file is staged-only
(fine — a refusal, and the test covers it), **admitted** when the file has
both a staged and an unstaged change. `plan` is then `apply -R -` against the
working tree only. Result: the hunk leaves the working tree, stays in the
index; the file now shows an *unstaged reversal of a staged change*. Not data
loss (undo works), but the operation did not do what its name says, and the
shape gives the UI no way to say "this patch came from the index".

**Fix (structural, so now):** `Target::Patch { path, patch, side: PatchSide
{ Index, Worktree } }` — or a separate `Discard` rule: a patch from the index
plans `apply --cached -R -` then `apply -R -` and `check` requires
`is_staged()`; from the worktree, today's rule. Whichever, the doc on
`Target::Patch` should say the patch is built with `PatchDirection::Reverse`
*and* applied with `-R` (today's "reverse for an unstage or discard" reads as
a double reversal until you open `plan`).

### M5. The stash-undo contract rests on untracked scripts, and one of its claims is only half true

Ticket `## NOTES`, deviation 2; `target/verify-ops-model-2.sh` (untracked).

"Subject byte-identical to the reflog message", "a failed pop keeps its
entry", "dropping the last stash deletes `refs/stash`", "`stash store -m
<subject>` re-lists identically" — all real, all verified by hand, none in
the repository. Nothing in the fast or heavy tier will notice when git
changes one. The conductor asked whether the tests prove the claim: they
assert the *plan* (`StashStore { commit }` is emitted; `refs/stash` is never
an `UpdateRef`); the git facts under it are unproven in-tree.

And "store = no-op if the reflog has it" is **not** git's behaviour, it is a
promise the executor must keep: *verified* — `git stash store -m x <tip>`
adds nothing when `<tip>` is already `stash@{0}`, but the same command for
the commit at `stash@{1}` adds a **duplicate** at the top. The executor has
to scan the reflog for the commit before storing; deviation 2's wording
suggests git gives this for free.

**Fix:** group 03's spec names one fixture test per claim (the group already
has real-git fixtures), and KNOWN_GAPS carries an *unverified* row until they
land. Reword deviation 2 in the ticket: "idempotent by commit — the executor
checks the reflog; git does not".

### M6. Three `check` gaps let git fail where a `Refusal` was due (each feeds C1)

`crates/graft-ops/src/operation/stash.rs:29-43` (`check_push`).

- `staged_only` with nothing staged: passes `check` (it accepts
  staged-or-unstaged), git says `No staged changes`, exit 1 — *verified*.
- `staged_only && include_untracked`: representable in `StashParams`, git
  refuses `Can't use --staged and --include-untracked` — *verified*. The
  design note's prompt toggles one *or* the other, so the shape allows a
  state the UI never sends.
- `StashPush` on an unborn head: `You do not have the initial commit yet`,
  exit 1 — *verified*; `Amend` refuses `UnbornHead`, push does not.

**Fix:** `staged_only` → `NothingToStash` unless `status.staged()` is
non-empty; `StashParams { scope: StashScope::All { include_untracked } |
StagedOnly }` (or refuse the pair); `UnbornHead` for push.

### M7. Pathspec magic in every `-- <path>`

`crates/graft-ops/src/operation/stage.rs:265-273` (`with_paths`).

Paths go to `add`, `restore`, `rm`, `clean` as pathspecs. `*`, `?`, `[` and
a leading `:` are magic even after `--`: a file literally named `a*b` (or a
status entry under a directory with a `[` in it) matches more than itself,
and `clean -f -- <glob>` removes every untracked file that matches. The
snapshot makes it recoverable; the plan is still wrong. `GitArg::Path` is
documented as raw bytes, which is what the reader expects here.

**Fix:** the runner passes `--literal-pathspecs` (global flag, git ≥ 1.8) for
every write `Invocation`, or `with_paths` prefixes `:(literal)`. One test
with a `*` in the name.

## Minor

1. `undo.rs:254-269` `touches_of` dedups with `Vec::dedup`, which only removes
   *adjacent* duplicates; it works because `UpdateRef` names come sorted from
   a `BTreeMap` and `HEAD`/`refs/stash` are appended after. A `BTreeSet`
   says what is meant.
2. `journal.rs:153-159` — after C1, an aborted `Undo` that got half way is in
   force and `next_redo` returns it; its plan is described `redo: <root>`
   although it puts back the *failed undo's* before-state. Say
   `redo: <root> (after a failed undo)` or keep the root text and document
   the case on `next_redo`.
3. `stash.rs:91-93` — `message: Some("")` plans `stash push -m ''`, which git
   treats as no message (*verified*: the stash is named `WIP on main: …`),
   while `describe` says `stash ""`. Normalise empty to `None` in `plan_push`
   and `describe_push`.
4. `journal.rs:271-290` `state_at` returns `None` for both "the ref was
   deleted" and "no entry ever recorded it" (`.flatten()`); M5's time machine
   will need to tell them apart. `BTreeMap<String, Option<Oid>>` could hold
   only refs some entry *recorded* at that moment — or document that a name
   is present only when known.
5. `journal.rs:74` — `parse` decodes with `from_utf8_lossy`; a `description`
   holding a non-UTF-8 path was already lossy on write (it is a `String`), so
   this is consistent, but worth one sentence in the doc so a reader does not
   look for the byte-exact path there.
6. `stage.rs:156-208` `plan_discard` is 70 lines with an early-return unborn
   branch inside; `plan_discard_unborn(classes)` would leave each path under
   the 40-line arm rule the spec set for helpers.
7. `operation.rs:70-83` `Target::Patch` doc (see M4's last paragraph) and
   `undo.rs:58-68` `StashStore` doc ("a no-op when the reflog already has it")
   state executor promises as if they were git's; both docs should say who
   guarantees what.
8. `lines.rs:82-90` `Line::from_json` parses every line twice (a `Head` for
   the status, then the record). Fine at journal sizes; a `#[serde(tag)]`
   would not work because both records share the `status` field's type, so
   this is a note, not a request.

## The eight deviations, on merit

| # | Deviation | Verdict |
|---|---|---|
| 1 | `Parsed { entries, torn_tail }`; `render_*` return `Result` | **Better.** A torn tail is reported, never swallowed; `StillPending` catches a misuse at the type boundary. Keep. |
| 2 | `refs/stash` never an `UpdateRef`; `StashStore`/`StashDrop` by commit, "idempotent" | **Better in principle** — moving `refs/stash` with `update-ref` would corrupt the list. Two holes: idempotence is the executor's to build, not git's (M5); a push that pushed nothing must emit no drop (C2). Keep with both fixes. |
| 3 | Missing `refs_after` key → `Incomplete`; missing after-tree → `Everything` | **Half right.** `Incomplete` is the honest answer and the test proves it. `Everything` is the right *scope* but arrives silently (M1); keep the variant, add the drift signal. |
| 4 | `inverse(entry, root)`, `reverses` names the direct target | **Better.** The spec's "original" was ambiguous; root-by-chain with a cycle guard is the only reading under which undo-of-redo equals undo-of-original, and the test `undoing_a_redo_is_undoing_the_original` shows it. Keep. |
| 5 | An unrecorded field cannot drift | **Regression** in combination with 3 (M1). "Unknown" must not read as "clean". |
| 6 | `clean -f`, never `-fd` | **Better.** Simpler, and the builder's script shows a named directory goes without `-d`; `--untracked-files=all` means `check` never sees a directory. Keep (2.54-only caveat already filed). |
| 7 | Unborn discard: `rm --cached --quiet` then `clean -f` | **Right.** `restore --source=HEAD` cannot exist without `HEAD`; the builder's script 2 shows the failure and the replacement. Keep. |
| 8 | `run::command_line(&[GitArg])` as a free fn | **Better.** One quoting path for the runner and the Command Log. Keep. |

## Spec compliance and Definition of Done

Every acceptance box is checked and has a test behind it: plan table
(`plan_*`), check table (every `Refusal` one fact away, mid-rebase stage
allowed / commit not, stash apply dirty-there vs dirty-elsewhere), round trip
incl. torn tail and orphan outcome, the four cursor scenarios, per-kind step
vectors incl. redo-of-undone-pop, `state_at` at 25/5/30, drift per field,
`explain` sentences, MAP.md for core and ops, edge test green. Nothing is
stubbed. Purity holds: no spawn, no clock, no file I/O in the group.

Architecture: `ops → git, core` only; `serde` external; ARCHITECTURE.md's
"Where to start reading" matches the code. Readability is good — module docs
say why, names are plain, one `match` per property. The 40-line arm rule is
kept; `plan_discard` is the one long function (Minor 6).

## Does the promise hold?

Not yet on the error path. On the happy path the shape delivers "byte-for-byte
back on `u`" for every M3 kind, and `NotInvertible` is honest wherever a
field is missing. After a failure the model is wrong twice (C1, C2) and
silent once (M1) — all three in the code that decides, none needing a git
spawn to fix or test. Fix C1 and C2 before group 03 builds on `in_force` and
`stash_step`; M1–M4 can go to the same pass or to 03's spec, but not later.

## For KNOWN_GAPS

- **Broken** (until C1/C2 land): "a failed undo marks its target reversed;
  the next `u` undoes an older operation"; "undo of a stash push that pushed
  nothing drops the previous stash".
- **Unverified**: the stash reflog facts deviation 2 rests on (subject ==
  reflog message; failed pop keeps its entry; store of a non-tip commit
  duplicates, so the executor must check the reflog) — proven only by
  untracked scripts on git 2.54; expiry group 03.
- **Unverified**: `PathScope::Everything` has no fixture that reaches it; the
  executor path that produces a `None` after-tree is group 03's to name.
