# f-recover — the set-aside put-back gives back his bytes, so `J ⏎ y` then `u` holds an LF file under autocrlf

Status: done (2026-10-01) — built per **Ruling 2026-10-01**; NOTES `### Built` (the session compacted once, before the
build; nothing remains).
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
**Superseded by Ruling 2026-10-01 §4.**
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
**Amended by Ruling 2026-10-01 §2–§3** (`same_content_paths`'s return type and parameters; the failure bullet below).
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
- ~~A failed raw write fails the put-back through its existing failure path (the one a clash takes; name it in NOTES),
  worded `could not write <path> back as it was: <io> · u restores the backup`.~~ **Replaced by Ruling 2026-10-01 §1.**
  Never logged and carried on (rule 3) still holds.
- A relaunch that resumes a put-back (30d) goes through the same function, reading the entry from the journal. Check it
  (grep the callers of the put-back); if it does not, say so in NOTES.

## Acceptance (name the verifying test at tick time; **(red)** = seen failing on today's code first, line in NOTES)
Each test: `core.autocrlf=true`; committed `src/a.rs`; `W = b"pub fn a() -> u32 {\n    4\n}\n"` written LF, uncommitted
(30q-a's `lf_edit_under_autocrlf`). Assert with `std::fs::read`, byte for byte.
- [x] V1–V5 in NOTES, each with its command and output. (NOTES `### Verify`; V1 is
      `undo_bytes::u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf`, red; V2/V3/V5 scratch probes
      printed and removed; V4 the grep named there.)
- [x] (red) `undo_bytes::u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf`: Recover `MoveRef` of the
      checked-out branch over the dirty tree (the scenario's `J ⏎ y`; copy its shape from 26's `promise/recover.rs`), then
      `u`. After `u`: `W`. (Red at :478 before, NOTES `### Done`; green now, that test.)
- [x] (red) `undo_bytes::a_set_aside_put_back_keeps_an_lf_file_under_autocrlf`: the same operation. After it: `W`.
      (Red at :488 before; green now, that test — its porcelain expectation corrected, NOTES `### Built`.)
- [x] (red) `undo_bytes::a_switch_with_edits_keeps_an_lf_file_under_autocrlf`: a `Checkout` of another branch that
      does not touch `src/a.rs`, over the dirty tree. After it: `W`; after `u`: `W`. (A second set-aside kind.)
      (Red at :519 before; green now, that test.)
- [x] `undo_bytes::a_put_back_that_merges_into_a_changed_file_writes_no_raw`: the target branch changes `src/a.rs`
      elsewhere in the file, so the put-back merges his edit into new content. His raw bytes are **not** written: the
      file holds the target's change and his (content asserted, not bytes); `u` gives `W`. Guard; red not expected.
      (Green, that test; red at :565 when the content check is removed, NOTES mutations.)
- [x] `undo_bytes::a_clashed_put_back_writes_no_raw`: the put-back clashes (S kept). No raw write; the conflict is as
      today (compare with the existing clash test's asserts). Guard. (Green, that test.)
- [x] `undo_bytes::a_set_aside_under_a_one_way_clean_filter_keeps_the_stripped_lines`: 30q-a's `strip` filter,
      `nb.txt` with `#out 42`, set aside and put back. After the operation, `nb.txt` holds `#out 42`. (red)
      (Red at :630 before; green now, that test.)
- [x] Mutation: `write_raw` skipped in the put-back → the ~~three~~ **four** red tests red again. Restored; said in NOTES.
      (The four above red at `undo_bytes.rs:428`, 14 others green; restored run 18/18 — NOTES `### Built`, mutations.)
- [x] Added: `undo_bytes::a_set_aside_put_back_keeps_an_lf_file_with_a_quoted_name` (a name starting `"`, with a
      space and `é`: C-quoting is load-bearing — red when unquoted) and
      `undo_bytes::a_put_back_whose_raw_step_cannot_land_says_so_and_drops_s` (§1 end to end); `restore::raw::tests::a_path_is_c_quoted_for_stdin_paths`.
- [x] Unchanged and green in the full run (name them): every `undo_bytes::` and `snapshot_raw::` test; every
      `set_aside` test; `recover::` 30p-a's seven and test 6; `undo_cancel::`; graft `m3_walk::every_scenario::`,
      `m7_walk::journal::the_three_lost_work_journeys`. (Workspace `cargo nextest run --no-fail-fast`: 4992 run,
      4992 passed, 5 skipped, 0 failed — every one of these in it: `tests/it/{undo_bytes,snapshot_raw,set_aside,set_aside_kill,recover,undo_cancel}.rs`,
      graft `tests/it/m3_walk.rs`, `m7_walk/journal.rs`; journal golden line `journal::tests::a_line_without_raw_parses_as_none_and_renders_unchanged` included.)
- [x] MAP.md: `restore::raw` (`write_raw`, `same_content_paths`), the put-back's raw step. Gates workspace-wide.
      (MAP.md `## restore` intro + file list, `apply` set-aside line; fmt --check, clippy --all-targets -D warnings,
      nextest 4992/4992, test --doc all ok, workspace-wide.)
- Further ticks: Ruling 2026-10-01 §6.

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
- More: Ruling 2026-10-01 §6.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~60k. If it compacts after the verify, say so; the conductor splits the build. Remaining after the ruling: ~35k.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (rows restated above).

## Ruling 2026-10-01 (planner) — the block in NOTES `### Blocked`
The builder's three facts are taken as its NOTES state them (the planner reads no source). Its proposal is **accepted,
amended** as below.

### §1. A failed raw step: a completed write with one aspect not kept
The `StagingNotKept` shape (ADR-0045 amendment (b)), not a failed put-back.
- **Covers** `same_content_paths` failing (nothing written raw) and `write_raw` failing (it stops at its first failed
  path; later paths are left as git wrote them).
- **State.** `stash apply` clean and the status read clean → raw step (stop swapped to `&()`) → drop `S` by id
  **whatever the raw step returned** → entry `Committed` → `ApplyError::AfterState { applied, unread }` with
  `AfterState::NotAsTheyWere` **first** in `unread`. A failed drop adds `SetAsideKept` after it, as today. After the
  fallback, `StagingNotKept` follows it in `unread`.
- **Rejected: the existing failure path** (`PutBackRan::Failed` → `Aborted`, `EditsAt::Stash`). His content is on disk;
  "your edits are in stash@{n}" would send him to the wrong place (NOTES Blocked fact 3).
- **Rejected: keeping `S`.** It holds the same content in git's encoding: no bytes the disk lacks, and a false hint.
- **Words (new product words, decided here).** `<path>` shown as `StagingNotKept` shows its paths; `<io>`/`<error>` the
  error's own Display, never re-worded:
  - one path to write: `your edits are back · <path> not written back as it was: <io> · its exact bytes are in the backup`
  - more than one: `your edits are back · not all written back as they were: <path>: <io> · their exact bytes are in the backup`
  - the check failed (not `WriteRaw`): `your edits are back · not written back as they were: <error> · their exact bytes are in the backup`
  - **No `u` clause**, because of the next bullet. If `ApplyError::AfterState`'s `explain` appends a `u` clause to every
    `unread`, **stop**: `blocked: AfterState explain adds a u clause (error.rs:<line>)` — do not change other variants.
- **What `u` does from there.** It undoes the operation as for any `Committed` set-aside entry (ref, `HEAD`, index,
  content). It does **not** rewrite the path: its cleaned content is equal before and after, so `ChangedBetween` leaves
  it (the Why's reading), and a raw write that failed on its first path leaves exactly `main`'s state, where V1 saw `u`
  keep CRLF (`undo_bytes.rs:453`, NOTES V1). The exact bytes stay in the before commit's `raw/` tree, pinned by the
  backup (V2). No in-app route reaches them (V5: `J`'s `Files` writes git's encoding). Not fixed here: a `u` that
  writes raw bytes outside its scope needs a journal field naming the paths the operation failed to write, which this
  ticket's Rules forbid. → Conductor: gap row `M7/30s-raw-fail`, **unverified** (no test produces a real io failure),
  naming both facts.
- **Where he lands.** graft unchanged: the answer is `Operated::Incomplete` (the app maps only a lone `StagingNotKept`
  to its own arm; graft MAP `Operated`), which refreshes and shows the `error` string. Read-only, record in NOTES with
  the line: (a) whether `update/operate.rs`'s `operated` `Incomplete` arm lands the cursor on the new `HEAD` as `Done`
  does; (b) whether `resumed`'s `Incomplete` arm shows `error` or only `resumed_words` (graft MAP: "takes the same
  words"). If (b) drops it, a resumed rebase's raw failure shows as a clean tick: a **question** to the conductor with
  the line, not a fix here (it covers every `AfterState` on a resume).

### §2. The variant
```rust
// crates/graft-ops/src/apply/error.rs
pub enum AfterState {
    // … today's variants, unchanged …
    /// A set-aside put-back brought his edits back in git's encoding, and writing his own bytes over them failed.
    /// The entry is `Committed` and `S` dropped; `u` does not rewrite these bytes. `files`: how many paths the raw
    /// step was to write (1 → the singular words).
    NotAsTheyWere { error: RestoreError, files: usize },
}
```
If `RestoreError` is not reachable as a public type where `AfterState` is, hold `{ path: Option<PathBuf>, error:
String, files: usize }` built from its fields and Display; say which in NOTES.

### §3. Signatures (amend `## Signatures`)
```rust
// restore/raw.rs — Vec<Vec<u8>> accepted: write_raw's own path type, no round trip through PathBuf.
pub(crate) fn same_content_paths(ctx: &mut Context<'_>, raw: Oid, tree: Oid, commands: /* write_raw's type */)
    -> Result<Vec<Vec<u8>>, RestoreError>;
// restore.rs
pub(crate) use raw::{same_content_paths, write_raw};

// apply/set_aside/put_back.rs — `None` when either entry field is `None` (+0 spawns).
pub(super) struct HisBytes { pub(super) raw: Oid, pub(super) tree: Oid }
fn run_put_back(ctx: &mut Context<'_>, /* today's */, his: Option<HisBytes>, /* today's */) -> PutBackRan;
// apply/set_aside/returned.rs
fn put_back(ctx: &mut Context<'_>, /* today's */, his: Option<HisBytes>) -> /* today's */;
// apply/set_aside.rs `close`: builds HisBytes from pending.entry, passes &mut ctx.
```
- Visibilities as today. The raw step runs after **either** clean apply (`Whole`, the fallback's `Unstaged`), before
  the drop; never on `Clashed`/`Failed`. Stop swapped to `&()`: reuse `restore/rollback.rs` `uncancellable` only if
  reachable as it is, else the same swap inline in `put_back.rs`.
- Its reads go on the Command Log exactly as `u`'s `write_raw` logs its own: one rule for both.
- How the raw failure travels (a field on `PutBackRan`/`Returned`, or a variant) is the builder's, inside §4's files.

### §4. Files (replaces `## Files`), graft-ops only
- `src/apply/set_aside/put_back.rs`; `src/apply/set_aside/returned.rs`; `src/apply/set_aside.rs` (the `close` call
  site and the `&mut` it passes, nothing else); `src/apply/set_aside/answer.rs` (the raw failure into `unread`,
  nothing else); `src/apply/error.rs` (the variant, its words, its tests; no other variant's words change).
- `src/restore/raw.rs`, `src/restore.rs`, `src/restore/steps.rs` (the move, done).
- `src/apply/resume.rs` only if its `close` call must change (it should not; `close` already holds `&mut`); name it.
- `tests/it/undo_bytes.rs`; `MAP.md`.
- Not: `src/undo*.rs`, `src/snapshot*`, the journal types, `restore/rollback.rs`, `operation/recover.rs`, anything in
  graft. An exhaustive `match` on `AfterState` in graft that stops compiling → **stop**, `blocked: needs <graft
  file:line>`.

### §5. V5 is a gap, not this ticket
Different write (`operation/recover.rs:238-245`, `restore --source`), different raw source (the lost edit's own earlier
entry, not the running one), and the observed case had no raw bytes (`worktree_raw_before: None`). → Conductor:
`M7/30q-putback-fwd` **broken** as observed (NOTES V5), "with raw kept: unverified"; the follow-up reuses `restore::raw`.
V4's two questions (`stash push` then `pop` across operations; `stage.rs:188-196` `apply -R`) go to the conductor as
questions.

### §6. Added ticks and touches
- [x] `error.rs` test: the three wordings, exact strings. (`apply::error::tests::not_as_they_were_says_the_edits_are_back_and_where_the_bytes_are`.)
- [x] The mapping, at the seam in `returned.rs`/`answer.rs` tests with a built `RestoreError::WriteRaw`: a clean
      put-back whose raw step failed → `ApplyError::AfterState`, `unread[0]` `NotAsTheyWere`, entry `Committed`, `S`
      dropped. No failure hook in production code; end-to-end **unverified**, said in NOTES.
      (`apply::set_aside::answer::tests::a_clean_put_back_whose_raw_step_failed_is_committed_not_as_they_were_first`
      — Committed, `[NotAsTheyWere, StagingNotKept]` and alone; `…::an_unlisted_drop_after_a_failed_raw_step_reports_both_raw_first`;
      `…::a_git_failure_after_a_failed_raw_step_carries_its_words`. `S` dropped and end to end **verified** with no hook:
      `undo_bytes::a_put_back_whose_raw_step_cannot_land_says_so_and_drops_s`, NOTES `### Built`.)
- [x] NOTES: §1's two graft reads, each with its line. (NOTES `### Built`, "§1's graft reads".)
- [x] MAP.md: `AfterState::NotAsTheyWere`; the `apply` set-aside line gains the raw step. (MAP.md `apply` set-aside
      line, new `AfterState::NotAsTheyWere` line, `## restore` intro and the file list.)
- Touches: graft `jobs/operate.rs:369` lone-`StagingNotKept` mapping unchanged (grep); graft exhaustive matches on
  `AfterState` (grep); `docs/shots` has no `back as it was` / `as they were` (grep). ADR-0045 gets an amendment line
  for the raw step and the words: the conductor's.

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

### Built after the ruling (2026-10-01) — **this session compacted once** (after the block, before the build)
- **Shape.** `put_back.rs`: `HisBytes { raw, tree }`; `NotAsTheyWere { error: RestoreError, files }` (crate-private
  carrier); `run_put_back(ctx: &mut Context, stash, his: Option<HisBytes>, commands)`; the raw step `as_they_were`
  runs after each clean read (`Whole { raw }`, `Unstaged { paths, raw }`), stop swapped to `&()` inline
  (`rollback.rs`'s `uncancellable` is private to a file not granted), `same_content_paths` then `write_raw` (skipped
  when no path qualifies). `returned.rs`: `put_back(ctx: &mut, stash, index_base, his, commands)` — `his` before
  `commands` (crate convention: `commands` last); drops `S` whatever `raw` is; `raw` rides on `Returned::Back`,
  `Kept::Git` (`None` unless `back`), `Kept::Unlisted`; `Kept::into_error` returns `(error, raw)`. `answer.rs`:
  `Outcome::Back`/`Kept` carry `raw`; `NotAsTheyWere` inserted at 0 **after** `StagingNotKept`/`SetAsideKept`, so it is
  first. `error.rs`: `AfterState::NotAsTheyWere { error: RestoreError, files }` — `RestoreError` reachable, so §2's
  fallback shape was not needed; words from `not_as_they_were_words`, `path.display()` as `staging_named` does.
- **§1 stop check:** `ApplyError::AfterState`'s `explain` adds no `u` clause — it falls to `other.to_string()`
  (`error.rs` `explain`, the `other =>` arm) = `"{description}: {joined unread}"`. No stop.
- **Deviations (builder's calls, for the reviewer):**
  1. `set_aside.rs` beyond the call site: (a) the journal-failure arm of `close` puts the raw words first in
     `ApplyError::Journal.kept` (`Outcome::raw_words`), since that arm drops `unread` and the failure would otherwise
     be swallowed (rule 3); (b) its `Outcome::Back` patterns gain `..`; (c) one module-doc sentence for step 6.
  2. `answer.rs` beyond `unread`: an `ApplyError::Git` answer (the operation failed but the edits came back, or the
     drop failed with a git error) has no `unread`, so the raw words ride on git's line after `; ` (`with_raw`, via
     `appended`) — rule 3. Like the staging clause, they ride only on an `Exited` error; a non-`Exited` one (git gone,
     a kill between the operation and the drop) loses them: **gap candidate**, unverified, for the conductor.
     `status_for` (the `Aborted` reason) does not carry them — as for the `Committed` case, the after-state is not
     journaled. One pattern in `status_for` gains `..`.
  3. `c_quoted` and its unit test in `raw.rs` (the C quoting the ruling's §3 implies but does not name).
- **End-to-end raw failure is verified, not only at the seam.** The ruling expected no test could produce a real io
  failure; `a_raw_write_that_cannot_land_fails_u_and_names_the_path` already does (a file at the temporary's name,
  `write_his_bytes`' `create_new`), with no hook in production code. The same setup through the put-back:
  `undo_bytes::a_put_back_whose_raw_step_cannot_land_says_so_and_drops_s` → `ApplyError::AfterState`, entry
  `Committed`, `unread == [NotAsTheyWere { WriteRaw { src/a.rs }, files: 1 }]`, words start
  `your edits are back · src/a.rs not written back as it was: ` and end ` · its exact bytes are in the backup`,
  `main` on `second`, `S` dropped, `src/a.rs` is `W` in CRLF, the file in the way untouched. The check-failed
  wording was also seen end to end, in the quoting mutation below (git's own 128 exit, carried verbatim).
- **Mutations (each restored; the restored run green):**
  - `write_raw` skipped in the put-back (`as_they_were`'s first arm made always-`None`) → the four red tests red
    again, each at `assert_bytes` (`undo_bytes.rs:428`): `u_after_a_set_aside_put_back…`, `a_set_aside_put_back_keeps…`,
    `a_switch_with_edits_keeps…`, `a_set_aside_under_a_one_way_clean_filter…`; the other 14 `undo_bytes::` green.
  - the content check removed (`same_content_paths` keeps every path) → `a_put_back_that_merges_into_a_changed_file_writes_no_raw`
    red at `undo_bytes.rs:565`: the guard guards.
  - the `--stdin-paths` line written unquoted → `a_set_aside_put_back_keeps_an_lf_file_with_a_quoted_name` red:
    `your edits are back · not written back as they were: \`git hash-object --stdin-paths\` failed (128): fatal: could
    not open 'é q' for reading: No such file or directory · their exact bytes are in the backup`.
- **One test expectation fixed:** `a_set_aside_put_back_keeps_an_lf_file_under_autocrlf` asserted porcelain
  `"M src/a.rs"`; git prints `" M src/a.rs"` (blank index column = unstaged, what the test means). A test typo, not
  behaviour: the bytes assert above it passed on the first run with the fix.
- **§1's graft reads (read-only):**
  - (a) `update/operate/outcome.rs:390-391` `operated`'s `Incomplete` → `incomplete_answered` (:11) → `incomplete`
    (:26-40): sets the sticky `"<description> — could not confirm it fully: <error>"` (:32-35), `commit_editor::committed`,
    then `refresh_scope` (:39) **only**. It does **not** land the cursor as `Done` does (`done_answered` :336
    `landed(state, applied, lands, …)`, `lands` from `owed_landing` :361) and does not tag recovered rows
    (`recover::tag_rows` :338). So a `J ⏎ y` whose raw step failed refreshes but leaves the cursor where it was.
    **Question for the conductor** (graft, not this ticket; it covers every `Incomplete`).
  - (b) `update/banner/resume.rs:157-158` `resumed`'s `Incomplete` → `resumed_incomplete` (:297-310): shows `error`
    — `"{what} — could not confirm it fully: {error}"` (:306-308) — so a resumed rebase's raw failure is **not** a
    clean tick. It calls `refresh_scope` (:309) but not `finished_lands_on_head` (which `resumed_done` does, :358).
  - Both prefix graft's own `could not confirm it fully:` to the ruling's words; the words themselves arrive intact
    (`jobs/operate.rs:373-380` joins `unread` with `; `).
- **Touches, checked:** callers of `run_put_back`/`returned::put_back` only `returned.rs`/`set_aside.rs` (grep);
  `close` from `apply.rs:383` and `resume.rs:443`, `resume.rs` unchanged (compiles, not in the diff); graft
  `jobs/operate.rs:369` lone-`StagingNotKept` mapping unchanged, its `_` arm (:373) takes `NotAsTheyWere` to
  `Incomplete`; no exhaustive `AfterState` match anywhere (grep `AfterState::` in graft: :369 only; workspace clippy
  compiles every crate); `docs/shots` has no `back as it was`/`as they were` (grep: none); journal: no new field;
  Command Log: +3 spawns (`ls-tree` ×2, `hash-object`) +2 when a path qualifies (`ls-tree`, `cat-file`), only when
  `worktree_raw_before` is `Some` (+0 otherwise); ADR-0045 amendment line: the conductor's.
- **Not verified:** a resumed rebase under autocrlf (the same `close`, `resume.rs:443`; the Pending line carries
  `worktree_raw_before`, `journal/lines.rs:55`) — no test; the raw failure after a failed drop / failed operation
  end to end (seam tests only); relaunch (`apply/recover.rs`) runs no put-back, so a put-back killed before its raw
  step leaves git's encoding on disk — gap candidate, unverified.
