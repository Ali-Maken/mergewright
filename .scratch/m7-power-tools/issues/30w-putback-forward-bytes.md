# putback-forward-bytes — `J n ⏎ y` puts a lost edit back in his own bytes, where the backup kept them

Status: ready
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
- [ ] V1–V5 in NOTES, each with its command and output.
- [ ] (red) 1 `a_lost_edit_put_back_writes_his_lf_bytes_under_autocrlf`: after the put-back, `src/a.rs` is `LOST`.
- [ ] 2 `u_after_a_lost_edit_put_back_gives_back_what_it_replaced_byte_for_byte`: then `u` → `E1`. Guard.
- [ ] (red) 3 `redo_after_undoing_a_lost_edit_put_back_writes_his_lf_bytes`: then `u`, `U` → `LOST`.
- [ ] (red) 4 `a_lost_edit_put_back_under_a_one_way_clean_filter_keeps_the_stripped_lines`: 30q-a's `strip` filter,
      `nb.txt` lost with `#out 42`; after the put-back it holds `#out 42`.
- [ ] 5 `a_lost_edit_put_back_with_nothing_kept_raw_runs_no_raw_step`: autocrlf off; the entry's `commands` equal
      today's (no `ls-tree`/`cat-file` added). Guard for +0.
- [ ] 6 `a_lost_edit_put_back_whose_raw_step_cannot_land_says_so`: a file at `write_raw`'s temporary name (30s's
      setup) → `ApplyError::AfterState`, entry `Committed`, `unread == [NotAsTheyWere { WriteRaw { src/a.rs }, files: 1 }]`,
      words exactly `your edits are back · src/a.rs not written back as it was: <io> · its exact bytes are in the backup`,
      `src/a.rs` holds `LOST`'s content in CRLF, the file in the way untouched.
- [ ] 7 `a_lost_edit_lost_before_autocrlf_was_set_comes_back_as_git_checks_it_out_now`: R2's bound — autocrlf set
      after the forced `u`; after the put-back `src/a.rs` is `LOST` in CRLF. Pins the ADR line; red not expected.
- [ ] Mutation: `write_back` skipped → 1, 3, 4 red again. Restored; said in NOTES with the lines.
- [ ] `promise_recover_without_leaving` PASS (workspace run), its `recover.rs:123` assert and every other assert
      unchanged: `git diff` of the file empty, or (R2) only the build call and the config line.
- [ ] Unchanged and green in the full run (name them): every `recover::`, `undo_bytes::`, `snapshot_raw::`,
      `lost_work::`, `journal_list` test; graft `m7_walk::journal::the_three_lost_work_journeys`, `m7_walk::promise::`.
- [ ] MAP.md lines. Gates workspace-wide.

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
