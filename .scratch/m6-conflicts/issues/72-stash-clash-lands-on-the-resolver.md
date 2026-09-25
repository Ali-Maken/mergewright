# stash-clash-lands-on-the-resolver: a pop or apply that clashes opens the resolver; done drops a popped stash by id

Status: ready
Blocked by: 67
From: pick 15 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24); review N3 (`docs/reviews/m6-usability-final.md`
:125-144).
Build with: (conductor, after the scout; planner suggests Opus — additive journal field, one pure rule with a table
test, app routing)
Scout: `.scratch/m6-conflicts/scouts/stash-clash-lands-on-the-resolver.md` — brief below.
Needs: none (67 merged first: its `refusal_status` is the one wording this ticket's `Unmerged` words go through).
Crates: graft-core (journal), graft-ops (journal lines, `apply`, `live_clash`, `DropPutBack`, `Refusal::Unmerged` words),
graft (operate, resolver close, stop line, relaunch, stash overlay, tests, walks).
Group: **E**. Before F: F's put-back clash reuses everything here.

## Read first
- `docs/design-notes/m6-stash-put-back.md` — all of it (two pages).
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §1–3, §6 ("Undo during a clash"), §7 last two bullets.
- `docs/research/commit-stash.md` addendum A1 (git's behaviour) and A4 (the questions this answers).
- `crates/graft-core/MAP.md` journal entries (`JournalEntry`, `is_interrupted`).
- `crates/graft-ops/MAP.md` :47-57 (stash kinds, refusals), :86-94 (journal lines, `merge_before`, `in_force`),
  :103-116 (inverse, `IndexState`), :161-179 (apply, resume, `recover_interrupted`).
- `crates/graft/MAP.md` :76-89 (`RefBadges::stashes`, `is_stash`), :120 (`AppState::stopped`), :218-229 (`operate`),
  :289-317 (`banner`, `stop_wordings`, `stop_line`), :321-335 (`resolver`, `OpenedFrom`), :436-449 (`relaunch`,
  `stash`), :674-677 (`resolver_context`, `done_line`).
- `docs/design/02-keymap.md` §2.6 `Enter`, §2.9 `S` overlay rows (keys are unchanged; words move).

## Decided (design note, ADR-0045 — not open here)
- A pop or apply the app runs writes `put_back: Some(PutBack { stash: <entry's commit>, why })` on its Pending line.
- A clash (index unmerged after the command, whatever git's exit code) answers `ApplyError::Clashed`, not `Git`. The
  entry is `Committed` with its unmerged `index_after`, so `u` undoes it.
- Landing: the `S` overlay closes, the changes pane opens, the resolver opens on the first clashed file,
  `OpenedFrom::Pane`. Notice: `stash@{0} clashed in 2 files · esc, u puts it all back` (`1 file` singular).
  `Esc` → the changes pane on `CONFLICTS`.
- Last file done: pop → `Operation::DropPutBack { stash }` runs; status
  `every file done · stash@{0} dropped ✓ · u brings it back`. Apply → nothing runs; `every file done · stash@{0} kept ✓`.
  The resolver closes to the changes pane (not the graph).
- While a clash is live and the resolver is shut: stop line `stash@{0} clashed · 1 file conflicted`, shorter wordings
  drop the count, then the name (`stop_wordings` ladder). Relaunch: `stash@{0} clash from last run · 1 file conflicted`.
- A pop made in the shell: no record → today's behaviour, no name, no drop.
- `Refusal::Unmerged` words (`graft-ops/src/operation.rs:550-553`, review N3): `<path> still has conflicts · resolve it
  first` (`+N` when more), through 67's `refusal_status`. "In the shell" and "a later milestone" go. The stop line
  already names where the conflicts are; the refusal does not guess a key for a screen it cannot see.

## Scout brief (≤1 page, code not intent)
1. `stash pop`/`apply` in `graft-ops` (`operation.rs:~994`): where a non-zero exit becomes `ApplyError::Git`; what the
   executor reads after the command (`RepoStateNow`? `index_after`?) and whether it reads the index on failure.
2. `begin` and every constructor of `PendingLine` and `JournalEntry` (grep, list files:lines) — each gains a field.
3. `Refusal::Unmerged`'s words (`operation.rs:550-553`) and every surface that shows them (grep the text).
4. `Operated` and its `Conflicted` arm in `update/operate.rs`: what sets `AppState::stopped`, `live_entries`, and opens
   the resolver (P5). Can a `Stop` exist with no `InProgress` (is `Stop::kind` an `InProgress`)?
5. The resolver's close: `close_after_last`, `finish_owed`/`finish_if_owed`, `resolved.rs:295-305` (the stash-pop special
   case the review cites) and where it lands (graph vs pane).
6. `side_names` for a stash (`yours`/`stashed`, ADR-0043): where the stash's `stash@{n}` comes from.
7. Where the app reads the journal (`jobs::undo::JournalRead`) and `RefBadges::stashes` — can `live_clash` be answered
   from state the app already holds after each write and at launch?
8. Existing tests and walks that pop or apply into a conflict and assert `Git`/the raw error (grep `Index was not
   unstashed`, `Unmerged`).
9. Ticket 66's `is_stash` filter (`update/selection.rs`) and the radar's stash row: does a clash's unmerged tree change
   what they draw?

## Scope
**In:** the journal field; `Clashed`; `live_clash`; `DropPutBack` (op, check, apply, inverse via the existing
`StashDrop` rule); app landing, notice, done words, drop on done, stop line, relaunch; the `Unmerged` words; tests,
walks, captures.
**Out:** set-aside (F); unstaging after a resolved clash (ADR-0045 Consequences, KNOWN_GAPS); a confirm box on the
drop (rejected, design note Prior art); drawing stash internals (KNOWN_GAPS :208).

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
pub struct Clash { pub stash: Oid, pub why: PutBackWhy, pub paths: Vec<PathBuf> }
// ApplyError gains (not retryable, like Conflicted):
Clashed { entry: Box<JournalEntry>, clash: Clash },
pub struct StashClash { pub entry: EntryId, pub stash: Oid, pub index: Option<usize>, pub why: PutBackWhy }
/// ADR-0045 §2. Pure. `stashes` is `RefBadges::stashes` (commit, index), newest first.
pub fn live_clash(entries: &[JournalEntry], in_progress: Option<InProgress>, any_unmerged: bool,
                  stashes: &[(Oid, usize)]) -> Option<StashClash>;
// Operation gains:
DropPutBack { stash: Oid },   // kind StashDrop; pins [stash]; index looked up at apply; Refusal::NoSuchStash if gone
// App-side shapes (the clash in `Operated`, the resolver's clash field, the stop kind): named by the scout, in NOTES.
```

## The failing tests first
- graft-core: `a_journal_line_without_put_back_re_renders_byte_identically` (a line from before, fixture text);
  `put_back_round_trips` (each `PutBackWhy`).
- graft-ops, real repo:
  - `a_pop_that_clashes_answers_clashed_and_keeps_the_stash` — entry Committed, `index_after` unmerged, `put_back` =
    `Pop(<commit>)`, stash list unchanged.
  - `an_apply_that_clashes_records_apply`.
  - `a_failed_apply_with_nothing_unmerged_is_not_a_clash` — an untracked file in the way: `Git`, not `Clashed`
    (ADR-0045 §7, the outcome comes from the index).
  - `drop_put_back_drops_by_commit_id_not_position` — a second stash pushed on top; the clashed one (now `stash@{1}`)
    goes, `stash@{0}` stays.
  - `drop_put_back_refuses_when_the_stash_is_gone`; `undo_of_drop_put_back_stores_it_again`.
  - `undo_of_a_clashed_pop_restores_the_tree_and_keeps_the_stash`.
- graft-ops, pure: `live_clash_table` — rows: nothing unmerged → None; in progress → None; pop clash → Some; after a
  stage/take step (`index_before` unmerged) → Some; an undo back into the clash → Some; shell pop, no record → None;
  stash gone → None; the residual shell-after-apply row (ADR §2) → Some(Apply).
- graft `update`: `a_clashing_pop_lands_on_the_resolver_on_the_first_clashed_file`;
  `the_last_file_done_after_a_pop_clash_drops_the_stash`; `after_an_apply_clash_the_stash_is_kept`;
  `esc_after_a_clash_lands_on_conflicts_in_the_changes_pane`; `relaunch_with_a_clash_open_names_the_stash`;
  `a_shell_pop_clash_names_no_stash_and_drops_nothing`; `the_unmerged_refusal_names_no_shell_and_no_milestone`.

## Touches (each entry, and how the builder checks it)
- **Journal constructors** (scout Q2): every one passes `put_back`. Check: build; `grep -rn "PendingLine {" crates`.
- **`ApplyError` matches**: every `match` gains `Clashed`. Check: build (no `_ =>` arm added to hide it; grep `_ =>`
  near `ApplyError` in the diff).
- **Undo/redo** (m6-conflicts/07 F2 tests, `InversePlan` pins): `DropPutBack` reuses `StashDrop`'s inverse. Check: the
  undo tests above plus the existing stash undo suite green.
- **Journal browser `J`**: a `DropPutBack` row reads as a drop of `stash@{n}`. Check: a `journal_view` test.
- **Stash overlay `S`**: closes on a clash; its list re-reads after the drop. Check: the landing test asserts
  `stash.shown == false`; the drop test asserts the re-read command.
- **Resolver close** (`resolved.rs:295-305`) and **`side_names`** (`yours · main` | `stashed · stash@{0}`): Check: the
  done tests; `every_key_the_bar_draws_is_answered` green.
- **Stop line** (`stop_wordings`, `stop_line`) gains the clash kind. Check: a wordings-ladder test at 80 and 120 cols.
- **`Refusal::Unmerged` surfaces** (scout Q3): every write refused on unmerged paths gets the new words. Check: grep of
  the old text returns nothing in `crates/`.
- **Radar** (ticket 66, `update/selection.rs` `is_stash`, `probes.rs`): no change expected. Check: 66's walk
  `radar_bookkeeping::a_pull_rebase_marks_no_duplicates` green; scout Q9 answer in NOTES.
- **Hint bars**: resolver bar on a clash `o yours  t stashed` (existing roles). Check: bar test at 80 and 120.
- **Captures**: `grep -rln "every file done\|later milestone\|Index was not unstashed" docs/shots` — each regenerated
  by its test or `git diff --exit-code`, listed in NOTES.
- **TESTING.md**: rows with `every file done` for a stash pop, or `later milestone`: listed in NOTES for H.
- **Keymap doc** §2.6 `Enter`, §2.9 `p`/`⏎`: rows owed by the conductor (design note "Where I'd differ"). The builder
  does not edit `docs/design/`.
- **MAP.md**: graft-core `PutBack`; graft-ops `Clashed`, `live_clash`, `DropPutBack`, `Unmerged` words; graft landing,
  drop-on-done, stop kind.

## Acceptance (name the verifying test at tick time)
- [ ] Every test above, red then green (graft-core round-trip tests may be green-first; say so).
- [ ] Walk, real repo (`tests/it/m6_stash_put_back.rs`): `a_stash_pop_that_clashes_is_four_keys` — from the graph,
      `S p` → resolver on the clashed file; `o ⏎` → done. Asserts: keys ≤ **4**; file has `yours`; `git stash list`
      empty; status `every file done · stash@{0} dropped ✓ · u brings it back`. Then `u` → `git stash list` has it again.
- [ ] Walk: `a_stash_apply_that_clashes_is_four_keys` — `S ⏎ o ⏎`, ≤ **4**, stash kept.
- [ ] Walk: `esc_u_puts_a_clashed_pop_back` — `S p esc u` (plus `y` if the drift box asks): tree as before the pop,
      no unmerged paths, stash listed. Keys after the clash ≤ **2** (+1 `y`).
- [ ] Captures (rule 6; colour **unverified**, rule 5), written by the walks:
  - `docs/shots/m6/stash-clash-landing-120x40.txt`, `-80x24.txt` — **What right looks like:** no `S` overlay; the
    resolver on `notes.md`, panes named `yours · main` and `stashed · stash@{0}`; the notice row reads
    `stash@{0} clashed in 1 file · esc, u puts it all back`; the bar has `o yours` and `t stashed`; no git error text.
  - `docs/shots/m6/stash-clash-done-120x40.txt` — the changes pane, no `CONFLICTS` section; bottom row
    `every file done · stash@{0} dropped ✓ · u brings it back`.
- [ ] NOTES: scout answers used, app-side shape names, captures and TESTING.md rows touched.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Large for one context (~70k): a field, an error arm, a pure rule, one op, app routing, three walks. If the scout finds
the resolver close needs more than `resolved.rs`, split the app half out (`handoff`) and say so.
