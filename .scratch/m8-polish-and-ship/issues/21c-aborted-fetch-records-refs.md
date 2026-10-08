# 21c — a fetch records the refs it moved; J stops saying "nothing written" when one moved

Status: **partial** — Part A (record: boxes 1–8, 12) done 2026-10-08; Part B (show: boxes 9–11) open, handoff in NOTES.
The builder compacted mid-ticket, so it split at Part A | Part B as Size says. (Planner spec-only pass 2026-10-08;
amended after scout `.scratch/m8-polish-and-ship/scouts/21c-fetch-refs.md`.)
Build with: **Opus (xhigh)** — `builder-opus-xhigh`, the undo area: it edits the journal line format and adds a
predicate beside `in_force`, which drives `u`. **Not Fable**: the approach is decided here (ADR-0062), and correctness
is judged by tests (the red test, the predicate table, `next_undo`/`revert_to` unchanged) — test 3 fails. Scout done
(file above); the conductor may revise this line after it.
Needs: none (no other ticket's signature). Decision: `docs/adr/0062-a-fetch-records-the-refs-it-moved-outside-the-undo-record.md`
(approved by the user).
Row closed on merge: **M8/aborted-refs-moved** (and proposed M8/aborted-fetch-not-in-force, same defect).

## The defect (21b-3 NOTES item 7, observed)
`LocalRemote` (Linear) + `commit_on_remote`, second remote `gone` = `file:///nonexistent/…`,
`apply(Fetch { remote: None, prune: false })` → `Err`; `origin/main` **moved** to the teammate's tip. Entry: `kind: Fetch`,
`status: Aborted { reason: "no repository at '/nonexistent/graft-21b-3.git', or no access to it" }`, `touches.refs: []`,
`refs_before: {}`, `refs_after: {}`, `worktree_before: None`, `commands: ["git fetch --progress --all"]`. `took_effect`
(graft-ops `journal.rs:162`, Aborted arm `:165`) is false → J marks it `RowMark::Refused` (graft `view/journal.rs:198`),
drawn `refused · nothing written` (graft-ui `journal_screen/view.rs:42`). A `Committed` fetch records no refs either.
`u` unaffected (`undoable_kind` `journal.rs:225` skips `Fetch`). The probe's test was **never committed** (no
`tests/it/aborted_fetch.rs`); box 1 writes it new.

## Decisions (ADR-0062; summary)
1. **Record:** one entry; a `for-each-ref` of `refs/` (minus `refs/graft/`) before `git fetch` and again after it exits
   (success, failure, stop); the differing refs go on the **outcome line** as `fetched_refs`. Fetch, and a pull's
   fetch line. Not per-remote entries.
2. **`u`: no.** `fetched_refs` is outside the undo record. `in_force`, `took_effect`, `changed_anything`, `touches`,
   `refs_before`, `refs_after` unchanged for fetch and pull. An Aborted fetch stays **not in force** — deliberately (an
   in-force Aborted *pull* would become `next_undo` with an empty inverse).
3. **J's mark** reads a new predicate, `wrote_anything`, not `in_force`.
4. **Words** (the mark is whatever `RowMark` variant and label HEAD has: `Refused` / `refused · nothing written` at
   26f1286f; 20b Part B renames it `NothingWritten` / `nothing written` — use the name at HEAD, change neither):
   - status (failure or stop of a fetch/pull that moved refs): today's `remote::explain` words + ` · origin/main moved
     before it stopped` (`origin/main moved, origin/feat new +3 more before it stopped`). Success status (`fetched`)
     unchanged.
   - J `●` fetch row summary (`view/journal/summary.rs:57–61`, today `remote-tracking refs · not undoable` +
     `with_failure`): the first clause becomes `<moves>`, ` · not undoable` and `with_failure` kept as today.
     `<moves>` = `moved_words(..)` (+ ` before it stopped` when Aborted) when `fetched_refs` is non-empty;
     `nothing new` when `Some({})` and Committed; **today's `remote-tracking refs`** when `None` (old lines) or
     `Some({})` and Aborted. E.g. Aborted partial fetch: `failed: no repository at '…' … · origin/main moved before it
     stopped · not undoable` in `with_failure`'s order, **no** mark. Aborted fetch that moved nothing → exactly today's
     row and mark.
   - J pull row: today's summary + ` · <moved_words>` (` before it stopped` when Aborted), only when non-empty.
   - J expanded row: after the failure, before `commands Graft ran`: heading `refs the fetch moved · u does not move
     them back` (`u` spelled from the keymap, `app.undo`; unbound → `refs the fetch moved`), then `┆ origin/main
     a1b2c3d → e4f5a6b`, `┆ origin/feat  new at e4f5a6b`, `┆ origin/old  gone, was a1b2c3d`, `┆ tag v1.2  new at …`;
     at most 20 lines, then `┆ +N more`.
   - Why not "fetched origin; gone failed": a remote that brought nothing reads the same as one that failed; naming
     the failed one means parsing git's per-remote stderr. The failure words already name `gone`'s location (20b R3).

**Journey** (no new screen; J's existing expanded-row pattern — no prior-art study exists for J, and this adds no
interaction): `f` → status line says it failed **and** what moved (0 more keys) → `J` (1) row says the same →
`space` (2) lists every ref. `u` from anywhere still takes back the last local change (ADR-0037 §3).

## Read first
- The scout file (whole, 1 page) — its line numbers are at 26f1286f; re-check at your base.
- `docs/adr/0062-…` (whole); ADR-0037 §3; `docs/design/06-undo-journal-design.md` §6.1, §6.2, §6.4.
- `crates/graft-ops/MAP.md` `## journal` (:106–124), `## undo` `inverse`/`plan_revert_to` (:135–137), `## apply`
  `apply` (:266), a pull's fetch first (:242), `ApplyError` `Remote` (:251), `Context` (:237).
- `crates/graft-core/MAP.md` `## journal` (:50–62).
- `crates/graft/MAP.md` `journal_view` (:947), `remote_done`/`remote_failed` (:621).
- `crates/graft-ui/MAP.md` `## journal_screen` (:252–268).
- `.scratch/m8-polish-and-ship/issues/20b-words-rulings.md` R8 (:37) — the mark.
- Source after MAP.

## Scope
**In**
- **Part A — record (graft-core, graft-ops).**
  1. `RefMove` + `JournalEntry.fetched_refs` (graft-core); `OutcomeLine.fetched_refs` (graft-ops `journal/lines.rs`,
     beside `merge_after`/`config_after` :96–101; carried by `OutcomeLine::of`, `apply_to`, `into_entry`), serde
     `default` + `skip_serializing_if = Option::is_none`: every other line renders byte-identically.
  2. `apply/fetched.rs` (new, crate-private): `ref_store` (one `for-each-ref --format=%(objectname) %(refname) refs/`,
     `refs/graft/` dropped) and `moved` (pure diff).
  3. **Not on the Command Log.** `begin`'s before-reads and `read_after` are unlogged (`record.rs:84–89`, `:~127`,
     `:249+`; `finish` sets `commands` to the plan lines only). The two listings are before/after-state reads of the
     same kind, so they stay off `commands` too. It does not affect what they record: they run through the same
     runner and stop as `begin`'s reads. So `remote_ops.rs:173` and `pull_set_aside.rs:89` stay unchanged.
  4. **Plain fetch:** the entry opens at `begin` (`apply.rs:285`) before git runs; git runs in `pull::run_rest`
     (`pull.rs:~95`, `Ahead::Nothing`). `ref_store` before `begin` (fails → `ApplyError::Facts`, no entry, nothing
     ran); again right after the fetch line exits, whatever its outcome; `fetched_refs = Some(moved(..))` on the
     entry before `finish` appends the outcome line (`record.rs:237`).
  5. **Pull** (the fetch runs **before** its entry opens, `pull.rs:41–59`, `run_plan` :50): `fetch_first` takes
     `ref_store` before `run_plan` (fails → `ApplyError::Facts`, no entry, nothing ran — today's no-entry shape for a
     read before the write) and again right after it returns, Ok or failed. It carries the `moved` map out in its
     result (the `Ahead` value or beside it) to `apply`. `apply` sets it on the entry after `begin`/`open_after`
     opens it (`pull.rs:67–81` for a failed fetch), so it lands on the **outcome** line. The pending line holds no
     before-fields for it, so opening late loses nothing. The merge's branch move is not in it (taken before
     `run_rest`). The fetch-ok-but-incoming-read-failed path (`pull.rs:54–57`) still opens no entry: **out of scope**
     (proposed row below).
  6. After-read failing: `fetched_refs: None`. On a successful fetch/pull that is `AfterState::Refs` as for today's
     unread refs. On a remote failure the `Remote` error stands, and `remote::explain` (`apply/remote.rs:315`) adds
     ` · which refs it moved could not be read: <error>`.
  7. `journal::wrote_anything`, `journal::moved_words`, `journal::moved_lines` (pub). **`changed_anything`
     (`journal.rs:173–186`, private) is left as is**, with both callers (`took_effect` :165, `is_no_op_pull` :245):
     `wrote_anything` sits beside it and calls `took_effect`, which still calls it. `remote::explain` appends
     ` · <moved_words> before it stopped` when the entry's `fetched_refs` is non-empty. `remote::short` (the entry
     reason, :290) is unchanged.
  8. `mark_interrupted` (`journal/file.rs:177`) clones the held entry: `fetched_refs` stays `None` (nothing to write).
     `recover_interrupted` (`apply/recover.rs:31`): no literal; check that it doesn't set the field.
- **Part B — show (graft, graft-ui).**
  9. `view/journal.rs:184–206`: the `Refused` arm (:198) fires iff `Aborted` ∧ `!wrote_anything` ∧ not reversed. The
     `in_force` term is swapped and nothing else changes: variant, label and the `Undone` arm stay as at HEAD.
     Summary words as Decision 4 (`view/journal/summary.rs:57–61`, `with_failure`, `failure` :133–150).
  10. graft-ui `JournalRowView.moved` (`journal_screen/view.rs:~70–101`), `moved_heading`; drawn only while expanded,
      in `journal_screen/pieces.rs:204–208` between `failure` and `COMMANDS_HEADING`, `┆ `-led like the commands,
      cut by `cut_at_word` at the pane width.
  11. MAP.md: graft-core, graft-ops, graft, graft-ui rows. DESIGN §6.1 (`fetched_refs` line in the struct), §6.2
      step 4 (`Nothing needs undoing; a fetch's moved refs are recorded, ADR-0062`), §1.10 mockup row →
      `fetch · origin/main moved`.

**Out** — `u`/`U`/`⏎` on a fetch row (unchanged: nothing); the success status `fetched`; push (one ref, named by its
description; a rejected push moves nothing); commit counts; the `RowMark` name and label (20b's); the pull whose
incoming read fails after its fetch (no entry — proposed row `M8/pull-incoming-read-unjournaled`, **broken by
design**, `pull.rs:54–57`, doc :37–39); regenerating earlier milestones' captures (records, not live).

## Signatures
```rust
// graft-core — src/journal (beside PutBack/MergeState), re-exported at graft_core::journal
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RefMove { pub before: Option<Oid>, pub after: Option<Oid> }   // None = absent (new / gone)
// JournalEntry, new field:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub fetched_refs: Option<BTreeMap<String, RefMove>>,  // Some(empty) = read, nothing moved; None = not recorded

// graft-ops — src/journal.rs (pub). Sits beside the private changed_anything (unchanged).
pub fn wrote_anything(entry: &JournalEntry) -> bool;
//   took_effect(entry)
//   || entry.fetched_refs.as_ref().is_some_and(|m| !m.is_empty())
//   || (entry.fetched_refs.is_none() && matches!(entry.kind, Fetch | Pull)
//       && matches!(entry.status, EntryStatus::Aborted { .. }))      // unknown ≠ unchanged
pub fn moved_words(moved: &BTreeMap<String, RefMove>) -> Option<String>;     // None when empty
//   "origin/main moved" · "origin/main moved, origin/feat new" · "…, … +3 more" · "tag v1.2 new" · "origin/old gone"
//   order: the map's (full ref name); shorthand via RefName::shorthand; refs/tags/* → "tag <short>"
pub fn moved_lines(moved: &BTreeMap<String, RefMove>, at_most: usize) -> Vec<String>;
//   "origin/main  a1b2c3d → e4f5a6b" · "origin/feat  new at e4f5a6b" · "origin/old  gone, was a1b2c3d"; then "+N more"
// OutcomeLine (journal/lines.rs), new field — same type and serde attributes as JournalEntry.fetched_refs.

// graft-ops — src/apply/fetched.rs (new, crate-private); lifetimes/stop as begin's reads (record.rs:84–89) take them
pub(crate) fn ref_store(ctx: &Context) -> Result<BTreeMap<String, Oid>, RunError>;
pub(crate) fn moved(before: &BTreeMap<String, Oid>, after: &BTreeMap<String, Oid>) -> BTreeMap<String, RefMove>;
// graft-ops — apply/pull.rs: fetch_first's result carries `Option<BTreeMap<String, RefMove>>` (None: after-read
// failed) to apply; the shape (a field on Ahead or a tuple) is the builder's, crate-private.

// graft-ui — journal_screen
pub struct JournalRowView { /* … */ pub moved: Vec<SanitizedText> }      // empty → nothing drawn
pub fn moved_heading(keys: &Keymap) -> String;  // "refs the fetch moved · u does not move them back"; u unbound → first clause
```

## Touches (check each; the builder's **Touches:** line says how)
- **Undo** — `inverse`, `drift`, `state_at`, `PointInTime`, `took_effect`, `changed_anything`, `in_force`,
  `in_force_all`, `next_undo`, `next_redo`, `plan_revert_to`, `clash_candidate`, `stage_source`: must not read
  `fetched_refs`. Check: `rg -n fetched_refs crates/graft-ops/src` lists only `journal.rs` (`wrote_anything`, words),
  `journal/lines.rs`, `apply/fetched.rs`, `apply/pull.rs`/`apply.rs` (setting it), `apply/remote.rs` (words).
  Also `journal/tests.rs:460` `next_undo_steps_past_a_push_and_a_fetch` must stay green.
- **Journal file** — old lines parse and re-render byte-identically (box 4); `render_outcome` :137; no version field,
  compat by serde attributes as `merge_after`/`config_after`.
- **`JournalEntry {…}` literals** — graft-ops 46, graft 45 (scout count; re-count at your base, quote both before and
  after). `OutcomeLine {…}`: only `OutcomeLine::of`.
- **J** — mark rule (`view/journal.rs:198`), summary (`summary.rs:57–61`), expanded row (`pieces.rs:204–208`),
  `enter_words` (unchanged: none on a fetch row), the cursor-preselect rule (keymap §2.11 "except a fetch":
  unchanged), `/` find (now matches ref names in the summary — say so in NOTES). Tests asserting `remote-tracking refs`
  row text: `graft/src/view/journal/tests.rs`, `graft-ui/src/journal_screen/tests.rs`. A Committed fetch's text
  changes (`nothing new` / moves), so update those tests and list each.
- **Status** — `remote::explain`; graft `remote_failed` shows it (no graft change expected; check).
- **Command Log** — unchanged (Scope 3). `remote_ops.rs:173` and `pull_set_aside.rs:89` stay green, unedited.
- **Captures** — new `docs/shots/m8/21c-*`. `m4-07-journal-remote-rows-*` are records and are not regenerated. Their
  harness (`graft/tests/it/m4_walk/hangs_and_undo.rs`, `coverage.rs`) still runs: if it asserts fetch/pull row text,
  update it and list the change.
- **Keymap** — none bound; `moved_heading` spells `u` from `app.undo` (W1/W3).
- **Other crates** — graft-git, graft-graph, graft-fixtures: none (the read goes through graft-ops' existing run path).
- **Docs** — four MAP.md files; DESIGN §6.1/§6.2/§1.10; KNOWN_GAPS is the conductor's.

## Acceptance (each tick names its test, at tick time)
- [x] 1. **Red first — new test, new file** `crates/graft-ops/tests/it/aborted_fetch.rs` (+ its `mod` line in the
      `tests/it` root). Test `a_fetch_that_moved_refs_before_one_remote_failed_records_them`, set up from the defect
      section: `origin/main` moved; entry `Aborted`; `fetched_refs == Some({"refs/remotes/origin/main": RefMove {
      before: old, after: teammate tip }})`; `wrote_anything` true; **`in_force` false**. It is written before Part A's
      code; NOTES quotes its red (compile or assert).
      — **ticked by `aborted_fetch::a_fetch_that_moved_refs_before_one_remote_failed_records_them`** (green; red quoted
      in NOTES; also reads the entry back from the journal file and asserts the real error's `explain` ends
      ` · origin/main moved before it stopped`).
- [x] 2. `aborted_fetch::a_fetch_that_brought_nothing_records_an_empty_map` — Committed, `Some({})`.
      — **ticked by `aborted_fetch::a_fetch_that_brought_nothing_records_an_empty_map`**.
- [x] 3. `aborted_fetch::a_fetch_records_new_moved_pruned_and_tag_refs` — one fetch with `prune: true`: a moved, a
      new, a pruned (`after: None`) branch and an auto-followed tag, each exactly once; no `refs/graft/` key.
      — **ticked by `aborted_fetch::a_fetch_records_new_moved_pruned_and_tag_refs`** (a `+refs/heads/*:refs/graft/fetched/*`
      refspec makes git write under `refs/graft/`, and `origin/HEAD` is set; the map is compared whole. Mutation
      check: with the symref and `refs/graft/` filters removed it fails with those two extra keys).
- [x] 4. `journal::tests::a_line_without_fetched_refs_re_renders_byte_identically`;
      `journal::tests::fetched_refs_round_trip` (`Some({})` renders `"fetched_refs":{}`; a map round-trips).
      — **ticked by both** (the first was green before any format change and stays green; it now also asserts every
      old entry reads `fetched_refs: None`. The second checks the pending line has no `fetched_refs`).
- [x] 5. `journal::tests::wrote_anything_table`. Rows:
      - Aborted fetch, `Some({})` → false.
      - Aborted fetch, `Some(1 ref)` → true.
      - Aborted fetch, `None` → true.
      - Committed fetch, `Some({})` → true.
      - Aborted commit with no backup → false (today's rule untouched).
      - Aborted pull, `Some(1 ref)` → true **and** `in_force` false.
      — **ticked by `journal::tests::wrote_anything_table`** (all six rows).
- [x] 6. `aborted_fetch::u_after_a_partial_fetch_takes_back_the_change_before_it` — commit, partial fetch, `next_undo`
      = the commit's entry; `plan_undo` steps hold no `refs/remotes/` ref.
      — **ticked by `aborted_fetch::u_after_a_partial_fetch_takes_back_the_change_before_it`** (also `plan.touches.refs`).
- [x] 7. `aborted_fetch::a_pull_whose_fetch_failed_records_what_it_moved` (pull from a remote whose fetch moves one ref
      then fails, e.g. a tag git refuses to clobber; **if no fixture can make a pull's fetch fail part-way, say so in
      NOTES and test the failing-fetch shape with `Some({})`**) and `aborted_fetch::a_diverged_pull_records_its_fetch_but_u_is_unchanged`
      (fetch ok, `merge --ff-only` refuses → Aborted): `fetched_refs` holds `origin/main` only (not the local
      branch); `next_undo` same as before the pull; `plan_revert_to` across it unchanged (plan compared before/after).
      — **ticked by `aborted_fetch::a_pull_whose_fetch_failed_records_what_it_moved`** (a real part-way failure: a
      `refs/tags/*:refs/tags/*` refspec and a tag git refuses to clobber; no `Some({})` fallback needed) **and
      `aborted_fetch::a_diverged_pull_records_its_fetch_but_u_is_unchanged`** (`plan_undo` and `plan_revert_to`
      compared equal before/after).
- [x] 8. The probe's `remote::explain` ends ` · origin/main moved before it stopped` (test beside `remote::explain`'s
      existing tests, name in NOTES). `journal::tests::moved_words_table`: 0 refs → None; 1; 2; 3+ with `+N more`;
      new; gone; tag. `moved_lines` caps at `at_most`, then `+N more`.
      — **ticked by `apply::remote::tests::a_failed_fetch_that_moved_refs_ends_with_them`** (also: `Some({})` → today's
      words; `moves_unread` → ` · which refs it moved could not be read: <error>`), **`journal::tests::moved_words_table`**
      and **`journal::tests::moved_lines_cap_at_at_most`**.
- [ ] 9. graft `view::journal::tests`:
      - `a_partial_fetch_row_names_what_moved_and_is_not_refused` — no `refused · nothing written`.
      - `a_fetch_that_moved_nothing_reads_as_today` — mark and text byte-equal to HEAD's.
      - `a_committed_fetch_reads_nothing_new_not_undoable`.
      - `an_old_aborted_fetch_line_is_not_refused` — `fetched_refs: None`; summary `remote-tracking refs`.
      - `a_pull_row_ends_with_its_moves`.
- [ ] 10. graft-ui `journal_screen::tests::an_expanded_fetch_row_lists_its_moves`:
      - the heading carries `u`;
      - with `app.undo` unbound, it is `refs the fetch moved` alone;
      - 25 moves → 20 lines + `+5 more`;
      - the collapsed row draws none.
- [ ] 11. Captures, each ≤ 200 KB:
      - `docs/shots/m8/21c-journal-partial-fetch-120x40.txt` and `…-80x24.txt`: J in the probe's state, collapsed,
        then the fetch row expanded.
      - `docs/shots/m8/21c-status-partial-fetch-120x40.txt`: the status line after `f`.

      Contact-sheet line: *"The `●` fetch row reads `failed: no repository at '…/gone…', or no access to it` with
      `origin/main moved before it stopped · not undoable`, and has no `refused · nothing written` and no `← u`.
      Expanded, `refs the fetch moved · u does not move them back` sits over `┆ origin/main  <7 hex> → <7 hex>`. The
      status line ends `· origin/main moved before it stopped`."* Colour unverified (rule 5); the user judges by eye
      (rule 6).
- [x] 12. `rg -n fetched_refs crates/graft-ops/src` output quoted in NOTES. No hit in `undo*`, `restore*`, `preview*`,
      `clash*`, or in `in_force`/`took_effect`/`changed_anything`/`next_*`.
      — **ticked by the grep quoted in NOTES** (no such hit) and by `journal::tests::next_undo_steps_past_a_push_and_a_fetch`
      staying green in the workspace run.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Fast tier only.

## Size
~75k, one context. Crate set: graft-core, graft-ops, graft, graft-ui — **no concurrent builder on these crates'
journal/apply/view-journal/journal_screen files**. If compacted, split at Part A | Part B (B needs A's three pub fns
and the field).

## NOTES — for the conductor
- Proposed row `M8/pull-incoming-read-unjournaled` (broken by design): a pull whose fetch succeeded but whose incoming
  read failed opens no entry (`apply/pull.rs:54–57`), so its tracking-ref moves are recorded nowhere. Not fixed here.

### Builder, Part A (2026-10-08) — **compacted once mid-ticket**; split at Part A | Part B per Size
- **Scout:** used the existing `.scratch/m8-polish-and-ship/scouts/21c-fetch-refs.md`; no new one.
- **Red (box 1),** written before any Part A code — compile errors:
  `error[E0432]: unresolved import graft_core::journal::RefMove`;
  `error[E0432]: unresolved import graft_ops::journal::wrote_anything`;
  `error[E0609]: no field fetched_refs on type Box<JournalEntry>`.
- **Deviations from Signatures (none change behaviour the spec names):**
  - `ref_store(repo, stop)` not `ref_store(ctx)`: the after-read must run with no stop (`&()`, as `record::read_after`),
    which `ctx` cannot express; the before-read passes `ctx.stop` (stop → `ApplyError::Stopped`).
  - Format is `%(objectname)%00%(refname)%00%(symref)`: **symbolic refs are skipped** (`origin/HEAD` would repeat
    `origin/main`'s move under a second name). Names are decoded lossily: display-only, and one non-UTF-8 branch must
    not refuse a fetch (comment in `apply/fetched.rs:47`).
  - New `FactsError::FetchedRefs(RunError)` for the listing's failure.
  - **`ApplyError::Remote` gained a public field `moves_unread: Option<Box<RunError>>`** (Scope 6 needs the error text in
    `explain`; `None` alone can't tell an old line from a failed read). Boxed: unboxed it tripped
    `clippy::result_large_err` on every `Result<_, ApplyError>`. Literals updated in graft
    (`update/tests/keymap_words/ops.rs`) and graft-ops tests.
  - `wrote_anything`, `moved_words`, `moved_lines` live in `journal/fetched.rs`, re-exported from `journal` (pub
    `graft_ops::journal::*` as specified); `wrote_anything` calls the private `took_effect` via `super::`.
  - `record::begin` hit `too_many_lines` (61/60) with the new field: its merge-state read moved to a helper
    `merge_before(ctx)` beside `config_before` (same behaviour, same error).
- **Where moves are set:** plain fetch — `fetch_first` lists before (`Ahead::Fetch(before)`), `run_rest` lists after
  and sets `entry.fetched_refs`. Pull — `fetch_first` lists around its fetch line and carries `moved` in
  `Ahead::{Fetched, Failed}`; `open_after` sets it right after the entry opens, so every close path carries it.
- **Box 12 grep** (`grep -rn fetched_refs crates/graft-ops/src`, test files left out):
  ```
  journal/lines.rs:106  pub fetched_refs: Option<BTreeMap<String, RefMove>>,
  journal/lines.rs:187  fetched_refs: None,                       (PendingLine::into_entry)
  journal/lines.rs:204  fetched_refs: entry.fetched_refs.clone(), (OutcomeLine::of)
  journal/lines.rs:218  entry.fetched_refs = self.fetched_refs;   (apply_to)
  journal/fetched.rs:5,29,32  doc + wrote_anything
  apply/error.rs:119    doc comment on moves_unread
  apply/pull.rs:127     pending.entry.fetched_refs = ahead.moved();
  apply/pull.rs:151     entry.fetched_refs = fetched::after(ctx.repo, &before, &mut run);
  apply/fetched.rs:4,141  doc + stopped_words (remote::explain's words)
  apply/fold.rs:178     fetched_refs: None,   (fold's record literal)
  apply/record.rs:133   fetched_refs: None,   (begin's literal)
  ```
  No hit in `undo*`, `restore*`, `preview*`, `clash*`, `in_force`, `took_effect`, `changed_anything`, `next_*`.
  Beyond the ticket's expected list: `fold.rs`/`record.rs` (literals, `None`), `journal/fetched.rs` (the submodule).
- **`JournalEntry {` grep lines:** base graft-ops 46, graft 45; now graft-ops 53 (+7, all tests: 4 literals, 3 fn
  return types), graft 45. `fetched_refs: None` added to every existing literal (graft-ops 4, graft 25 sites).
- **`mark_interrupted`** (`journal/file.rs:177`) clones the entry parsed from the pending line → `fetched_refs: None`
  (unknown); `recover.rs` has no literal and no mention.
- **Command Log:** both listings run through `GitInvocation` but are not added to `commands`; `remote_ops.rs:173` and
  `pull_set_aside.rs:89` green and unedited.
- **Status words — read, not observed:** graft `remote_failed` (`update/remote/in_flight.rs:166`) splits `explain` at
  its first ` · ` (`split_reason_and_keys` :232), so the appended moves land in the keys part, kept whole. Two paths
  will **not** show the moves (out of scope, say so to the user if asked): a diverged pull whose tracking tip is known
  (graft builds its own reason), and a cancelled fetch (graft's `cancelled` :88). Part B's status capture verifies the
  first claim.
- **Gates (workspace-wide, own `CARGO_TARGET_DIR`):** fmt ok; clippy ok; nextest 6107/6108 — the one failure is
  `graft::it published_repo::nothing_a_commit_would_carry_names_this_machine` on
  `.scratch/m8-polish-and-ship/issues/19c-tour-remainder.md:227`, **pre-existing at this worktree's base a198c95d**
  and fixed on main by 7e7fa9c3 ("fix: scratch path in 19c-c C+2"); not touched here. `cargo test --doc` ok.
- **Docs done:** graft-core MAP (`RefMove`), graft-ops MAP (`OutcomeLine.fetched_refs`, `wrote_anything`,
  `moved_words`/`moved_lines`, `FactsError::FetchedRefs`, apply's listing row, `ApplyError::Remote.moves_unread`),
  DESIGN `06-undo-journal-design.md` §6.1 (field) and §6.2 step 4.

### Handoff — Part B (boxes 9, 10, 11; graft + graft-ui; Part A's API is in place)
1. **Mark** — `crates/graft/src/view/journal.rs:192–202`, inside `in_force == false`: keep `reversed || !Aborted` →
   `Undone`; else `!graft_ops::journal::wrote_anything(entry)` → `RowMark::Refused`; else `RowMark::None` (the
   enum's default, `graft-ui/src/journal_screen/view.rs:11`). Variant and label unchanged.
2. **Summary** — `crates/graft/src/view/journal/summary.rs` fetch arm (pushes `remote-tracking refs`, `not undoable`,
   then `with_failure`). `<moves>` = `moved_words(map)` (+ ` before it stopped` when Aborted) if non-empty;
   `nothing new` if `Some({})` and Committed; `remote-tracking refs` if `None` or (`Some({})` and Aborted).
   **Gotcha:** `with_failure` returns only `failed: <first_line(reason)>` for Aborted and drops the parts — the moves
   clause must survive it (target row: `failed: no repository at '…' … · origin/main moved before it stopped · not
   undoable`). Pull row: today's summary + ` · <moved_words>` (+ ` before it stopped` when Aborted), non-empty only.
3. **graft-ui** — `JournalRowView.moved: Vec<SanitizedText>` (Default empty); `pub fn moved_heading(keys: &Keymap)
   -> String` spelling `u` via `keys.spell_keys(&[ActionId("app.undo")])`, unbound → `refs the fetch moved`.
   Draw in `journal_screen/pieces.rs` `under_pieces(row, selected, columns, width)` between the failure and
   `COMMANDS_HEADING` as `┆ ` lines cut by `cut_at_word` (pub in `journal_screen.rs`). **Gotcha:** `under_pieces` has
   no keymap — pass the heading (or keys) in. graft-ui does not depend on graft-ops: graft fills `moved` from
   `moved_lines(map, 20)` (≤ 21 lines incl. `+N more`), graft-ui draws what it gets; box 10's "25 → 20 + `+5 more`"
   is then a graft-ui test fed 21 lines plus a graft view test feeding 25 refs.
4. **Tests** — the five box-9 names and box 10; update existing tests asserting `remote-tracking refs` row text
   (`graft/src/view/journal/tests.rs`, `graft-ui/src/journal_screen/tests.rs`) and list each; check
   `graft/tests/it/m4_walk/{hangs_and_undo,coverage}.rs` for fetch/pull row text. `/` find will now match ref names
   in the summary — say so.
5. **Captures** (box 11) per the contact-sheet line; **docs** — graft and graft-ui MAP rows; DESIGN §1.10 mockup row
   → `fetch · origin/main moved`.

## Part B additions (conductor 2026-10-08)
- [ ] B+1: a **cancelled** fetch (graft's `cancelled`, see NOTES :307-308) also shows its moves in the status words, ending ` · origin/main moved before it stopped`. The user approved exactly this case on 2026-10-08 ("if you stop a fetch … partway … the line reads … moved before it stopped"), so it is in scope, not a new decision. Test seen red first; name it at tick. The diverged-pull path (graft builds its own reason) stays as NOTES says; Part B's status capture shows it.
