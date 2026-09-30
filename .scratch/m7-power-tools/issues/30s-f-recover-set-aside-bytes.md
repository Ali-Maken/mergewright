# f-recover — the set-aside put-back gives back his bytes, so `J ⏎ y` then `u` holds an LF file under autocrlf

Status: ready
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
- [ ] V1–V5 in NOTES, each with its command and output.
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
