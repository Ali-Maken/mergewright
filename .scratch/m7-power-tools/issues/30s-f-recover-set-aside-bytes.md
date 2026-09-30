# f-recover — the set-aside put-back gives back his bytes, so `J ⏎ y` then `u` holds an LF file under autocrlf

Status: blocked (verify done, move committed, tests red; the fix needs files outside the list — NOTES `### Blocked`)
Build with: **Opus, `builder-opus-xhigh`.** Undo territory and the M3 promise ("you cannot lose work"); the cause is
not confirmed, so the builder verifies before it builds, and real-git byte tests judge it. Not Fable: the fix and its
signatures are fixed below; no structural choice is left.
Needs: 30q-a/30q-b as merged (`Snapshot::worktree_raw`, `JournalEntry::worktree_raw_before`, `restore/steps.rs`
`write_raw`). Nothing from the other fix specs.
Base: `main`. 26's branch (`worktree-agent-aeb403f0c2282df64`) is **not merged**; its promise test is read with
`git show worktree-agent-aeb403f0c2282df64:crates/graft/tests/it/m7_walk/promise/recover.rs`, never run here.
Runs concurrently with: `f-guard-refusal-at-80` (graft-ops too; disjoint files), `captures-reshoot-named`,
`captures-verify-questions`.

## Why
26 NOTES, F-recover: `promise_recover_without_leaving` fails at `promise/recover.rs:103` (`assert_bytes`, :145-158).
`src/a.rs` is LF before `J ⏎ y`, CRLF after it, still CRLF after `u`. The scout's reading (**unverified**, rule 7):
- the set-aside put-back's `git stash apply --index` re-smudges his edit (`graft-ops/src/apply/set_aside/put_back.rs:66,99-109`);
- `u`'s `RestoreWorktree` is scoped `ChangedBetween` on **cleaned** trees (`undo.rs:339-347`); the file is equal there,
  so `scoped_paths` is empty (`restore/steps.rs:604-606`) and `write_raw` never runs (`steps.rs:280-283`);
- the raw bytes were kept (`snapshot/raw.rs:80-107`, `apply/record.rs:126`).

## Ruling (planner): fix the write that changes his bytes, not `u`'s scope
- The operation itself changed his file: the set-aside is meant to put his edits back **as they were** (ADR-0045 §4),
  and git's stash hands them back in git's encoding. That is the cause; `u` missing it follows from it.
- Fixing the put-back makes the forward result right too (closes KNOWN_GAPS `M7/30q-setaside`, today **unverified**).
  After it, `u` needs nothing new: the operation no longer changes the file, so `ChangedBetween` is right to skip it.
- **Rejected: widen `u`'s scope to every raw path whose bytes differ.** The after-snapshot keeps no raw bytes, so `u`
  cannot tell an ending the operation changed from one he changed after it. It would widen KNOWN_GAPS
  `M7/30q-drift-eol` (endings-only edits overwritten unasked) to paths the operation never touched.
- The put-back is the only write that changes endings of his content inside one operation: every other write either
  leaves a path or changes its content (then it is in `ChangedBetween`, and 30q-b's `write_raw` restores it).
  **Unverified**; step V4 checks it by grep.

## Files (exact; outside this list → stop, `blocked: needs <file>`)
- graft-ops:
  - `src/apply/set_aside/put_back.rs` (the call after a clean apply);
  - `src/restore/steps.rs` (`write_raw` and `batch_blobs` moved **out**, nothing else);
  - new `src/restore/raw.rs`; `src/restore.rs` (one `mod raw;` line, and a `pub(crate) use` if the move needs it);
  - the caller of `put_back` only if the entry's `worktree_before`/`worktree_raw_before` must be passed in (one
    argument; name the file in NOTES);
  - `tests/it/undo_bytes.rs` (new tests); `MAP.md` (`restore::raw`, the put-back line).
- Not: `src/undo*.rs`, `src/snapshot*`, the journal types, anything in graft.

## Verify first (before any code; NOTES `### Verify`, each fact with the command and what it printed)
- **V1, red.** `undo_bytes::u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf` (below) on today's code.
  - Green → **stop**: `blocked: graft-ops holds LF through a set-aside put-back and u; the cause is in graft's route`.
    Test body in NOTES.
- **V2, the before-snapshot kept it.** In V1's repo, after the operation: `git ls-tree -r <backup commit>:raw`
  lists `src/a.rs`, and `git cat-file blob <backup>:raw/src/a.rs` is `W` (LF).
  - Not listed → **stop**: `blocked: raw/ lacks src/a.rs — R2's bound (ADR-0052) or raw_record, not the put-back`.
- **V3, the put-back is the write.** The bytes of `src/a.rs` just before `stash apply --index` and just after it (a
  debug print in a scratch copy, reverted; or `GIT_TRACE` of the op): LF → CRLF across that call, and no other step of
  the operation writes `src/a.rs` after it.
  - Another step writes it → **stop**: `blocked: cause elsewhere: <step, file:line>`.
- **V4, no other endings-only writer.** Grep graft-ops `src` for worktree writers inside one operation that re-apply his
  own content (`stash apply`, `stash pop`, `checkout-index`, `restore --source`, `read-tree -u`). List each with
  file:line and whether it can change only endings of a path whose content it leaves. Any other one found → name it in
  NOTES as a question; do not fix it here.
- **V5, observe J's `Files` put-back forward (no fix).** Through the path `recover::u_after_a_file_put_back_brings_back_what_it_replaced`
  uses: an LF lost edit under `core.autocrlf=true`, put back. Record the bytes after the forward step (command and
  bytes) in NOTES. That is KNOWN_GAPS `M7/30q-putback-fwd` (**unverified** today); the conductor reclassifies it from
  this line. No committed test for it.

## Signatures
```rust
// crates/graft-ops/src/restore/raw.rs (new). First commit: `write_raw` and `batch_blobs` moved here from
// restore/steps.rs unchanged (a pure move; every test green; steps.rs shrinks). Visibility pub(crate).
pub(crate) fn write_raw(/* today's parameters, unchanged */) -> Result<(), RestoreError>;

/// The paths of `raw` whose bytes on disk, cleaned, are `tree`'s blob at that path: his content, in git's encoding.
/// One `git hash-object --stdin-paths` (filters on: the path decides them). A path `tree` lacks, or missing on disk,
/// is left out. Leading parameters: `write_raw`'s repo/git/stop ones, copied.
pub(crate) fn same_content_paths(/* write_raw's leading params */, raw: Oid, tree: Oid)
    -> Result<BTreeSet<PathBuf>, RestoreError>;
```
```rust
// crates/graft-ops/src/apply/set_aside/put_back.rs — after `stash apply --index` exits 0 (never on a clash):
// if let Some(raw) = entry.worktree_raw_before {
//     let his = same_content_paths(.., raw, entry.worktree_before)?;
//     write_raw(.., raw, &his)?;   // the same temp-and-rename write `u` uses
// }
```
- `worktree_before`/`worktree_raw_before` are the running operation's own entry (the pending line `begin` wrote).
  Reach them through the put-back's existing context; if that needs a journal or shape change → **stop**,
  `blocked: the put-back cannot see the entry's raw tree`.
- A failed raw write fails the put-back through its existing failure path (the one a clash takes; name it in NOTES),
  worded `could not write <path> back as it was: <io> · u restores the backup`. Never logged and carried on (rule 3).
- A relaunch that resumes a put-back (30d) goes through the same function, reading the entry from the journal. Check it
  (grep the callers of the put-back); if it does not, say so in NOTES.

## Acceptance (name the verifying test at tick time; **(red)** = seen failing on today's code first, line in NOTES)
Each test: `core.autocrlf=true`; committed `src/a.rs`; `W = b"pub fn a() -> u32 {\n    4\n}\n"` written LF, uncommitted
(30q-a's `lf_edit_under_autocrlf`). Assert with `std::fs::read`, byte for byte.
- [x] V1–V5 in NOTES, each with its command and output. (NOTES `### Verify`; V1 is
      `undo_bytes::u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf`, red; V2/V3/V5 scratch probes
      printed and removed; V4 the grep named there.)
- [ ] (red) `undo_bytes::u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf`: Recover `MoveRef` of the
      checked-out branch over the dirty tree (the scenario's `J ⏎ y`; copy its shape from 26's `promise/recover.rs`), then
      `u`. After `u`: `W`.
- [ ] (red) `undo_bytes::a_set_aside_put_back_keeps_an_lf_file_under_autocrlf`: the same operation. After it: `W`.
- [ ] (red) `undo_bytes::a_switch_with_edits_keeps_an_lf_file_under_autocrlf`: a `Checkout` of another branch that
      does not touch `src/a.rs`, over the dirty tree. After it: `W`; after `u`: `W`. (A second set-aside kind.)
- [ ] `undo_bytes::a_put_back_that_merges_into_a_changed_file_writes_no_raw`: the target branch changes `src/a.rs`
      elsewhere in the file, so the put-back merges his edit into new content. His raw bytes are **not** written: the
      file holds the target's change and his (content asserted, not bytes); `u` gives `W`. Guard; red not expected.
- [ ] `undo_bytes::a_clashed_put_back_writes_no_raw`: the put-back clashes (S kept). No raw write; the conflict is as
      today (compare with the existing clash test's asserts). Guard.
- [ ] `undo_bytes::a_set_aside_under_a_one_way_clean_filter_keeps_the_stripped_lines`: 30q-a's `strip` filter,
      `nb.txt` with `#out 42`, set aside and put back. After the operation, `nb.txt` holds `#out 42`. (red)
- [ ] Mutation: `write_raw` skipped in the put-back → the three red tests red again. Restored; said in NOTES.
- [ ] Unchanged and green in the full run (name them): every `undo_bytes::` and `snapshot_raw::` test; every
      `set_aside` test; `recover::` 30p-a's seven and test 6; `undo_cancel::`; graft `m3_walk::every_scenario::`,
      `m7_walk::journal::the_three_lost_work_journeys`.
- [ ] MAP.md: `restore::raw` (`write_raw`, `same_content_paths`), the put-back's raw step. Gates workspace-wide.

## Touches (each on the report's **Touches:** line, with how it was checked)
- Every kind that sets aside (grep the put-back's callers: checkout, branch create+checkout, merge, pull (03b), rebase,
  rewrite, Recover `MoveRef`): one put-back, one fix. Name each caller.
- The set-aside's clash and relaunch paths (30d): no raw write on a clash; relaunch as above.
- `u` and redo: unchanged code; the operation's after-snapshot now sees his bytes. `recover.rs` and `undo_bytes.rs` green.
- Drift and `replaced`: unchanged (cleaned trees).
- The journal: no new field, no new kind. Golden lines byte-identical (`journal::tests::a_line_without_raw_parses_as_none_and_renders_unchanged`).
- Perf: +0 when `worktree_raw_before` is `None`; else +1 `hash-object` +2 (`write_raw`) per put-back. No time claimed.
- UI, words on screen, keymap, captures: none, except the new failure words above (grep `docs/shots` for
  `back as it was`: none expected).
- KNOWN_GAPS (conductor's): `M7/30q-setaside` closes on green; `M7/30q-putback-fwd` from V5; `M7/30q-drift-eol`
  unchanged.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~60k. If it compacts after the verify, say so; the conductor splits the build.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (rows restated above).

## NOTES

### Verify (on `main` at 68810e43, before any source change; git 2.54.0)
Repo for V1–V3 (`undo_bytes::reset_off_second_with_an_lf_edit`): `core.autocrlf=true`; `first` = `src/a.rs`
(`COMMITTED`), `second` = `b.txt`; `reset -q --hard HEAD~1`; `W` written LF; op = `Recover MoveRef main → second`.

- **V1, red — held.** `cargo nextest run -p graft-ops --test it -E 'test(u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf)'`
  → `FAIL` at `undo_bytes.rs:453`: `W is back, byte for byte, not "pub fn a() -> u32 {\r\n    4\r\n}\r\n"`.
  Not green, so no stop.
- **V2, the before-snapshot kept it — held.** A scratch test (removed) printed, after the op: the entry's
  `backup_ref` (`refs/graft/backup/main/01HF7YAT…`) is the **after** commit (`graft … recover after: …`; tree
  `index`, `worktree`, no `raw`); its first parent is the before commit (`graft … recover: …`), whose tree is
  `index`, `raw 2997161f`, `worktree 2997161f`. `git ls-tree -r <backup_ref>^1:raw` →
  `100644 blob f8304d84…\tsrc/a.rs`; `git cat-file blob <backup_ref>^1:raw/src/a.rs` →
  `"pub fn a() -> u32 {\n    4\n}\n"` (`W`, LF). `entry.worktree_raw_before` = `Some(2997161f…)`, the same tree.
  (The ticket's `<backup commit>:raw` is `<backup_ref>^1:raw`: the ref names the after commit.)
- **V3, the put-back is the write — held, with one correction to the ticket's wording.** A debug `eprintln!` in
  `put_back.rs` `run_put_back` around the first `apply` (scratch, reverted; `git diff --stat -- crates/graft-ops/src`
  empty after): just before `stash apply --index`: `"pub fn a() -> u32 {\r\n    1\r\n}\r\n"`; just after:
  `"pub fn a() -> u32 {\r\n    4\r\n}\r\n"`; after the op returned: the same. So before the call the file is not
  `W` LF but step 4's `reset --hard` checkout of `HEAD` (CRLF, `1`); the put-back is the write that brings **his
  content** back, and it writes it CRLF. The entry's commands after it: only `git stash drop 'stash@{0}'` (full list:
  `stash create`, `stash store`, `reset --hard`, `reset --hard <second>`, `stash apply --index <S>`, `stash drop`).
  No step writes `src/a.rs` after the put-back (bytes after the op = bytes just after the apply). No stop.
- **V4, no other endings-only writer inside one operation — held (grep:
  `grep -rn -e 'Flag("apply")' -e 'Flag("pop")' -e '"checkout-index"' -e 'Flag("restore")' -e '"read-tree"' -e 'Flag("-u")' -e StashApply -e StashPop crates/graft-ops/src`).**
  - `apply/set_aside/put_back.rs:99` `stash apply [--index] S` — **this ticket's**: brings back his content
    unchanged, so for such a path only the endings change within the operation.
  - `operation/stash.rs:144-151` (`plan_at`, used `operation.rs:1612-1613`), his own `StashApply`/`StashPop`:
    within one op the path's content changes (from the tree's to the stash's), so it is in `ChangedBetween` and
    `u`'s `write_raw` restores it. Not endings-only. **Question (unverified):** push then pop, two operations,
    gives his LF content back CRLF — the same family as `M7/30q-putback-fwd`, across operations.
  - `operation/recover.rs:238-245` Recover `Files`, `restore --source=<backup_tree> --worktree`: content changes
    (`lost_edits` only lists a path whose disk content differs from the backup). V5's path.
  - `operation/stage.rs:188-196` discard hunk, `apply -R` on the working tree: a content change. **Question
    (unverified):** `git apply` rewrites the whole file through the working-tree conversion, so the lines he kept
    may change endings along with the hunk.
  - `operation/stage.rs:222-236` `restore [--staged] --worktree [--source=HEAD]` (discard): writes `HEAD`'s or the
    index's content, not his. `stage.rs:132,151,160` `apply --cached`/`restore --staged`: index only.
  - `absorb/build.rs:77,92`, `absorb/judge.rs:61,66` `read-tree` + `apply --cached`: a `TempIndex`, no working tree.
    `preview/stash.rs:244` `apply --cached --check` on an index copy: no write. `snapshot/raw.rs:254`
    `checkout-index -a -f --prefix=<scratch>/` into a scratch dir, not the working tree.
  - `restore/steps.rs:305,455,463` `u`'s own `RestoreWorktree`, followed by `write_raw` (30q-b).
  - `operation/stash.rs:129`, `operation/remote.rs:317` `-u`: `stash push -u`, `push -u` — no re-apply.
  - `apply/set_aside/rebuild.rs:62` `commit-tree` only (objects). `set_aside.rs:128` `reset --hard` clears his
    edits into `S` (expected).
- **V5, J's `Files` put-back forward (no fix) — observed.** A scratch test (removed) through the path of
  `recover::u_after_a_file_put_back_brings_back_what_it_replaced` (`LostWork::build`, then
  `git config core.autocrlf true`, then `apply` of `RecoverTarget::Files` from `lost_edits`): his lost edit
  (`LOST_EDIT`) is `"pub fn a() -> u32 {\n    2\n}\n"` (LF); after the put-back `src/a.rs` is
  `"pub fn a() -> u32 {\r\n    2\r\n}\r\n"` (bytes `…123, 13, 10, 32, 32, 32, 32, 50, 13, 10, 125, 13, 10`).
  Command: `git --literal-pathspecs restore --source=9955b078… --worktree -- src/a.rs`. The entry holding the lost
  edit has `worktree_raw_before: None` (autocrlf was set after the fixture, as 26's promise test does), so on this
  path no raw bytes exist to give back. `M7/30q-putback-fwd` is **broken** as observed here (forward write is
  git's encoding), for the conductor to reclassify.

### Done (committed on this branch)
- `4647a00b` the pure move: `write_raw`, `batch_blobs` and their two private helpers (`write_his_bytes`,
  `unparsable` — used only by them), with their two unit tests, from `restore/steps.rs` to the new `restore/raw.rs`,
  unchanged; `restore.rs` gets `mod raw;`. `steps.rs`'s `tree_entries` and `run` become `pub(super)` (raw.rs calls
  them); nothing else in steps.rs changes. graft-ops `cargo nextest run -p graft-ops --no-fail-fast`: 950/951, the one
  red the V1 test (then uncommitted). Clippy/fmt workspace-wide clean.
- The six acceptance tests in `tests/it/undo_bytes.rs` (next commit, **red by design** until the fix lands), run on
  the moved code: `nextest -E 'test(/undo_bytes::(u_after_a_set_aside|a_set_aside_put_back|a_switch_with_edits|a_put_back_that_merges|a_clashed_put_back|a_set_aside_under_a_one_way)/)'`
  → 2 passed, 4 failed. Red lines (the panic is in `assert_bytes`, `undo_bytes.rs:428`; the calling line named):
  - `u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf` :478 `after u: src/a.rs is "…{\r\n    4\r\n}\r\n"`;
  - `a_set_aside_put_back_keeps_an_lf_file_under_autocrlf` :488 `after the put-back: src/a.rs is "…\r\n…"`;
  - `a_switch_with_edits_keeps_an_lf_file_under_autocrlf` :519 `after the checkout: src/a.rs is "…\r\n…"`;
  - `a_set_aside_under_a_one_way_clean_filter_keeps_the_stripped_lines` :630 `after the put-back: nb.txt is "cell 2\n"`;
  - guards green today, as expected: `a_put_back_that_merges_into_a_changed_file_writes_no_raw`,
    `a_clashed_put_back_writes_no_raw` (its asserts copied from `set_aside::a_clashing_put_back_is_clashed_and_keeps_the_stash`).

### Blocked — `blocked: needs apply/set_aside/returned.rs, apply/set_aside.rs, and a ruling on the failure path (answer.rs, apply/error.rs)`
Three facts from the code make the fix leave the file list:
1. **Two hops, not one, to reach the entry.** `put_back.rs` `run_put_back` is called by `returned.rs:149`
   (`returned::put_back`), which is called by `set_aside.rs:255` (`close`), the only place holding `pending.entry`.
   Passing the entry's raw/before in is one argument through **two** files (`returned.rs` signature + `set_aside.rs`
   call). The ticket allows one caller file.
2. **`&mut Context`, and the put-back never stops.** `write_raw` and the `tree_entries`/`run` it calls take
   `&mut Context` and run with `ctx.stop`; `returned::put_back`/`run_put_back` take `&Context`. The put-back runs every
   git with `&()` ("the put-back never stops", `set_aside.rs:245-247`), so the raw step must too: swap `ctx.stop` to
   `&()` around it, as `restore/rollback.rs:170` `uncancellable` does. That needs `&mut Context` through
   `returned::put_back` (`close` already holds `&mut`).
3. **The existing failure path cannot carry the words.** The put-back's failure path is `PutBackRan::Failed { error:
   RunError }` (`put_back.rs:55`) → `Kept::Git { back: false }` (`returned.rs:153-158`) → entry `Aborted`,
   `EditsAt::Stash { n }` "your edits are in stash@{n}" (`apply/error.rs:141,166`, the app's `S p` hint). A raw write
   fails with `RestoreError::WriteRaw { path, io }`; there is no honest `RunError` for it, and after a clean apply
   the edits **are back on disk**, so `EditsAt::Stash` would say they are only in the stash (wrong recovery hint).
   The clash path (`Returned::Clashed`) is for unmerged paths. Every `Kept`/`Outcome::Kept` error is
   `Result<RunError, FactsError>` (`returned.rs:51-56`, `answer.rs:154`).

**Proposed (for the planner/conductor to rule):** the raw failure is the same shape as `AfterState::StagingNotKept`
(`apply/error.rs:269-275`): the write went through, the edits are back, one aspect was not kept, and `u` restores it
exactly. So: `S` dropped as on a clean put-back; a new `AfterState::NotAsTheyWere(RestoreError)` (words
`could not write <path> back as it was: <io> · u restores the backup`) first in `unread`, returned as
`ApplyError::AfterState`; the entry `Committed`. Knock-on to check: graft `jobs/operate.rs:369` special-cases a lone
`[AfterState::StagingNotKept]`; any exhaustive match on `AfterState` in graft.

### Exact next step (once granted)
- `raw.rs`: `pub(crate) fn same_content_paths(ctx: &mut Context<'_>, raw: Oid, tree: Oid, commands) ->
  Result<Vec<Vec<u8>>, RestoreError>` — `tree_entries(raw, [])` → `tree_entries(tree, those)` → keep paths `tree` holds
  and that are regular files on disk (`symlink_metadata().is_file()`; `hash-object` exits 128 on a missing path) →
  one `git hash-object --stdin-paths`, every path C-quoted (`"…"`, `\"`, `\\`, other bytes outside 0x20–0x7e as
  `\ooo`: measured on git 2.54.0, octal, `\n` and `\"` unquote and CRLF cleans to LF) → keep those whose hash is
  `tree`'s blob. **Proposed deviation:** returns `write_raw`'s path type (`Vec<Vec<u8>>`), not `BTreeSet<PathBuf>`,
  to feed `write_raw` without two conversions. `restore.rs`: `pub(crate) use raw::{same_content_paths, write_raw};`.
- `put_back.rs`: `run_put_back(ctx: &mut Context<'_>, stash, his: Option<HisBytes>, commands)`; after **either**
  clean apply (`Whole`, and the fallback's `Unstaged` — the plain apply re-smudges too), with `ctx.stop` swapped to
  `&()`: `same_content_paths` then `write_raw`. Never on `Clashed`/`Failed`. `HisBytes { raw, tree }` from the
  entry's `worktree_raw_before`/`worktree_before`, `None` when either is (`+0` spawns).
- Then the failure path as ruled; mutation check (skip `write_raw` → the four red tests red); MAP.md; full gates.
- Relaunch (30d): `apply/recover.rs` `recover_interrupted` does not call `run_put_back`/`close` (grep); a resumed
  rebase goes through `resume.rs:443` → `close`, the same function — covered.
- Set-aside kinds, all through `apply.rs:383`/`resume.rs:443` → `close` → `run_put_back` (`operation.rs:1323-1340`):
  `Rebase`, `Rewrite`, `CherryPick`, `Pull` (overlapping), `Checkout` (`OnEdits::SetAside`), `BranchCreate`
  (checkout + `SetAside`, not at `HEAD`), `Recover` (`MoveRef` checked out). Not `Merge` (the ticket lists it).
