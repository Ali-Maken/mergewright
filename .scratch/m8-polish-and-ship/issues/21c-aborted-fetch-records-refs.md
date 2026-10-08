# 21c — a fetch records the refs it moved; J stops saying "nothing written" when one moved

Status: draft (planner spec-only pass, 2026-10-08)
Build with: **Opus (xhigh)** — `builder-opus-xhigh`, the undo area: it edits the journal line format and adds a
predicate beside `in_force`, which drives `u`. **Not Fable**: the approach is decided here (ADR-0062), and correctness
is judged by tests (the probe, the predicate table, `next_undo`/`revert_to` unchanged) — test 3 fails. Brownfield:
**scout first** (below); the conductor may revise this line after it.
Needs: none (no other ticket's signature). Decision: `docs/adr/0062-a-fetch-records-the-refs-it-moved-outside-the-undo-record.md`
(Proposed — the conductor accepts it before filing; if refused, this spec goes back to the planner).
Row closed on merge: **M8/aborted-refs-moved** (and proposed M8/aborted-fetch-not-in-force, same defect).

## The defect (21b-3 NOTES item 7, observed)
`LocalRemote` (Linear) + `commit_on_remote`, second remote `gone` = `file:///nonexistent/…`,
`apply(Fetch { remote: None, prune: false })` → `Err`; `origin/main` **moved** to the teammate's tip. Entry: `kind: Fetch`,
`status: Aborted { reason: "no repository at '/nonexistent/graft-21b-3.git', or no access to it" }`, `touches.refs: []`,
`refs_before: {}`, `refs_after: {}`, `worktree_before: None`, `commands: ["git fetch --progress --all"]`. `took_effect`
(graft-ops `journal.rs:165`) is false → J marks it `nothing written` (graft `view/journal.rs:192–202`). A `Committed`
fetch records no refs either. `u` unaffected (`undoable_kind` skips `Fetch`).

## Decisions (ADR-0062; summary)
1. **Record:** one entry; a `for-each-ref` of `refs/` (minus `refs/graft/`) before `git fetch` and again after it exits
   (success, failure, stop); the differing refs go on the **outcome line** as `fetched_refs`. Fetch, and a pull's
   fetch line. Not per-remote entries.
2. **`u`: no.** `fetched_refs` is outside the undo record. `in_force`, `took_effect`, `touches`, `refs_before`,
   `refs_after` unchanged for fetch and pull. An Aborted fetch stays **not in force** — deliberately (an in-force
   Aborted *pull* would become `next_undo` with an empty inverse).
3. **J's mark** reads a new predicate, `wrote_anything`, not `in_force`.
4. **Words:**
   - status (failure or stop of a fetch/pull that moved refs): today's words + ` · origin/main moved before it stopped`
     (`origin/main moved, origin/feat new +3 more before it stopped`). Success status (`fetched`) unchanged.
   - J `●` row summary: Committed fetch → `origin/main moved` … or `nothing new` (moved nothing); Aborted fetch with
     moves → `failed: <git's first line> · origin/main moved before it stopped`, **no** `nothing written`, no mark;
     Aborted fetch that moved nothing → as today (`nothing written`); pull rows → today's summary + ` · <moved>`
     (` before it stopped` when Aborted).
   - J expanded row: after the failure, before `commands Graft ran`: heading `refs the fetch moved · u does not move
     them back` (`u` spelled from the keymap, `app.undo`; unbound → `refs the fetch moved`), then `┆ origin/main
     a1b2c3d → e4f5a6b`, `┆ origin/feat  new at e4f5a6b`, `┆ origin/old  gone, was a1b2c3d`, `┆ tag v1.2  new at …`;
     at most 20 lines, then `┆ +N more`.
   - Why not "fetched origin; gone failed": a remote that brought nothing reads the same as one that failed; naming
     the failed one means parsing git's per-remote stderr. The failure words already name `gone`'s location (20b R3).

**Journey** (no new screen; J's existing expanded-row pattern — no prior-art study exists for J, and this adds no
interaction): `f` → status line says it failed **and** what moved (0 more keys) → `J` (1) row says the same →
`space` (2) lists every ref. `u` from anywhere still takes back the last local change (ADR-0037 §3).

## Scout first (ADR-0035, ≤ 1 page, report code with file:line)
Where fetch's entry is opened/closed (`apply`, `record::begin`/`finish`, `apply/remote.rs`); pull's `fetch_first`
(`apply/pull.rs`) and where its entry opens; whether `begin`'s before-reads go on the entry's `commands` (decides
point 3 of Scope); `took_effect` (`journal.rs:165`) and any `changed_anything` (rg) and their callers; every
`JournalEntry { … }` and `OutcomeLine { … }` literal (`rg -c` per crate); `view/journal.rs:192–202` and the summary
code (`view/journal/summary.rs`); which tests assert a fetch's `commands` or a fetch/pull J row's text; the harness
behind `docs/shots/m4-07-journal-remote-rows-*`.

## Read first
- `docs/adr/0062-…` (whole, 1 page); ADR-0037 §3; `docs/design/06-undo-journal-design.md` §6.1, §6.2, §6.4.
- `crates/graft-ops/MAP.md` `## journal` (:106–124), `## undo` `inverse`/`plan_revert_to` (:135–137), `## apply`
  `apply` (:266), a pull's fetch first (:242), `ApplyError` `Remote` (:251), `Context` (:237).
- `crates/graft-core/MAP.md` `## journal` (:50–62).
- `crates/graft/MAP.md` `journal_view` (:947), `remote_done`/`remote_failed` (:621).
- `crates/graft-ui/MAP.md` `## journal_screen` (:252–268).
- `.scratch/m8-polish-and-ship/issues/20b-words-rulings.md` R8 (:37) — the `nothing written` mark.
- The scout's file. Source after MAP.

## Scope
**In**
- **Part A — record (graft-core, graft-ops).**
  1. `RefMove` + `JournalEntry.fetched_refs` (graft-core); `OutcomeLine.fetched_refs` (graft-ops), serde
     `default` + `skip_serializing_if = Option::is_none`: every other line renders byte-identically.
  2. `apply/fetched.rs` (new, crate-private): `ref_store` (one `for-each-ref --format=%(objectname) %(refname) refs/`,
     `refs/graft/` dropped) and `moved` (pure diff).
  3. Fetch: `ref_store` before `begin` (fails → `ApplyError::Facts`, no entry, nothing ran); after git exits, again;
     `fetched_refs = Some(moved(..))` on the outcome line, Committed **and** Aborted/cancelled. After-read failing:
     `fetched_refs: None`; on success that is `AfterState::Refs` as today's unread refs; on a remote failure the
     `Remote` error stands and its words gain ` · which refs it moved could not be read: <error>`. The two reads go
     on the Command Log **exactly as `begin`'s own before-reads do** (scout says which; NOTES quotes it).
  4. Pull: the same two reads around `fetch_first`'s git line only (before it; right after it exits), onto the pull
     entry's outcome line, so the merge's branch move is not in `fetched_refs`.
  5. `journal::wrote_anything`, `journal::moved_words`, `journal::moved_lines` (pub). `ApplyError::Remote`'s
     `explain` appends ` · <moved_words> before it stopped` when the entry's `fetched_refs` is non-empty.
  6. `recover_interrupted` / `mark_interrupted`: write `fetched_refs: None` (unchanged behaviour; check the literal).
- **Part B — show (graft, graft-ui).**
  7. `view/journal.rs`: `RowMark::NothingWritten` iff `Aborted` ∧ `!wrote_anything` ∧ not reversed (the
     `took_effect`/`in_force` term swapped, nothing else). Summary words as Decision 4.
  8. graft-ui `JournalRowView.moved`, `moved_heading`; drawn only while expanded, between `failure` and
     `COMMANDS_HEADING`, `┆ `-led like the commands, cut by `cut_at_word` at the pane width.
  9. MAP.md: graft-core, graft-ops, graft, graft-ui rows. DESIGN §6.1 (`fetched_refs` line in the struct),
     §6.2 step 4 (`Nothing needs undoing; a fetch's moved refs are recorded, ADR-0062`), §1.10 mockup row →
     `fetch · origin/main moved` — **only after the conductor marks ADR-0062 Accepted**.

**Out** — `u`/`U`/`⏎` on a fetch row (unchanged: nothing); the success status `fetched`; push (one ref, named by its
description; a rejected push moves nothing); commit counts; the pull whose incoming read fails after its fetch (no
entry — proposed row `M8/pull-incoming-read-unjournaled`, **broken by design**, MAP graft-ops :242); regenerating
earlier milestones' captures (records, not live).

## Signatures
```rust
// graft-core — src/journal (beside PutBack/MergeState), re-exported at graft_core::journal
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RefMove { pub before: Option<Oid>, pub after: Option<Oid> }   // None = absent (new / gone)
// JournalEntry, new field:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub fetched_refs: Option<BTreeMap<String, RefMove>>,  // Some(empty) = read, nothing moved; None = not recorded

// graft-ops — src/journal.rs (pub)
pub fn wrote_anything(entry: &JournalEntry) -> bool;
//   took_effect(entry)
//   || entry.fetched_refs.as_ref().is_some_and(|m| !m.is_empty())
//   || (entry.fetched_refs.is_none() && matches!(entry.kind, Fetch | Pull)
//       && matches!(entry.status, EntryStatus::Aborted { .. }))      // unknown ≠ unchanged
// If `changed_anything` already means this, extend it under its own name instead; NOTES says which.
pub fn moved_words(moved: &BTreeMap<String, RefMove>) -> Option<String>;     // None when empty
//   "origin/main moved" · "origin/main moved, origin/feat new" · "…, … +3 more" · "tag v1.2 new" · "origin/old gone"
//   order: the map's (full ref name); shorthand via RefName::shorthand; refs/tags/* → "tag <short>"
pub fn moved_lines(moved: &BTreeMap<String, RefMove>, at_most: usize) -> Vec<String>;
//   "origin/main  a1b2c3d → e4f5a6b" · "origin/feat  new at e4f5a6b" · "origin/old  gone, was a1b2c3d"; then "+N more"
// OutcomeLine, new field — same type and serde attributes as JournalEntry.fetched_refs.

// graft-ops — src/apply/fetched.rs (new, crate-private); lifetimes/command-log parameter as begin's reads take them
pub(crate) fn ref_store(ctx: &Context) -> Result<BTreeMap<String, Oid>, RunError>;
pub(crate) fn moved(before: &BTreeMap<String, Oid>, after: &BTreeMap<String, Oid>) -> BTreeMap<String, RefMove>;

// graft-ui — journal_screen
pub struct JournalRowView { /* … */ pub moved: Vec<SanitizedText> }      // empty → nothing drawn
pub fn moved_heading(keys: &Keymap) -> String;  // "refs the fetch moved · u does not move them back"; u unbound → first clause
```

## Touches (check each; the builder's **Touches:** line says how)
- **Undo** — `inverse`, `drift`, `state_at`, `PointInTime`, `in_force`, `in_force_all`, `next_undo`, `next_redo`,
  `plan_revert_to`, `clash_candidate`, `stage_source`: must not read `fetched_refs` — `rg -n fetched_refs
  crates/graft-ops/src` lists only `journal.rs`, `apply/fetched.rs`, the fetch/pull close, the words; tests in boxes 6–7.
- **Journal file** — old lines parse and re-render byte-identically (box 4); `render_outcome`; recovery literals.
- **Every `JournalEntry {…}` / `OutcomeLine {…}` literal** across the workspace (graft tests included) gains the
  field; count before/after quoted.
- **J** — mark rule, summary, expanded row, `enter_words` (unchanged: none on a fetch row), the cursor-preselect rule
  (keymap §2.11 "except a fetch": unchanged), `/` find (now matches ref names in the summary — say so in NOTES).
- **Status** — `ApplyError::Remote::explain`; graft `remote_failed` shows it (no graft change expected; check).
- **Command Log** — two more lines per fetch/pull if `begin`'s reads are logged; every test asserting a fetch's or
  pull's `commands` (e.g. `tests/it/remote_ops.rs`) updated and listed.
- **Captures** — new `docs/shots/m8/21c-*`; `m4-07-journal-remote-rows-*` and `m7/journal-*` are records, not
  regenerated; any live scenario test asserting a fetch/pull J row's text updated and listed.
- **Keymap** — none bound; `moved_heading` spells `u` from `app.undo` (W1/W3).
- **Other crates** — graft-git, graft-graph, graft-fixtures: none (the read goes through graft-ops' existing run path).
- **Docs** — four MAP.md files; DESIGN §6.1/§6.2/§1.10 after acceptance; KNOWN_GAPS is the conductor's.

## Acceptance (each tick names its test, at tick time)
- [ ] 1. **Red first** (graft-ops `tests/it/aborted_fetch.rs`, the probe re-created from the defect section):
      `a_fetch_that_moved_refs_before_one_remote_failed_records_them` — `origin/main` moved; entry `Aborted`;
      `fetched_refs == Some({"refs/remotes/origin/main": RefMove { before: old, after: teammate tip }})`;
      `wrote_anything` true; **`in_force` false**. NOTES quotes its failure before Part A's code.
- [ ] 2. `a_fetch_that_brought_nothing_records_an_empty_map` — Committed, `Some({})`, J summary `nothing new`
      (graft test in box 9 covers the words).
- [ ] 3. `a_fetch_records_new_moved_pruned_and_tag_refs` — one fetch with `prune: true`: a moved, a new, a pruned
      (`after: None`) branch and an auto-followed tag, each exactly once; no `refs/graft/` key.
- [ ] 4. `journal::tests::a_line_without_fetched_refs_re_renders_byte_identically`;
      `journal::tests::fetched_refs_round_trip` (`Some({})` renders `"fetched_refs":{}`; a map round-trips).
- [ ] 5. `journal::tests::wrote_anything_table` — rows: Aborted fetch `Some({})` → false; `Some(1 ref)` → true;
      Aborted fetch `None` → true; Committed fetch `Some({})` → true (took effect); Aborted commit, no backup → false
      (today's rule untouched); Aborted pull `Some(1 ref)` → true **and** `in_force` false.
- [ ] 6. `u_after_a_partial_fetch_takes_back_the_change_before_it` — commit, partial fetch, `next_undo` = the
      commit's entry; `plan_undo` steps hold no `refs/remotes/` ref.
- [ ] 7. `a_diverged_pull_records_its_fetch_but_u_is_unchanged` — pull Aborted (diverged): `fetched_refs` holds
      `origin/main` only (not the local branch); `next_undo` same as before the pull; `plan_revert_to` across it
      unchanged (compare plan before/after).
- [ ] 8. `remote::tests::a_partial_fetch_says_what_moved` (or where `explain` is tested) — the probe's `explain`
      ends ` · origin/main moved before it stopped`; `journal::tests::moved_words_table` (0 → None, 1, 2, 3+ with
      `+N more`, new, gone, tag); `moved_lines` caps at `at_most` then `+N more`.
- [ ] 9. graft `view::journal::tests`: `a_partial_fetch_row_names_what_moved_and_not_nothing_written`,
      `a_fetch_that_moved_nothing_still_reads_nothing_written`, `a_committed_fetch_reads_nothing_new`,
      `an_old_aborted_fetch_line_is_not_marked_nothing_written`, `a_pull_row_ends_with_its_moves`.
- [ ] 10. graft-ui `journal_screen::tests::an_expanded_fetch_row_lists_its_moves` — heading with `u`; `app.undo`
      unbound → `refs the fetch moved` alone; 25 moves → 20 lines + `+5 more`; collapsed row draws none.
- [ ] 11. Capture `docs/shots/m8/21c-journal-partial-fetch-120x40.txt` and `…-80x24.txt` (J, the probe's state:
      collapsed, then the fetch row expanded) and `docs/shots/m8/21c-status-partial-fetch-120x40.txt` (the status
      line after `f`). Each ≤ 200 KB. Contact-sheet line: *"The `●` fetch row reads `failed: no repository at
      '…/gone…', or no access to it · origin/main moved before it stopped`, with no `nothing written` and no `← u`;
      expanded, `refs the fetch moved · u does not move them back` over `┆ origin/main  <7 hex> → <7 hex>`. The status
      line ends `· origin/main moved before it stopped`."* Colour unverified (rule 5); the user judges by eye (rule 6).
- [ ] 12. `rg -n fetched_refs crates/graft-ops/src` output quoted in NOTES: no hit in `undo*`, `restore*`, `preview*`,
      `clash*`, or `in_force`/`took_effect`/`next_*`.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Fast tier only.

## Size
~75k, one context. Crate set: graft-core, graft-ops, graft, graft-ui — **no concurrent builder on these crates'
journal/apply/view-journal/journal_screen files**. If compacted, split at Part A | Part B (B needs A's three pub fns
and the field).

## NOTES — for the conductor
