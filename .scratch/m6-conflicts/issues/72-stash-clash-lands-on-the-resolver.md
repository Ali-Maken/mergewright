# stash-clash-lands-on-the-resolver: a pop or apply that clashes is recorded and answered as a clash; done can drop it by id

Status: ready
Blocked by: 67 (merged)
From: pick 15 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24); review N3 (`docs/reviews/m6-usability-final.md`
:125-144).
Build with: **Opus** — a journal-format change (additive) and one pure rule with a table test; `apply`'s failure path
for stash kinds changes route.
Scout: `.scratch/m6-conflicts/scouts/stash-clash-lands-on-the-resolver.md` (done, main @ `fc3a0f47`).
Needs: none.
Crates: graft-core (journal), graft-ops (journal lines, `apply`, `live_clash`, `DropPutBack`, `Refusal::Unmerged` words),
graft **only** for: the journal fixtures that gain a field (scout :6) and one interim `ApplyError::Clashed` arm in
`update/operate.rs` (below). The app half is **72b** `stash-clash-lands-on-the-resolver-app`.
Group: **E1** (the ops half). 72b follows; then 73.

## Amended 2026-09-25 (planner, after scout 72)
- **Split** (scout's recommendation): this ticket is graft-core + graft-ops; 72b is the app (landing, close, stop line,
  names, relaunch). 72b `Needs:` this ticket.
- **Route** (scout :5): a clashed pop falls to `apply.rs:250-254` (`Aborted` + `Git`), because `stopped_on_conflict`
  (:361-377) returns `None` for kinds that do not start an operation. Decided: for `StashPop`/`StashApply` with
  `put_back` set, the failure branch **reads the index**; unmerged → entry `Committed` with the read `index_after`,
  answer `Clashed`. Not through `stopped_on_conflict`: a clash is not an operation in progress (no `c`, no `a`).
- **`live_clash` input** (scout :22): the app holds only `live_entries`. Decided: `live_clash` stays pure here; 72b
  calls it in the journal read job, which already reads the journal (graft MAP :430 says on launch, after every write,
  and on `Ctrl+R`; scout could not confirm — pinned in 72b).
- **Stop shape, landing, names**: 72b (decided there).

## Read first
- `.scratch/m6-conflicts/scouts/stash-clash-lands-on-the-resolver.md` :3-13.
- `docs/design-notes/m6-stash-put-back.md` "Interfaces".
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §1–3, §7 (last two bullets).
- `docs/research/commit-stash.md` addendum A1.
- `crates/graft-core/MAP.md` journal entries. `crates/graft-ops/MAP.md` :47-57, :86-94, :103-116, :161-179.

## Decided (ADR-0045 — not open here)
- A pop or apply writes `put_back: Some(PutBack { stash: <entry's commit>, why })` on its Pending line (`begin`,
  `apply/record.rs:50`).
- After the command, whatever git's exit code: index unmerged → `Committed`, `index_after` = the stage tree read,
  `ApplyError::Clashed { entry, clash }` with the unmerged paths. Nothing unmerged and git failed → today's `Aborted` +
  `Git` (an untracked file in the way).
- `live_clash`: ADR-0045 §2, pure.
- `Operation::DropPutBack { stash }`: `stash drop stash@{n}`, `n` looked up by commit in the facts at apply; refused
  `NoSuchStash` when gone; journal kind `StashDrop`, `pins: [stash]`, so the existing `StashStore` inverse brings it back.
- `Refusal::Unmerged` words (`operation.rs:550-553`): `<path> still has conflicts · resolve it first` (`+N` when more).
  They reach the screen through 67's `refusal_status` (`graft/src/update/operate.rs:954`, callers `:928`,
  `preview.rs:648`): unchanged.
- **Interim app arm** (so the workspace builds): `ApplyError::Clashed` in `operate.rs` sets
  `Sticky("<description> left conflicts in N files")` and nothing else. 72b replaces it. NOTES says so; not a gap
  (72b is sequenced next).

## Scope
**In:** the journal field; the stash failure branch reading the index; `Clashed`, `Clash`, `StashClash`; `live_clash`;
`DropPutBack`; the `Unmerged` words; the interim arm; fixtures; tests.
**Out:** everything the user sees beyond the interim arm (72b); set-aside (73).

## Signatures
```rust
// graft-core, journal — additive; old lines parse as None and re-render byte-identically
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutBack { pub stash: Oid, pub why: PutBackWhy }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum PutBackWhy { Pop, Apply, SetAside }
// JournalEntry gains:
#[serde(default, skip_serializing_if = "Option::is_none")] pub put_back: Option<PutBack>,

// graft-ops
// PendingLine gains `put_back: Option<PutBack>` (same serde rule); `begin` takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clash { pub stash: Oid, pub why: PutBackWhy, pub paths: Vec<PathBuf> }
// ApplyError gains (not retryable, like Conflicted):
Clashed { entry: Box<JournalEntry>, clash: Clash },
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashClash { pub entry: EntryId, pub stash: Oid, pub index: Option<usize>, pub why: PutBackWhy }
/// ADR-0045 §2. Pure. `stashes`: (commit, index), newest first.
pub fn live_clash(entries: &[JournalEntry], in_progress: Option<InProgress>, any_unmerged: bool,
                  stashes: &[(Oid, usize)]) -> Option<StashClash>;
// Operation gains:
DropPutBack { stash: Oid },
```

## The failing tests first
- graft-core: `a_journal_line_without_put_back_re_renders_byte_identically`; `put_back_round_trips`.
- graft-ops, real repo: `a_pop_that_clashes_answers_clashed_and_keeps_the_stash` (entry `Committed`, `index_after`
  unmerged, `put_back = Pop(<commit>)`, stash list unchanged); `an_apply_that_clashes_records_apply`;
  `a_failed_apply_with_nothing_unmerged_is_not_a_clash` (`Aborted` + `Git`); `drop_put_back_drops_by_commit_id_not_position`;
  `drop_put_back_refuses_when_the_stash_is_gone`; `undo_of_drop_put_back_stores_it_again`;
  `undo_of_a_clashed_pop_restores_the_tree_and_keeps_the_stash`.
- graft-ops, pure: `live_clash_table` (rows as ADR §2: nothing unmerged; in progress; pop clash; after a take
  (`index_before` unmerged); an undo back into the clash; shell pop, no record; stash gone; residual shell-after-apply).
- graft-ops: `the_unmerged_refusal_names_no_shell_and_no_milestone`.

## Touches (each entry, and how the builder checks it)
- **Journal constructors** (scout :6: `apply/record.rs:50,101`, `journal/lines.rs:114,135-136`, `apply/fold.rs:134`, ~25
  fixtures across graft-ops, graft and `tests/it`): each passes `put_back`. Check: build; list in NOTES.
- **`ApplyError` matches**: every one gains `Clashed`. Check: build; no new `_ =>` arm (grep the diff).
- **`stopped_on_conflict`** (`apply.rs:361-377`): unchanged. Check: existing merge/rebase conflict tests green.
- **Undo/redo** (m6-conflicts/07 F2; `record.rs:313` pins): Check: the undo tests above and the stash undo suite.
- **`in_force`** for a `Committed` clash entry: in force, so `u` reaches it. Check: `undo_of_a_clashed_pop…`.
- **`Refusal::Unmerged` surfaces** (raised `operation.rs:907`; shown via `refusal_status`): Check: grep of the old text
  in `crates/` returns nothing.
- **Today's stash-clash tests** (`graft/tests/it/resolver/stash_pop.rs:12,74`): they drive a clash; the interim arm
  changes the status only. Check: green, or NOTES names the assertion and why it moved.
- **Journal browser `J`**: a `Committed` pop with conflicts and a `DropPutBack` row. Check: a `journal_view` test.
- **MAP.md**: graft-core `PutBack`; graft-ops `Clashed`, `live_clash`, `DropPutBack`, the stash failure branch, `Unmerged`.

## Acceptance (name the verifying test at tick time)
- [ ] Every test above, red then green (round-trip tests may be green-first; say so).
- [ ] Pin (scout could not confirm): what `finish` records as `index_after` on the stash failure branch. The clash test
      asserts `index_after == IndexState::Unmerged(<stage tree>)` read from the journal file, and NOTES names the line
      that records it.
- [ ] NOTES: the interim arm's file:line, for 72b.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large (~55k): a field across ~30 constructors, one failure branch, one pure rule, one op.
