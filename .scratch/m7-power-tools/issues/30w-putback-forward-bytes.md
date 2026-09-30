# putback-forward-bytes — `J n ⏎ y` puts a lost edit back in his own bytes, where the backup kept them

Status: partial — graft-ops fix, tests 1–7 and `build_with` done; the walk blocked (`blocked: needs
crates/graft/tests/it/m7_walk/journal.rs`, NOTES § Blocked)
Build with: **Opus, `builder-opus-xhigh`.** Undo territory and the M3 promise ("you cannot lose work"), which the M7
acceptance walk asserts. The cause is named by two builders but its preconditions in the walk are not confirmed, so the
builder verifies before it builds; real-git byte tests judge it. Not Fable: the shape is fixed below, no structural
choice is left (no public type changes).
Needs: 30q-a/30q-b and 30s as merged (`JournalEntry::worktree_raw_before`, `restore::write_raw`,
`AfterState::NotAsTheyWere`). Nothing from other open tickets.
Base: **26b's branch (`worktree-agent-a14300ec2a4a22d53`) with `main` merged** — the walk this ticket must turn green
(`crates/graft/tests/it/m7_walk/promise/recover.rs`) exists only there. The conductor prepares the worktree; the builder
does no merge.
Runs: concurrently with 30w-b (same base; disjoint files, no crate code there).

## Why
26b NOTES, F-recover: `promise_recover_without_leaving` red at `promise/recover.rs:123`. After `J n ⏎ y` ("your edit to
src/a.rs") the file is `…{\r\n    2\r\n}\r\n`; his lost edit was `…{\n    2\n}\n`, under `core.autocrlf=true`. The write
is `RecoverTarget::Files`'s plan, `--literal-pathspecs restore --source=<backup_tree> --worktree -- <paths>`
(`graft-ops/src/operation/recover.rs:239-247`, 26b; `:238-245` in 30s V4), which smudges. KNOWN_GAPS
`M7/30q-putback-fwd`, **broken**, observed by 30s V5 (same path: V5 ran `apply` of `RecoverTarget::Files` from
`lost_edits`, command `git --literal-pathspecs restore --source=9955b078… --worktree -- src/a.rs`).
Two facts from 30s V5, **unverified for the walk** (V2/V3 check them):
- the lost edit lives in an undo/redo entry's before-snapshot: `backup_tree` is that entry's `worktree_before`, and its
  `raw/` (ADR-0052) sits beside it in the same backup commit;
- in V5's fixture that entry had `worktree_raw_before: None`, because `core.autocrlf` was set **after** `LostWork::build`
  (at snapshot time git converted nothing, so ADR-0052 §2 kept nothing). 26b says the walk does the same.

## Rulings (planner, 2026-10-01; not open)
- **R1. Fix the forward write, as 30s did for the set-aside.** After the plan's `restore --source` exits 0, write his raw
  bytes over each put-back path the entry's `raw/` holds (`restore::write_raw`, the same temp-and-rename write `u`
  uses). The raw source is **the lost edit's own entry** (`RecoverTarget::Files.entry`), read from the journal entries
  `ctx.journal` already holds (`Journal::entries`, in memory, 0 spawns), and used **only when** that entry's
  `worktree_before == backup_tree` (`raw/` belongs to that tree and no other). No `same_content_paths`: the restore has
  just written `backup_tree`'s blob at exactly these paths, so every path `raw/` holds is his content.
  - Rejected: a `raw` field on `RecoverTarget::Files`/`LostEdit`. 24 source files construct or match them (grep), 10
    in graft; the journal already has the answer.
- **R2. `raw: None` stays git's checkout; the walk's scenario is the one he lives in.** A path the backup did not keep
  raw is one git's checkout gave back byte-exact **under the config in force when the backup was taken** (ADR-0052 §2).
  If he changes `core.autocrlf` between losing an edit and putting it back, git's checkout now converts it: the content
  comes back, the endings follow his new setting, as every other checkout in his repo now does.
  - Rejected: write the `worktree/` blob unfiltered when `raw` is `None`. `None` also means "an entry older than 30q-a"
    (the line renders identically, ADR-0052 §3); there, the blob is git's **cleaned** encoding (a CRLF file he had comes
    back LF). A guess either way; rule 2.
  - Rejected: record in the backup that `raw_record` ran and the config it saw. A journal/snapshot shape change for a
    config flip nobody reported.
  - **So the walk sets `core.autocrlf=true` before the fixture's first commit**, when V3 finds the lost edit's entry
    with `worktree_raw_before: None`. graft-fixtures gains `LostWork::build_with` (Signatures); the walk calls it with
    `[("core.autocrlf", "true")]` and drops its later `git config`. **No assert, count or key changes** in
    `promise/recover.rs`; `git diff` of it shows only the build call and the removed config line. Any other step of the
    walk going red under the earlier config → **stop**, `blocked: walk step <n> red under autocrlf-from-start: <line>`,
    never loosened.
  - Conductor: ADR-0052 amendment line (J's forward put-back now keeps his bytes; a conversion setting changed since the
    backup gives git's checkout of today). No KNOWN_GAPS row for the flip: it is the ADR's bound, pinned by test 7.
- **R3. A failed raw step: 30s's `NotAsTheyWere`, unchanged.** The restore went through; his content is on disk in git's
  encoding; one aspect was not kept. Entry `Committed`; `ApplyError::AfterState { applied, unread }` with
  `AfterState::NotAsTheyWere { error, files }` **first** in `unread`. `files`: `paths.len()` when the error is
  `RestoreError::WriteRaw`, else `0` (the "check failed" words). **Words unchanged** (30s §1; "your edits are back"
  reads true for one edit put back). Never a roll-back, never `Aborted`: the content he asked for is there.
  - What `u` does from there: undoes the put-back (its own before-snapshot, exact since 30q). The exact bytes stay in
    the lost edit's entry's `raw/`; after the put-back the path no longer shows as a lost edit (its cleaned content
    matches), so no in-app route reaches them — the same as 30s. Conductor: widen `M7/30s-raw-fail` to name this path.
- **R4. Never stopped, always logged.** The raw step runs with `ctx.stop` swapped for `&()` (the restore has run; a stop
  here would leave git's encoding and no record), inline as 30s did. Its spawns go on the entry's Command Log, so it
  runs **after** the plan and **before** `finish` writes the outcome line. It runs only when the plan exited 0.

## Verify first (before any code; NOTES `### Verify`, each fact with its command and what it printed)
- **V1, red.** Test 1 below on today's code. Green → **stop**: `blocked: a lost-edit put-back holds LF under autocrlf in
  graft-ops; the walk's red is elsewhere`. Test body in NOTES.
- **V2, the backup kept them, and it is that tree's.** In V1's repo, scratch: the lost edit's entry (`Files.entry`) has
  `worktree_raw_before = Some(r)`; `git ls-tree -r r` lists `src/a.rs`; `git cat-file blob r:src/a.rs` is the LF edit;
  and `Files.backup_tree == entry.worktree_before`.
  - `raw` `None` or lacking the path → **stop**: `blocked: raw/ lacks src/a.rs under autocrlf-from-start — raw_record's
    bound, not the put-back`.
  - `backup_tree` ≠ `worktree_before` → **stop**: `blocked: the put-back restores from <which tree>, not the entry's
    worktree_before`.
- **V3, the walk's case.** In the walk and graft-fixtures `lost_work`: where `core.autocrlf` is set relative to the
  fixture's forced undo (file:line each). Then, scratch, the walk's order: the lost edit's entry's
  `worktree_raw_before`. `None` → R2 applies. `Some` → the walk needs no change; R2's fixture work is **out**, say so.
- **V4, the restore is the last writer.** The entry's commands after the put-back (V1's test, printed): nothing writes
  `src/a.rs` after the `restore`. Another step does → **stop**, `blocked: cause elsewhere: <step, file:line>`.
- **V5, one route.** Grep graft for `RecoverTarget::Files` and `Operation::Recover`: every put-back of a lost edit (J's
  list, and any other view building `Files`) reaches graft-ops through `apply`. List each call site. One that does not
  → **stop**, name it. And: can `Operation::Recover` reach `apply/fold.rs` (a step inside a live operation)? Grep and
  say; if it can, the raw step goes there too (fold.rs joins Files).

## Files (exact; outside this list → stop, `blocked: needs <file>`)
- graft-ops:
  - new `src/apply/lost_edit.rs` (the raw step); `src/apply.rs` (`mod lost_edit;` and the one call after the plan, for
    `Operation::Recover` with `RecoverTarget::Files`);
  - `src/apply/run.rs` **or** `src/apply/record.rs`, only for the call site and pushing `NotAsTheyWere` into `unread` —
    whichever holds them; name it and why in NOTES;
  - `src/apply/fold.rs` only if V5 says Recover reaches it;
  - `tests/it/recover_bytes.rs` (new), `tests/it/main.rs` (one `mod` line); `MAP.md` (`apply` line for the raw step;
    `Operation::Recover`'s `Files` plan line).
- graft-fixtures, **only under R2**: `src/lost_work*.rs` (`build_with`; `build` calls it with `&[]`); `MAP.md` line 9.
- graft, **only under R2**: `tests/it/m7_walk/promise/recover.rs` (the build call; the later `git config` removed).
- Not: `src/operation/recover.rs` (the plan stays pure), `src/restore/*` (`write_raw` as it is), `src/apply/error.rs`
  (words unchanged), `src/journal_list*`, `src/snapshot*`, the journal types, anything in graft `src/`, `TESTING.md`.

## Signatures
```rust
// crates/graft-ops/src/apply/lost_edit.rs (new; crate-private)

/// The `raw/` tree holding his own bytes for a lost edit put back from `backup_tree`: the entry's
/// `worktree_raw_before`, only when its `worktree_before` is `backup_tree`. `None` → nothing to write (+0 spawns).
pub(super) fn his_bytes(entries: &[JournalEntry], entry: EntryId, backup_tree: Oid) -> Option<Oid>;

/// After the put-back's `restore --source` exited 0: his raw bytes over each of `paths` that `raw` holds, through
/// `restore::write_raw`. Runs with `ctx.stop` swapped for `&()`. Every spawn pushed to `commands`.
/// Err is `AfterState::NotAsTheyWere { error, files }` (R3's `files` rule), for `unread[0]`.
pub(super) fn write_back(
    ctx: &mut Context<'_>,
    raw: Oid,
    paths: &[/* RecoverTarget::Files's path type */],
    commands: &mut Vec<String>,
) -> Result<(), AfterState>;
```
```rust
// crates/graft-fixtures/src/lost_work*.rs — only under R2
/// `build`, with each `(key, value)` set by `git config` right after `init`, before the first commit.
pub fn build_with(graft: &mut dyn Graft, config: &[(&str, &str)]) -> Result<LostWork, LostWorkError>;
```
- Path conversion to `write_raw`'s `Vec<Vec<u8>>`: inside `write_back`, bytes as git names them (no lossy UTF-8).
- `EntryId`/`JournalEntry` as graft-ops names them today; if `Context` cannot hand out `&[JournalEntry]` without a
  signature change → **stop**, `blocked: apply cannot see the journal's entries`.

## Acceptance (name the verifying test at tick time; **(red)** = seen failing on today's code first, line in NOTES)
Tests in `graft-ops/tests/it/recover_bytes.rs`, real git, `std::fs::read` byte for byte. Shape (tests 1–6):
`core.autocrlf=true` **before the first commit**; committed `src/a.rs`; he edits it LF (`E1`); Graft discards it; he
re-edits LF (`LOST`); a forced `u` of the discard writes `E1` over `LOST` (a lost edit); `lost_edits` lists `src/a.rs`;
`apply(Recover(Files))` from it. Copy the shape from `recover::u_after_a_file_put_back_brings_back_what_it_replaced`.
- [x] V1–V5 in NOTES, each with its command and output. (NOTES § Verify)
- [x] (red) 1 `a_lost_edit_put_back_writes_his_lf_bytes_under_autocrlf`: after the put-back, `src/a.rs` is `LOST`.
      — `recover_bytes::a_lost_edit_put_back_writes_his_lf_bytes_under_autocrlf`; red at `recover_bytes.rs:115`.
- [x] 2 `u_after_a_lost_edit_put_back_gives_back_what_it_replaced_byte_for_byte`: then `u` → `E1`. Guard.
      — `recover_bytes::u_after_a_lost_edit_put_back_gives_back_what_it_replaced_byte_for_byte` (green before and after).
- [x] (red) 3 `redo_after_undoing_a_lost_edit_put_back_writes_his_lf_bytes`: then `u`, `U` → `LOST`.
      — `recover_bytes::redo_after_undoing_a_lost_edit_put_back_writes_his_lf_bytes`; red at `:153`.
- [x] (red) 4 `a_lost_edit_put_back_under_a_one_way_clean_filter_keeps_the_stripped_lines`: 30q-a's `strip` filter,
      `nb.txt` lost with `#out 42`; after the put-back it holds `#out 42`.
      — `recover_bytes::a_lost_edit_put_back_under_a_one_way_clean_filter_keeps_the_stripped_lines`; red at `:177`.
- [x] 5 `a_lost_edit_put_back_with_nothing_kept_raw_runs_no_raw_step`: autocrlf off; the entry's `commands` equal
      today's (no `ls-tree`/`cat-file` added). Guard for +0.
      — `recover_bytes::a_lost_edit_put_back_with_nothing_kept_raw_runs_no_raw_step` (asserts the one `restore` line).
- [x] 6 `a_lost_edit_put_back_whose_raw_step_cannot_land_says_so`: a file at `write_raw`'s temporary name (30s's
      setup) → `ApplyError::AfterState`, entry `Committed`, `unread == [NotAsTheyWere { WriteRaw { src/a.rs }, files: 1 }]`,
      words exactly `your edits are back · src/a.rs not written back as it was: <io> · its exact bytes are in the backup`,
      `src/a.rs` holds `LOST`'s content in CRLF, the file in the way untouched.
      — `recover_bytes::a_lost_edit_put_back_whose_raw_step_cannot_land_says_so`; red at `:218` (no `AfterState`).
- [x] 7 `a_lost_edit_lost_before_autocrlf_was_set_comes_back_as_git_checks_it_out_now`: R2's bound — autocrlf set
      after the forced `u`; after the put-back `src/a.rs` is `LOST` in CRLF. Pins the ADR line; red not expected.
      — `recover_bytes::a_lost_edit_lost_before_autocrlf_was_set_comes_back_as_git_checks_it_out_now` (green before and after).
- [x] Mutation: `write_back` skipped → 1, 3, 4 red again. Restored; said in NOTES with the lines. (NOTES § Mutation)
- [ ] `promise_recover_without_leaving` PASS (workspace run), its `recover.rs:123` assert and every other assert
      unchanged: `git diff` of the file empty, or (R2) only the build call and the config line.
      **Not ticked: blocked** (NOTES § Blocked). `git diff` of the file is empty; the walk is still red at `:123`.
- [ ] Unchanged and green in the full run (name them): every `recover::`, `undo_bytes::`, `snapshot_raw::`,
      `lost_work::`, `journal_list` test; graft `m7_walk::journal::the_three_lost_work_journeys`, `m7_walk::promise::`.
      **Not ticked: all but one.** Workspace `cargo nextest run`: 5013 run, 5012 passed, 1 failed =
      `m7_walk::promise::recover::promise_recover_without_leaving` (`recover.rs:123`, blocked). Green in that run (counts
      from `cargo nextest list`): `graft-ops::it recover::` 37, `undo_bytes::` 20, `snapshot_raw::` 13, `lost_work::` 3
      (+ graft-fixtures `lost_work::tests::` 6), `journal_list` 45, `recover_bytes::` 7, `apply::lost_edit::` 2,
      `m7_walk::journal::the_three_lost_work_journeys` 1, `m7_walk::promise::{guard,rewrite,trace}` 3.
- [ ] MAP.md lines. Gates workspace-wide.
      **Not ticked: gates not all green.** MAP lines done (graft-ops: `Operation::Recover` plan line, `## apply` file
      list, new `apply`, a lost edit's raw step line; graft-fixtures: `lost_work` line). `cargo fmt --check` pass,
      `cargo clippy --all-targets -- -D warnings` pass, `cargo test --doc` pass, `cargo nextest run` 5012/5013 (the
      blocked walk).

## Touches (each on the report's **Touches:** line, with how it was checked)
- **The Files-view put-back is this path.** 30s V5 ran `apply(Recover(Files))`, the same plan line; V5 here lists every
  graft call site building `Files` (keys, find, recover in `update/`) and shows each goes through `apply`. One fix.
- `u`/`U` of the put-back entry: unchanged code; tests 2, 3.
- `lost_edits` / J's rows: after the put-back the row goes as today (cleaned content matches); `recover::` tests green.
- Preview (`preview/recover.rs`), `in_the_way` (`written.rs`): unchanged (grep; no bytes drawn).
- Where he lands: success unchanged (`Done`). Failure: `Operated::Incomplete` — refresh and the sticky words, no cursor
  landing (30s §1 read (a), already a question to the conductor). No graft code changes.
- Journal: no new field or kind; golden lines byte-identical. Command Log: +2 spawns (`ls-tree`, `cat-file`) when `raw`
  is `Some`; +0 otherwise (test 5).
- Other `LostWork::build` callers (graft-ops `tests/it/lost_work.rs`, `recover.rs`, graft walks): unchanged under R2
  (`build` = `build_with(.., &[])`); grep and list.
- Relaunch: a put-back killed between the restore and the raw step leaves git's encoding (`recover_interrupted` runs no
  raw step) — the same gap candidate 30s named; NOTES, unverified.
- UI, keymap, captures: none (`grep -rn 'as it was' docs/shots`: none expected).
- KNOWN_GAPS (conductor's): `M7/30q-putback-fwd` closes on green (tests 1, 3, 4 and the walk); `M7/30s-raw-fail` widened
  to this path; ADR-0052 amendment (R2). TESTING.md's Recover "Fails today" is 26b's re-run, not this ticket.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~50k. If it compacts after the verify, say so; the conductor splits the build.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (rows restated above).

## Conductor (2026-10-01): R2 accepted
R2 stands: no fallback; the walk turns autocrlf on from the fixture's start. The mid-way case (autocrlf switched on after
the edit was lost → put-back returns CRLF) is a real byte change, so it is recorded as **broken** in KNOWN_GAPS
(`M7/30w-autocrlf-later`), pinned by R2's test, not hidden.

## NOTES (builder, 2026-10-01)
Base: merged `worktree-agent-a14300ec2a4a22d53` (26b, `141a00d1`) into this worktree at `93803bc2` (clean merge, the
conductor's instruction; the ticket's "builder does no merge" overridden by it). Scout used:
`.scratch/m7-power-tools/scouts/30w.md` (apply's seam, 30s's wiring, types, fold reach). Did not compact.

### Verify
Scratch tests in `recover_bytes.rs` (removed before commit), run with
`cargo nextest run -p graft-ops --test it recover_bytes::<name> --no-capture`.
- **V1, red.** Test 1 on today's code (26b + main): FAIL `recover_bytes.rs:115`, left (disk)
  `…123, 13, 10, 32, 32, 32, 32, 51, 13, 10, 125, 13, 10]` (CRLF), right (`LOST`) `…123, 10, …, 51, 10, 125, 10]`.
  Red: the cause is in graft-ops, not elsewhere.
- **V2, holds.** Same fixture, the row's entry printed:
  `entry=01HF7YAT010000000000000000 kind=Undo worktree_before=Some(444ab8bb…) worktree_raw_before=Some(444ab8bb…)
  backup_tree=444ab8bb…`; `backup_tree == worktree_before: true`; `git ls-tree -r 444ab8bb…` →
  `100644 blob ee0ffa4a…	src/a.rs`; `git cat-file blob 444ab8bb…:src/a.rs` → `LOST` in LF (`…123, 10, …, 51, 10, 125`,
  trailing LF trimmed by the helper). raw and worktree trees share an id here only because `src/a.rs` is the fixture's
  one file and its cleaned blob is its LF bytes; test 4 (one-way filter) is where they differ.
- **V3, R2 applies.** Walk: `crates/graft/tests/it/m7_walk/promise/recover.rs:83` `build()` (=
  `journal::lost_work()`, `m7_walk/journal.rs:94-95`, `LostWork::build(&mut AppOps)`), then `:86` `set_autocrlf(repo)`
  → `:133` `git config core.autocrlf true`. Fixture (lines before this ticket's edit):
  `graft-fixtures/src/lost_work.rs:194` `harden()` sets
  `core.autocrlf false` (`:286`); the forced undo is `:260-262` inside `build` (`graft_edit_steps`, `:216`). So autocrlf
  goes on **after** the undo's snapshot. Scratch, the walk's order (fixture built through graft-ops' `apply`/`undo`,
  then `git config core.autocrlf true`): `lost edit paths=["src/a.rs"] entry=01HRS2M2T00000000000000000 kind=Undo
  worktree_before=Some(9955b078…) worktree_raw_before=None backup_tree=9955b078…` → **None**, R2's fixture work is in.
- **V4, restore is the last writer.** V1's put-back entry's `commands` (before the fix):
  `git --literal-pathspecs restore --source=444ab8bb… --worktree -- src/a.rs` — the only line; disk after it is CRLF.
- **V5, one route.** `grep -rn "RecoverTarget::Files\|Operation::Recover" crates/graft/src`: graft never constructs
  `Files` (every hit is a pattern: `update/recover.rs:43,81,115,150`, `update/operate.rs:194,197`,
  `update/operate/outcome.rs:76`, `update/preview/keys.rs:169`, `update/preview/answer.rs:236,345,364`). graft-ops
  constructs it once: `journal_list/recover.rs:65` (`recover_for`, a `ListRow::LostEdit`) → `Recover::Run` →
  graft `update/journal/answers.rs:137-149` opens the preview with `Operation::Recover` → `update/preview/answer.rs:236-250`
  `launch_unstoppable` → `Command::Operate` (`update/operate.rs:165`) → `jobs/operate.rs:280` `apply::apply(ctx, op)`.
  One route, through `apply`. **Fold:** unreachable. `check` (`operation.rs:1633`) runs at `apply.rs:217` before
  `fold::folded_step` (`:222`); `check_conflict_state` refuses `InProgress` for every kind not in the exempt list
  (`operation.rs:1602-1615`), Recover not in it; the one other exception `amends_at_edit_stop` is `Operation::Amend`
  only (`operation.rs:369-373`). So `fold.rs` untouched.

### Build
- `graft-ops/src/apply/lost_edit.rs` (new): `after_put_back(ctx, op, went_through, commands)` (the one call),
  `his_bytes(entries, &EntryId, backup_tree)`, `write_back(ctx, raw, &[PathBuf], commands)`. Deviation from the
  Signatures: `his_bytes` takes `&EntryId` (not `Copy`; by value would clone for nothing); `after_put_back` is a third,
  crate-private wrapper so `apply.rs` has one call. Paths → `write_raw` via `as_encoded_bytes()` (raw bytes on unix,
  as `finish.rs:330` does).
- **Seam file: `apply.rs`, not `run.rs`/`record.rs`.** "Plan exited 0" (`failure.is_none()`), the owned `commands`,
  `finish` and the `unread` assembly are all in `apply.rs` `closed` (scout §1), so neither `run.rs` nor `record.rs`
  changed. The call sits after the status is decided and before `finish`; `NotAsTheyWere` goes first via
  `raw.into_iter().chain(unread)` into `succeeded`. When `finish` itself fails, the raw words go in
  `ApplyError::Journal.kept`, as 30s's do (rule 3: not dropped).
- **Lint-forced extraction in `apply.rs`:** `closed` was at clippy's 60-line limit; the call pushed it to 61+. Moved the
  existing status `match` into `fn outcome_status(failure, clashed, cleanup)` — same truth table ((None,_)/(Some,Some)
  → `Committed`, (Some,None) → `Aborted`), comment moved with it. No `allow` (no precedent in logic code).
- `graft-fixtures/src/lost_work.rs`: `build_with(graft, config)`, config set after `harden` (so it can override its
  `core.autocrlf false`), before the first commit; `build` = `build_with(graft, &[])`. Scratch check (not committed):
  `LostWork::build_with(&mut <graft-ops ops>, [("core.autocrlf","true")])` → the lost edit's entry has
  `worktree_raw_before=Some(d24b940b…)`, `backup_tree == worktree_before` (`9955b078…`), and after the put-back
  `src/a.rs == lost_work::LOST_EDIT` (true). That is the walk's step-4 assert at the ops level; the walk itself is not run
  this way (below).

### Mutation
`after_put_back` made to return `None` before `write_back` → `recover_bytes` 3 passed / 4 failed: 1 at `:115`, 3 at
`:153`, 4 at `:177` (and 6 at `:218`). Restored; 7/7 green. (Line numbers are the pre-rustfmt file's; rustfmt later split
test 1's `ls-tree` assert, +4 lines below `:127`.)

### Blocked
`blocked: needs crates/graft/tests/it/m7_walk/journal.rs`. R2 says the walk calls `build_with` from
`promise/recover.rs`, but the walk's build is `journal::lost_work()` (`m7_walk/journal.rs:94-95`), which hard-codes
`LostWork::build(&mut AppOps)`, and `AppOps` is private to `m7_walk::journal` (`:39`, a sibling module of
`m7_walk::promise`). `recover.rs` cannot name it. Smallest seam (either):
- `pub(crate) fn lost_work_with(config: &[(&str, &str)]) -> LostWork` in `journal.rs` (`lost_work()` =
  `lost_work_with(&[])`); then `recover.rs` imports it, calls `lost_work_with(&[("core.autocrlf", "true")])` and drops
  `:133`'s `git config` — the diff R2 names; or
- `AppOps` made `pub(super)`.
Not done: `promise/recover.rs` untouched (`git diff` empty); the walk is still red at `:123`, R2's bound (V3). Whether
steps 1–3 of the walk stay green under autocrlf-from-start is **unverified** (needs the seam).

### Other
- Relaunch: `recover_interrupted` (`apply/recover.rs`) calls no `write_raw` (grep: `write_raw(` only in
  `apply/lost_edit.rs`, `set_aside/put_back.rs:152`, `restore/steps.rs:337`), so a put-back killed between the restore
  and the raw step keeps git's encoding. Behaviour **unverified** (no kill test); gap candidate, as 30s named.
- Doc drift, not mine to fix (file excluded): `apply/error.rs` `AfterState::NotAsTheyWere`'s doc speaks of "a
  set-aside put-back" only; it now also covers a lost edit's put-back.

### Touches (how each was checked)
- Files-view put-back: the one route, V5 (grep, file:line above). One fix covers it.
- `u`/`U` of the put-back: tests 2, 3 green; `u`'s code untouched.
- `lost_edits` / J's rows: `graft-ops::it recover::` 37 green in the full run (incl.
  `a_put_back_leaves_no_lost_edit_row_and_u_brings_it_back`); `journal_list` 45 green.
- Preview (`preview/recover.rs`), `in_the_way` (`written.rs`): not edited; `write_raw(` callers grep above.
- Where he lands / graft: `git diff --stat -- crates/graft/src` empty. Failure lands as any `ApplyError::AfterState`
  (30s's `Operated::Incomplete` path) — not exercised through the app here, **unverified** end to end.
- Journal: no new field or kind; `journal_list` goldens green. Command Log: +2 (`ls-tree`, `cat-file --batch`) when
  raw is kept (test 1 asserts the three lines), +0 otherwise (test 5 asserts the single `restore` line).
- Other `LostWork::build` callers: `build` = `build_with(graft, &[])`; graft-fixtures `lost_work::tests::` 6 green
  (incl. `the_ids_are_the_same_every_build`), graft-ops `lost_work::` 3, `recover::` 37, graft `journal_screen`,
  `journal_put_back`, `m7_walk::journal` green in the full run.
- UI, keymap, captures: none. `grep -rln 'as it was' docs/shots` → 3 hits, all "the graph as it was" (tickets 05b, 08
  captions), not the `NotAsTheyWere` words.
- `apply.rs` `closed`: `outcome_status` extraction is behaviour-preserving (same truth table); every apply/stash/clash
  test green in the full run.
