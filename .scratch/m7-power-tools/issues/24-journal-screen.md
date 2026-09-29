# journal-screen — `J` shows the shell's steps and the lost commits as rows; `⏎` puts one back; he lands on it

Status: ready — respecified 2026-09-27 against `scouts/24.md` (main @ b90a49e4); **amended 2026-09-29** after 24-a's
verify-first (commit 80b72040): V3, V4, V5 decided below; a part **24-0** (the drop's why) and a part **24-a2** (the
put-back box) added. Scope unchanged.
Build with:
- **24-0: Sonnet.** Mechanical: one field added and passed through, one words arm, red test first. Every site is listed.
- **24-a, 24-a2, 24-b: Opus.** A screen over a decided list and a decided operation; every choice is fixed here and
  judged by a named test, a walk or a capture. The one structural piece (the landing target) is a small widening of
  03's mechanism, pinned by 03's own tests → not Fable.
Scout: done (`.scratch/m7-power-tools/scouts/24.md`). No second scout. ~15 merges since it: **line numbers in this
ticket are only those 24-a's NOTES verified** (marked `(NOTES)`); everything else is named by fn, test or MAP entry —
find it by grep.

## Crates and files (ADR-0051; outside a part's list → stop, `blocked: needs <file>`)
**24-0** (drop why):
- graft-ops: `src/operation.rs` (the `DropPutBack` variant gains `why`; `describe_in`'s and `describe`'s arms; every
  other `DropPutBack` pattern in the file), `src/operation/tests.rs`, `src/apply/record.rs` (its `DropPutBack` pattern
  only), `tests/it/stash_clash.rs`, `MAP.md` (the `Operation::DropPutBack` and `describe_in` entries).
- graft: `src/update/resolver/resolved.rs` (the one constructor, :170 (NOTES), passes `clash.why`),
  `src/update/clash.rs`, `src/update/state.rs` (its `DropPutBack` mention only), `src/update/operate/tests/edits_words.rs`
  (:14 (NOTES)), `tests/it/m6_stash_put_back.rs` (only if it asserts a set-aside drop's words), `MAP.md`
  (`AppState::clash_drop`, if it spells the variant).
- Read only (V6): graft-ops `src/apply/set_aside/returned.rs`, `src/operation/stash.rs`; graft `src/view/journal.rs`,
  `src/update/stash/tests.rs`.
- That set is every file naming `DropPutBack` or `drop stash@{` at 802a98a2 (grep, counts only). No `docs/shots/`
  file quotes `drop stash@{` (grep) → no capture changes.

**24-a:**
- graft: `update/journal.rs` (+ tests), `update/recover.rs` (new: `done_status`, `put_back_tags`), `jobs/journal_list.rs`
  (new), `jobs/runner.rs` (dispatch arms only), `jobs/mod.rs` (the `Command`/`Product` variants), `update/mod.rs` (the
  new `Product` arms only), `update/operate.rs` (`Landing`, `landing_of`, the `Done` arm), `update/banner/stop.rs`
  (`finished_lands_on`, `HeadLanding.on`), `update/state.rs` (`lands`), `update/name_prompt.rs`
  (`open_name_prompt_with`; the create arm closes J), `view/entries.rs` (tag read only), `tests/it/` (Part A tests),
  `MAP.md`.
- graft-ui: `graph_screen/mod.rs` (`RowTag` variants and their draw).
- graft-ops: `journal_list.rs` `words` (the `commit (initial)` arm) + its test. **Not** `operation.rs` (moved to 24-0).

**24-a2** (the put-back box, V3):
- graft: `update/recover.rs` (`preview_title`, `preview_lines`, `files_words` + tests), `update/preview.rs`
  (`preview_box`'s `Recover` arm; `confirm_preview` closes J for a `Recover` subject), `view/frame/overlays.rs` (the box
  docked over J, as `draw_stash` docks it over S), `tests/it/` (the box tests), `MAP.md`.
- Only if `keys_go_to_the_put_back_box_over_j` is red on 24-a's code: the one site that routes a key to J before the
  preview. Name it in NOTES. It is `update/mod.rs` or `update/journal.rs` → allowed; anywhere else → stop, blocked.
- graft-ui: **none.** `preview_box.rs` is used through its existing API (`lines`, `with_choices`, `again`).
- Docs: `docs/shots/m7/put-back-preview-187x50.txt`.

**24-b:**
- graft: `view/journal.rs`, `tests/it/` (walks, hint bar, snapshots), the m7 walk harness.
- graft-ui: `journal_screen.rs` (+ tests).
- Docs: `docs/shots/m7/`, the m3/m4/m5 J captures it re-shoots, the contact-sheet captions, `TESTING.md` rows, MAP.md
  (graft, graft-ui).

## Blocked by
- **22** journal-list, **23** recover-op, **03 part -a** (landing), **09** (`RowTags`) — merged.
- **rebase-preview-keeps-drops** (building now): takes `update/preview.rs` (an `R` arm), `jobs/runner.rs` (the
  OpenRewrite path), graft-ui `preview_box.rs` (footer `R keep` after the `again` slot), a new m7 walk. 24-a
  (`runner.rs`), 24-a2 (`preview.rs`) and 24-b (the m7 walk harness) wait for it. 24-a2's footer uses the `again`
  slot only; `R keep` is never set on a `Recover` box.
- **10** branch-picker: may generalise `return_to_journal` in `update/preview.rs` and holds the modal predicate.
- **30j**: holds `jobs/runner.rs` (the exhaustive `Command` match) and `update/mod.rs`. 24-a adds `Command` variants
  on its exhaustive match. Not concurrent with **21c** (also `jobs/runner.rs`).
- **03b**: holds graft-ops `operation.rs` (→ 24-0) and graft's pull `Done` words in `update/operate.rs` (→ 24-a).
- **09b**, if it took `operation.rs` or `update/preview.rs`. Conductor checks its NOTES.
- **The `lost_work` fixture order** (Ruling below; graft-fixtures + graft-ops tests). 24-b's walks only. Not landed →
  24-b reports `blocked: needs graft-fixtures (lost_work order)` at the walks and stops.
- **Not concurrent with 08** rewrite-editor: both take the m7 walk harness, the `journeys` parser, the m7 captions TSV,
  `docs/shots/m7/` and the key route.
- **Order:** 24-0 → 24-a (both touch graft `update/state.rs`) → 24-a2 → 24-b.

Source: `docs/design-notes/m7-power-tools.md`: pass 1b "Journal"; "Pass 2b" F1, F4, F5, J1, "Where he lands" (2b),
keymap diff (2b) §2.11, "Journey counts (2b)", "Where I'd differ (2b)" 9–10; field notes :72, :78; ADR-0044 §1–2,
ADR-0046; review `docs/reviews/m7-22-journal-recover.md` M4.

Needs (as merged; MAP entries by name):
- graft-ops `journal_list::{read_outside, find_lost, build, words, recover_for, recover_scope, lost_edits,
  JournalList, ListRow, OutsideRecord, OutsideStep, LostCommit, LostEdit, HeldBy, LostScan, Recover}`.
- `Operation::Recover(RecoverParams)`, `RecoverTarget::{MoveRef, Recreate, Files}`, `FilesEffect`,
  `PreviewPlan.files` (graft-ops MAP `operation`, `PreviewPlan`).
- graft-git `ReflogRecord { refname: String, old: Option<Oid>, new: Option<Oid>, at: i64, subject: String }`
  (`graft-git/src/reflog.rs:22-32` (NOTES)).
- graft `update::banner::finished_lands_on_head`, `AppState::{lands_on_head, land_on_head, head_landing}` — **there is
  no `land_on_result`**; 24-a widens these.
- graft `update::row_tags::RowTags::{set, on, clear}`, graft-ui `graph_screen::RowTag`.
- graft `update::switch::{switch_to, SwitchTarget}`, `update::preview::{open_preview, preview_box, confirm_preview,
  close_preview}`, `PreviewSubject::Operation`, `update::name_prompt::{open_name_prompt, NameKind}`.
- graft-ui `preview_box::PreviewBox::{new, with_choices}`, fields `lines`, `confirm_key`, `again`.

## Background (from KNOWN_GAPS; builders don't read it)
Rows this ticket closes:
- **M3 :217:** no status line while `J` is open, so a refusal inside `J` has nowhere to show.
- **M6/68:** an entry aborted by the user reads `failed: aborted`. It should read `aborted by you · nothing changed`.
- **M6 usability (:848):** a row cut to fit runs words together (`the woundone`). Cut at a word boundary, with `…`.
- **M6 usability (:847; `docs/reviews/m6-usability-2.md:181`):** the set-aside drop has two names. The done line says
  `set-aside copy dropped`; the journal's undo row says `undo: drop stash@{0}`. One name: the row reads
  `drop the set-aside copy (stash@{0})`. The done line already says `set-aside copy` and stays.
New gap row (conductor files it): **the put-back box has no `N commits back on main` clause** (J1 mock-up). No field
carries the count (`PreviewPlan` has `ref_moves` and `files`, 23), and `update` reads nothing. `now` / `after y` still
show both tips.

## What changed from the first spec (scout 24; history, not instructions)
| First spec said | Code said (scout, b90a49e4) | Now |
|---|---|---|
| `land_on_result(state, kind, new_head)` | Does not exist. 03-a gave `Operation::moves_head`, `AppState::lands_on_head`, `finished_lands_on_head` (seeks `refs.head()` only); 03 excludes `Recover`. | Widen to a **target**: `Landing::{Head, Commit(Oid), Changes(PathBuf)}` by `landing_of(op)`. `finished_lands_on_head` stays as a wrapper. |
| `⏎` on `●` = revert-to | `⏎` on the `← u` row runs `u`, on `← U` runs `U`, no preview; older rows open `RevertTo` | **Stays.** Bar per row: `⏎ undo this` / `⏎ redo this` / `⏎ revert to before this`. |
| :847 fixed in graft | Words made in graft-ops `describe_in`; `DropPutBack` carries no why (V4) | **24-0** adds `why: PutBackWhy`; `describe_in` reads it. Stash screen untouched. |
| `b` prefilled `<b>-restored` | `open_name_prompt` has no prefill | `open_name_prompt_with(state, kind, prefill)`; `open_name_prompt` calls it with `""`. |
| Cursor keeps its row | `selected: usize`, `expanded: Option<usize>` | `RowKey`; both become `Option<RowKey>`. |
| `b` lands on the new tip | `BranchCreate { checkout: false }` never lands | `landing_of` gives it `Commit(at)`. `moves_head` unchanged. |
| The J1 box | Not drawn (V3) | **24-a2** builds it in graft over the existing `PreviewBox`, docked over J. |

## Decided (the note; not open)
- **Stages:** `J` opens at once with the `●` rows (stage 1), as today.
  - A job reads the outside rows (stage 2), then another runs the lost scan and `lost_edits` (stage 3). Both are in
    `jobs/journal_list.rs`; `runner.rs` only dispatches.
  - Each lands by calling `build` again. **The cursor keeps its `RowKey`** across stages, unless the preselect applies.
    A key that no longer exists after a rebuild falls to the newest row.
  - Title suffixes (J1): `· finding lost commits…` / `· looked at the newest 500 steps` / `· couldn't look for lost
    commits: <why>`. Nothing once done.
- **Preselect:** the Ruling below (it replaces F1's first wording).
- **Rows:** pass 1b's mock-up.
  - Columns: glyph `●`/`$`, the source word `Graft`/`outside Graft`, time, words, and `git: "<subject>"` on `$` rows.
  - `✗` rows are indented `└ ✗ lost`, with the sha7, the quoted subject, and `was <b>'s tip · no branch holds it` /
    `· <b> is gone` / `was HEAD`, plus `+N under it`.
  - Lost-edit rows: `└ ✗ lost  your edit to <path> · replaced by this undo · kept in its backup`, with no hash.
  - The legend line: `● done in Graft: u can undo it · $ done outside Graft: u never touches it · ✗ a commit no branch
    holds`.
  - The muted last line: `a branch deleted outside Graft and never checked out leaves no trace here`.
  - **The first commit's record** (22b's question): git's `commit (initial): Start the api` reads
    `committed "Start the api" on main`. Made in graft-ops `journal_list::words` (one arm).
- **A `$` row's identity and its commit (V5).** `ReflogRecord.old` and `.new` are both `Option<Oid>`. The key is all
  four stable fields: `RowKey::Outside { refname, old, new, at }` (`old` in it: two records of one ref in one second can
  share `new`). The row's commit, for `b`: `new`, else `old`.
  - **A row with no `new`** (git recorded the ref ending at nothing): drawn as any `$` row — glyph, source, time,
    `words()`'s words (git's subject when its table has none), `git: "<subject>"`. Nothing hidden, nothing added.
  - `⏎` on it: the `RecoverFor` job, as every `$` row; `recover_for` decides what, the screen adds no rule.
  - The bar, from the row alone: `old` is `Some` and `refname` is a branch → `⏎ make <b> again`; otherwise no `⏎`
    hint (`⏎` still asks `recover_for`, and its refusal goes to the status row). Why: the bar never promises what the
    row can't name (UX checklist 16).
  - `b` on it: at `old`. Neither `old` nor `new` → status row `this step names no commit`.
- **Breakpoints (J1):**

  | Width | Columns |
  |---|---|
  | ≥180 | glyph · source · time · words · `git: "…"` |
  | 120–179 | no `git:` column; the selected `$` row shows it on a line under it |
  | <120 | no source word (the legend carries it) |

  At 24 rows the title, legend, status row (when shown) and bar stay, and the rows scroll.
- **The command log** of an expanded `●` row: `┆ git …` lines under `commands Graft ran` (was `    $ git …`).
- **Keys (§2.11, keymap diff 2b):**

  | Key | Does |
  |---|---|
  | `j`/`k`, `↑`/`↓` | move |
  | `⏎` on the `← u` / `← U` `●` row | `u` / `U`, as today (no preview) |
  | `⏎` on an older `●` | revert-to, as today (`PreviewSubject::RevertTo`, `→ ⏎`, ADR-0044 §2) |
  | `⏎` on `✗` / `$` / lost edit | the `RecoverFor` job (`recover_scope` facts, then `recover_for`). `Run(params)` → `open_preview(PreviewSubject::Operation { op: Recover(params), opened_by: "⏎" }, return_to_journal: false)` with **J left shown** under the box (V3); `y` confirms (ADR-0044 §1). `OfferBranch` → no preview; J's status row shows `<why> · b makes <name> at <sha7>`. `Err(refusal)` → the status row, its words |
  | `⏎` on a `$` switch row | `switch_to(state, SwitchTarget::Branch(<from>), "⏎", next_id)` (the M7 switch rule) |
  | `b` | on any row with a commit (`✗`: its tip; `$`: as V5 above): `open_name_prompt_with(NameKind::Branch { at, checkout: false }, prefill)`, prefill `<b>-restored` when the last `recover_for` on this row offered it, else `""`. On a lost-edit row: status row `a lost edit has no commit · ⏎ puts it back` |
  | `/`, `n`/`N` | find; next/previous match. With no find active, next/previous `✗` row |
  | `Space` | as today (the command log), keyed by `RowKey` |
  | `Esc`/`J` | close |

  Every other key stays swallowed. The bar is per row, from the row alone (no facts): `⏎ undo this` / `⏎ redo this` /
  `⏎ revert to before this` / `⏎ put <b> back to it` / `⏎ make <b> again` / `⏎ put your edit back` /
  `⏎ switch back to <b>`, or none (V5).
- **The put-back box (V3; 24-a2).** Not drawn today (NOTES V3). Built in graft, drawn with graft-ui's existing
  `PreviewBox`; **docked over J, J shown above it**, the way the stash box docks over S (`draw_stash`). Why over J and
  not over the graph: the graph cannot show a commit no branch holds, so the picture is the box's lines; and `Esc` is
  then J, untouched, on the same `RowKey`.
  - **Title:** `Preview: put main back to 9a8b7c6 "Add rate limiter"` / `Preview: make spike again at 4d5e6f7 "<subject>"`
    / `Preview: put back your edit to src/a.rs` (`+N more`). Subject unknown → the quoted part is left out (the sha7
    is already there).
  - **Lines** (labels padded to 10 columns):
    - MoveRef: `now       main → 3d4e5f6  Add metrics endpoint` (`expected`) / `after y   main → 9a8b7c6  Add rate
      limiter` (`to`).
    - Recreate: `now       spike is gone` / `after y   spike → 4d5e6f7  <subject>`.
    - Files: no `now`/`after y` lines (no ref moves); the files line says it.
    - Then `your files: …` from `PreviewPlan.files`: `Unchanged { not_checked_out }` → `your files: unchanged
      (<not_checked_out>)`; `Changes { come_back, go, set_aside }` → the clauses that apply, joined by ` · `:
      `<p> comes back` / `<p> +N more come back`; `<p> goes` / `<p> +N more go`; `1 unsaved file set aside, put back
      after` / `N unsaved files set aside, put back after`; none apply → `your files: unchanged`. `files` is `None` →
      no line (never invented).
    - Last: `u undoes it`.
    - Subjects: the resident history rows, then J's list `LostCommit.subject`; unknown → the sha7 alone, never blank.
  - **Footer and buttons:** `again = Some(("y", "put it back"))`, `confirm_key = false`, `with_choices(["Cancel",
    "Put back"])`, as the stash box sets `y apply`. The footer is the widget's widest fitting rung (`y put it back
    ←→ choose  ⏎ act  esc cancel` down to `y put it back  esc cancel`). **Differs from the J1 mock-up**, which shows
    no buttons row: every other preview has one (ADR-0038 §1, cursor on `Cancel`), and the same box must act the same
    way everywhere (UX consistency; m6 usability, `⏎` vs `y`). No graft-ui change.
  - **Keys:** the box is modal over J: `y` confirms, `esc` cancels, `←`/`→`/`⏎` choose and act; J takes none of them.
  - `y` → the box closes **and J closes** (the landing is on the graph). `esc` → the box closes; J is as it was, same
    `RowKey`. A `Refused` answer → the box closes (`preview_produced`), the refusal on J's status row.
  - The `N commits back on main` clause is **not drawn** (Background: new gap row).
- **The name prompt from J (V1 corrected).** V1 found the stash overlay **stays shown** under its prompt
  (`update/stash.rs:238-251` (NOTES)) and hides only when `⏎` creates (`name_prompt.rs:428` (NOTES); shown again if the
  preview fails, `:444` (NOTES)). J does the same: J stays shown under the prompt; `Esc` clears the prompt
  (`name_prompt.rs:263` (NOTES)) → J as it was, same `RowKey`; `⏎` creating → J closed, the landing below.
- **Landings** (via `landing_of` and `finished_lands_on`):
  - **MoveRef:** the graph, cursor on `to`, detail pane on it; `to` tagged `↺ put back`, `expected` (the old tip)
    `○ main was here`. Status `✓ put back · main has "<subject>" again · u undoes`. Not checked out → still lands on
    `to`, not HEAD.
  - **Recreate:** cursor on `at`, tagged `↺ put back`. Status `✓ spike is back at "<subject>" · u undoes`.
  - **Files:** the graph with the changes pane on the first path. Status `✓ your edit to src/a.rs is back · u undoes`
    (`+N more` for several).
  - **`b`:** cursor on the new branch's tip. Status `✓ made main-restored at "<subject>" · u undoes`.
  - `<subject>` comes from the list's `LostCommit.subject`; unknown → the sha7, never blank.
  - Tags are set on `Done`, after `claim_write` cleared the old ones, and clear on the next operation, as `✎ changed`
    does.
  - `J` again: the put-back is a `●` row (`put main back (the shell's reset)`), and its `✗` row is gone.
- **`u`** undoes the put-back (it's a journal entry). In J, `u` stays swallowed; it never acts on a `$` or `✗` row.
- **Status row in `J`:** one line under the rows. It shows `AppState::status`'s problem, else its notice, the way
  `draw_status_or_hints` picks them, **only while there is one** (status rule). Refusals inside J set
  `AppState::status` as everywhere else; nothing J-private.
- **:847 (V4; 24-0).** `Operation::DropPutBack { stash }` has no why (`graft-ops/src/operation.rs:156-159` (NOTES));
  its one constructor has `clash.why` and drops it (`graft/src/update/resolver/resolved.rs:170` (NOTES)). **Add the
  field**, don't drop the fix. Cost of dropping it: every set-aside clash he finishes keeps two names for one stash
  (done line `set-aside copy`, J's row `drop stash@{0}`), the confusion the m6 usability walk filed. Cost of adding
  it: ~10 files, mostly `..` in patterns; no format change.
  - `DropPutBack { stash: Oid, why: PutBackWhy }`. `describe_in`: `SetAside` → `drop the set-aside copy (stash@{n})`;
    `Pop` / `Apply` → `drop stash@{n}` (a pop's done line says `stash@{0} dropped`). `describe` (not listed):
    `SetAside` → `drop the set-aside copy (<short id>)`, else `drop stash <short id>` as today.
  - **Journal format:** unchanged if `Operation` is not serialised (the entry stores kind `StashDrop` and the
    description string, MAP `DropPutBack`, `describe_in`). V6 checks. If it **is** serialised: `why:
    Option<PutBackWhy>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; `None` reads `drop
    stash@{n}`.
  - Entries already on disk keep their recorded words (descriptions are stored at `apply`); say so in NOTES.

## Verify first (rule 7: file:line into NOTES before building on it)
- **V1–V5: done** (24-a NOTES below). Their results are folded into "Decided".
- **V6 (24-0)** Each `drop stash@{` in the read-only files above: a git command, a pop's words, or a set-aside's
  words? file:line each. A set-aside's words made anywhere but `describe`/`describe_in` → stop, `blocked: a second
  source of the drop's words`. And whether `Operation` derives `Serialize` (file:line); pick the journal-format branch
  above by it.
- **V7 (24-a2)** How `draw_stash` docks the box over S and what it does with S's bar (file:line). J copies it; name
  the site.

## Read first
- This ticket, then `scouts/24.md` (all; its line numbers are b90a49e4's).
- The note: pass 1b "Journal"; "Pass 2b" F1, F5, J1, keymap diff (2b) §2.11, "Journey counts (2b)", "Where he lands"
  (2b).
- `docs/design/02-keymap.md` §2.11, §2.3b (`y`); `docs/design/01-view-mockups.md` §1.10.
- `docs/research/ux-principles.md` §3.1, §3.6, checklist 3, 16–17.
- 22's, 23's, 03's (Decided, Handoff) and 09's NOTES; 24-0: 23's NOTES only.
- `crates/graft/MAP.md` sections: `update`, `update::row_tags`, `update::operate`, `update::banner`,
  `update::preview`, `update::journal`, `update::clash`, `update::name_prompt`, `update::switch`, `view` (`frame`
  children, `journal_view`), `jobs`. `crates/graft-ops/MAP.md`: `Operation::DropPutBack`, `describe_in`,
  `RecoverTarget`, `Operation::Recover`, `FilesEffect`, `PreviewPlan`, `journal_list`. `crates/graft-ui/MAP.md`:
  `journal_screen`, `graph_screen` `RowTag`, `preview_box`.

## Signatures (exact; visibility as written)
```rust
// ── 24-0 ── graft-ops::operation
Operation::DropPutBack { stash: Oid, why: PutBackWhy }   // was { stash: Oid }; or why: Option<PutBackWhy> (V6)
// describe_in / describe: signatures unchanged; arms as "Decided" :847.
// graft::update::resolver::resolved — the constructor: DropPutBack { stash: clash.stash, why: clash.why }

// ── 24-a ── graft::update::journal
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowKey {
    Graft(EntryId),
    Outside { refname: String, old: Option<Oid>, new: Option<Oid>, at: i64 }, // OutsideStep.record's own fields (V5)
    Lost(Oid),                                                              // the lost run's tip
    LostEdit(EntryId),                                                      // LostEdit.entry
}
pub(crate) fn row_key(list: &JournalList, index: usize) -> Option<RowKey>;
pub(crate) fn index_of(list: &JournalList, key: &RowKey) -> Option<usize>;
/// The commit `b` names on a row: ✗ its tip; $ `new`, else `old`; ● as today; lost edit None.
pub(crate) fn row_commit(list: &JournalList, index: usize) -> Option<Oid>;
/// The Ruling below. Pure: no state, no terminal.
pub(crate) fn preselect(list: &JournalList, is_fetch: impl Fn(EntryId) -> bool) -> Option<usize>;
// JournalState: `selected: usize` → `cursor: Option<RowKey>`; `expanded: Option<usize>` → `Option<RowKey>`;
// adds `touched: bool` (a key since `J`), `outside: Loading<Arc<Vec<OutsideRecord>>>`,
// `lost: Loading<Arc<LostFound>>`, `list: Option<Arc<JournalList>>`, `offered: Option<(RowKey, String)>` (the prefill).
/// `J`: stage 1 now, and the stage-2 read. `next_id` as `journal_key` takes it.
pub(crate) fn journal_enter(state: &mut AppState, next_id: /* journal_key's type */) -> Vec<Command>;

// graft::jobs::journal_list (new)
pub(crate) struct LostFound { pub unreached: Unreached, pub edits: Vec<LostEdit>, pub scan: LostScan }
// graft::jobs (Command / Product)
Command::ReadOutside { id: JobId }                                  → Product::Outside(Result<Arc<Vec<OutsideRecord>>, String>)
Command::FindLost { id: JobId, outside: Arc<Vec<OutsideRecord>> }   → Product::LostFound(Result<Arc<LostFound>, String>)
Command::RecoverFor { id: JobId, list: Arc<JournalList>, row: usize } → Product::RecoverFor(Result<Recover, Refusal>)

// graft::update::operate
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Landing { Head, Commit(Oid), Changes(PathBuf) }
/// Head iff op.moves_head(); Recover MoveRef{to} → Commit(to), Recreate{at} → Commit(at),
/// Files{paths} → Changes(paths[0]); BranchCreate{checkout: false, at} → Commit(at); else None. Exhaustive.
pub(crate) fn landing_of(op: &Operation) -> Option<Landing>;
// AppState::lands_on_head: Option<JobId>  →  AppState::lands: Option<(JobId, Landing)>; set by launch_with.

// graft::update::banner (stop.rs)
// HeadLanding { refs, walk } gains `on: Landing`; the private land_on_head seeks by `on`.
// Changes(path): waits for the working-tree read instead, then the changes pane on `path` (the KeptCursor shape
// relaunch::land_on_the_work uses; name the site).
pub(crate) fn finished_lands_on(state: &mut AppState, commands: &[Command], on: Landing); // params as finished_lands_on_head's
pub(crate) fn finished_lands_on_head(state: &mut AppState, commands: &[Command]);          // = finished_lands_on(.., Landing::Head)

// graft::update::recover (new)
pub(crate) fn done_status(params: &RecoverParams, subject_of: impl Fn(Oid) -> Option<String>) -> Status;
pub(crate) fn put_back_tags(params: &RecoverParams) -> Vec<(Oid, RowTag)>;

// graft::update::name_prompt
pub fn open_name_prompt_with(state: &mut AppState, kind: NameKind, prefill: &str); // same visibility as open_name_prompt

// graft-ui::graph_screen
pub enum RowTag { Changed, PutBack, WasHere { branch: String } }   // `↺ put back`, `○ <branch> was here`

// ── 24-a2 ── graft::update::recover
pub(crate) fn preview_title(params: &RecoverParams, subject_of: impl Fn(Oid) -> Option<String>) -> String;
/// now / after y / your files / u undoes it. Plain strings; `preview_box` sanitises them into PreviewBox.lines.
pub(crate) fn preview_lines(params: &RecoverParams, files: Option<&FilesEffect>,
                            subject_of: impl Fn(Oid) -> Option<String>) -> Vec<String>;
pub(crate) fn files_words(effect: &FilesEffect) -> String; // "your files: …"
// update::preview::preview_box: a Recover arm (title, lines, choices, again); confirm_preview: Recover → journal hidden.

// ── 24-b ── graft-ui::journal_screen
pub enum RowKind { Graft, Outside, Lost, LostEdit }
// JournalRowView gains: kind: RowKind, git_subject: Option<String>, under: Option<usize>
pub struct JournalStatusRow { pub words: String, pub problem: bool }
// JournalView gains: title_suffix: Option<String>, status: Option<JournalStatusRow>, enter_words: Option<String>
pub fn cut_at_word(text: &str, width: usize) -> String; // reuse an existing graft-ui word-cut if one exists; name it
// graft-ops (24-a, unchanged signature): journal_list::words' `commit (initial)` arm.
```

## Acceptance (name the verifying test at tick time; **(red)** = write it, see it fail on today's code, then build)
Part 0 (24-0): **done** (builder 24-0, 2026-09-29; NOTES 24-0 at the end)
- [x] **V6** in NOTES, file:line. — NOTES 24-0.
- [x] `a_stash_drop_has_one_name` (red; graft-ops): `SetAside` at 0 → `drop the set-aside copy (stash@{0})`; `Pop` →
  `drop stash@{0}`; not listed → `describe()`'s words per why. — `graft-ops/src/operation/tests.rs::a_stash_drop_has_one_name`
- [x] `the_drop_after_a_clash_carries_its_why` (red; graft, `resolved.rs`'s tests or `edits_words.rs`): a set-aside
  clash finished builds `why: SetAside`; a pop clash `why: Pop`. — `graft/src/update/operate/tests/edits_words.rs::the_drop_after_a_clash_carries_its_why`
- [x] Green, named: `stash_clash.rs`, `m6_stash_put_back`, `set_aside.rs`, the journal round-trip tests. Serde
  branch not taken (V6). — full workspace `cargo nextest run`, 4765/4765 passed, includes all four.

Part A (24-a):
- [ ] **Verify-first:** V1–V5 in NOTES (done, 80b72040).
- [ ] **Identity:** `the_cursor_stays_on_its_row_when_outside_rows_arrive` (red),
  `the_cursor_stays_on_its_row_when_lost_rows_arrive` (red), `a_vanished_row_falls_to_the_newest`,
  `space_expands_by_row_not_by_index` (red), `two_steps_of_one_ref_in_one_second_are_two_rows`.
- [ ] **Stages:** `j_opens_with_graft_rows_at_once`, `outside_rows_arrive_second`,
  `lost_rows_arrive_third_and_the_title_says_while`, `a_failed_scan_says_why_in_the_title`.
- [ ] **Preselect** (Ruling): `the_newest_lost_row_is_preselected` (red), `a_fetch_above_does_not_stop_the_preselect`,
  `no_preselect_when_a_graft_row_is_newer`, `a_lost_row_under_a_graft_row_is_never_preselected`,
  `no_preselect_after_a_key`, `n_goes_to_the_next_lost_row`.
- [ ] **Enter:** `enter_on_the_next_undo_row_is_u_as_before`, `enter_on_the_next_redo_row_is_redo_as_before`,
  `enter_on_a_graft_row_is_the_revert_as_before`, `enter_on_a_lost_commit_opens_the_put_back_preview` (red; J still
  shown), `enter_on_a_gone_branch_opens_make_it_again`, `enter_on_an_outside_row_puts_that_ref_back`,
  `enter_on_a_switch_row_switches_back`, `enter_on_a_lost_edit_puts_the_file_back`,
  `esc_on_the_put_back_preview_returns_to_j_on_the_same_row`, `moved_on_shows_the_words_and_b_prefills_the_name`,
  `b_on_any_row_opens_the_name_prompt`, `b_on_a_step_with_no_new_names_its_old`,
  `b_on_a_step_with_no_commit_says_so`, `esc_on_the_name_prompt_returns_to_j_on_the_same_row`,
  `creating_from_j_closes_j`, `open_name_prompt_with_prefills_and_ctrl_u_clears`.
- [ ] **Landing:** `landing_of_names_every_kind`,
  `put_back_lands_on_the_restored_commit_with_tags_and_words` (red),
  `a_put_back_of_an_unchecked_branch_lands_on_the_restored_commit_not_head`,
  `a_file_put_back_lands_on_the_changes_pane_on_the_file`, `b_from_j_lands_on_the_new_branch`,
  `b_from_the_graph_lands_where_it_was`, `the_tags_clear_on_the_next_operation`, `u_undoes_the_put_back`,
  `u_never_acts_on_outside_rows`. 03's and 10's landing tests green, named.
- [ ] **Carried:** `the_first_commit_reads_committed` (red; graft-ops), `a_refusal_in_j_sets_the_status` (red).

Part A2 (24-a2):
- [ ] **V7** in NOTES, file:line.
- [ ] **Words:** `the_put_back_box_says_now_and_after` (red; J1's MoveRef lines),
  `the_box_says_unchanged_when_not_checked_out`, `a_make_again_box_says_the_branch_is_gone`,
  `a_file_put_back_box_says_only_your_files`, `the_box_names_the_sha_when_the_subject_is_unknown`,
  `files_words_joins_the_clauses_that_apply` (one case per clause, the `+N more` plural, none → `unchanged`),
  `no_files_line_when_the_plan_has_none`.
- [ ] **Box:** `the_box_footer_is_y_put_it_back` (`again`, `confirm_key` false, choices `Cancel`/`Put back`, cursor 0),
  `keys_go_to_the_put_back_box_over_j` (red: `j` moves nothing in J, `y` confirms), `y_on_the_put_back_box_closes_j`
  (red), `a_refused_plan_closes_the_box_onto_js_status_row`, `the_put_back_box_draws_over_j` (render: J's title row
  above, the box at the bottom, J's rows between).
- [ ] Green, named: the stash box's tests (`draw_stash`'s), every `preview_box` test in graft, 10's
  `return_to_journal` tests, rebase-preview-keeps-drops' `R` tests.
- [ ] **Capture** `docs/shots/m7/put-back-preview-187x50.txt` (≤ 200 KB) from `the_put_back_box_draws_over_j`, Right
  line: `J's rows above; the box titled "Preview: put main back to <sha7> "<subject>""; now / after y / your files: /
  u undoes it; footer starts "y put it back"`.

Part B (24-b):
- [ ] **Drawing:** `an_aborted_entry_reads_aborted_by_you` (red; M6/68, replaces the `failed: aborted` test in
  `view/journal.rs`), `a_cut_row_cuts_at_a_word` (red; :848), `a_refusal_in_j_shows_on_its_status_row` (red; :217),
  `no_status_row_when_nothing_is_wrong`, `the_command_log_uses_the_dotted_rule` (red; replaces `journal_screen.rs`'s
  `$ git` tests), `the_bar_names_what_enter_does_on_each_row`, `no_enter_hint_on_a_step_with_neither`,
  `j_again_shows_a_graft_row_and_no_lost_row`, `put_back_and_was_here_tags_draw`,
  `the_name_prompt_draws_over_j` (V1 left it unchecked).
- [ ] **UI snapshots:** 187×50, 140×40, 100×30, and 187×24 (title, legend, status row and bar kept).
- [ ] **Walks** (`m7_walk`), on the reordered "lost work" fixture, each landing asserted from the frame and
  `len(keys) <= noted_keys(id)`: `lost_to_shell_reset` `J ⏎ y` = 3; `deleted_branch_back` `J n ⏎ y` = 4 (after 13, as
  F4); `edit_in_undo_backup` `J n ⏎ y` = 4. Stage 3 is a wait, not a key.
- [ ] **Captures**, each ≤ 200 KB, each with a Right line; they lead the contact sheet (24-a2's
  `put-back-preview-187x50.txt` among them, in journey order):
  - `docs/shots/m7/journal-187x50.txt`: `●`/`$`/`✗` rows, the preselected `✗` row, the legend, the muted last line.
  - `journal-140x40.txt`: no `git:` column; the selected `$` row's subject under it.
  - `journal-100x30.txt`: no source word.
  - `put-back-landed-187x50.txt`: `↺ put back`, `○ main was here`, the `✓` line.
  - `journal-after-put-back-120x40.txt`: the new `●` row; the D row gone.
  - `journal-expanded-120x40.txt`: `┆ git …` under `commands Graft ran`.
- [ ] **Re-shot (ADR-0048):** `docs/shots/m3-journal-expanded.txt` and `docs/shots/m3-contact-sheet.txt` no longer
  show `$ git`; name the capture test.
- [ ] MAP.md (graft, graft-ui) entries. NOTES: the scout answers used, every J capture by file as re-shot or unchanged,
  with why. V2: no J frame in m6 (NOTES) → no m6 re-shoot.

## Touches (check each; the report's **Touches:** line says how)
- **24-0 — `DropPutBack`'s shape:** every file in 24-0's list; the build is the check for patterns, the named tests
  for words. `AppState::clash_drop` (graft MAP) reads the variant: its tests green. CONTEXT.md already has "set-aside
  copy"; check. The stash screen's `d` (`git stash drop`, a different op): its snapshot unchanged.
- **03's landing** (`lands_on_head` → `lands`, `HeadLanding.on`): every reader of the field, the harness `settled` and
  prelude ("no key while a landing is owed"), `resumed`'s `--continue` landing, the m6 `LandedOnHead` wait. Check:
  grep `lands_on_head`/`land_on_head`/`head_landing`, 03's tests, m6 journeys green.
- **10's picker landing** (via `moves_head` → `Landing::Head`): its tests green.
- **Graph `b`** now owes a landing on its own commit: m5 walks using `b` green.
- **The revert-to and `u`/`U` rows:** unchanged. Check: the `RevertTo` preview tests in `tests/it/preview.rs`,
  `undo_by_key.rs`, `undo_over_drift.rs`, `m3_walk.rs` `journal_and_reads` green.
- **Every reader of `JournalState::selected`/`expanded`:** grep; each moved to `RowKey`.
- **The name prompt:** existing callers (`open_from_graph`, stash `b`, any 10 added) unchanged; their tests green. The
  create arm's new J close: stash `b`'s tests green (it closes S, not J).
- **The preview** (24-a2): `preview_box`'s other arms (the stash box, `take_title_of`, rebase-preview-keeps-drops'
  `R` arm), `confirm_preview`'s other subjects, `return_to_journal` (and 10's generalisation) — their tests green;
  `R keep` never set on a `Recover` box.
- **`view/frame/overlays.rs`** (24-a2): J's and S's draws; the stash-over-S tests and J snapshots green.
- **Row tags:** `RowTag` matches in graft-ui and `view/entries.rs`; 09's tag tests (`amend_into/tests.rs`).
- **`jobs/runner.rs`:** the new arms in 30j's exhaustive match; no catch-all added; rebase-preview-keeps-drops'
  OpenRewrite arm untouched.
- **graft-ops `words`:** 22's `lost_work` tests that quote `commit (initial)`.
- **Hint-bar tests** (`tests/it/hint_bar.rs`, the J rows): the per-row bar, updated and named.
- **J captures** (scout Q6): m3-journal-80x24, m3-journal-160x48, m3-journal-expanded,
  m3-08-journal-after-unstage-{80x24,160x48}, m4-07-journal-remote-rows-{80x24,160x48},
  m5-ui-preview-time-machine-{160x48,80x24}, m3-contact-sheet, m5-contact-sheet-captions.tsv. m6: none (V2). Each
  re-shot or said unchanged with why.

## Ruling (planner, 2026-09-26) — F1's preselect; review `docs/reviews/m7-22-journal-recover.md` M4
When stage 3 lands and no key has been pressed since `J`, the cursor moves to **the newest `✗` row that sits under a
`$` row, when that `$` row is newer than every `●` row except a fetch** (`OperationKind::Fetch`). Otherwise it stays
on the newest row. A lost edit (always under a `●`) and a commit lost under a `●` row are never preselected; `n`
reaches them. `preselect` holds the rule; the tests call it directly.
- Why: it is what he approved (rev 2's frame `docs/shots/m7-proto2-c-journal.txt:18-20`; the note's mock-up :239-241
  both put `● fetch 10:44` over the preselected `✗`). A fetch moves nothing he holds (J1, :1173). The shell's loss is
  the one only the journal answers; a `●` newer than the `$` means he worked in Graft since (UX checklist 3).
- Counts unchanged: 13 `J ⏎ y` = 3; 14 and 15 `J n ⏎ y` = 4 (note :1250-1252).
- **The fixture changes first** (graft-fixtures `lost_work` order and times, its tests; graft-ops
  `tests/it/lost_work.rs` row order), routed by the conductor. Newest first the list reads: `● fetch` ·
  `$ reset └ ✗ D` · `$ switch spike→main └ ✗ E` · … · `● undo └ ✗ your edit` · `● discard`. The shell steps after the
  discard/undo leave the lost edit showing and never commit `src/a.rs` into E.
- The note's F1 text and `02-keymap.md:456` are the conductor's to edit.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. `b`, `/` and `n` in `J`
are given by the 2b keymap diff; take no other key.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Four parts, one context each, **0 → A → A2 → B**; the conductor dispatches them as four groups.
- **24-0** (~25k, Sonnet): V6, the `why` field through every site, both `describe` arms, Part 0's tests.
- **24-a** (~65k): `RowKey` (V5) and the cursor, stages and jobs, preselect, keys, `RecoverFor`,
  `Landing`/`landing_of`/`finished_lands_on`, `update/recover.rs` (`done_status`, `put_back_tags`), `RowTag` variants
  and their draw, `open_name_prompt_with` and J under the prompt, the `words` arm, Part A's tests.
- **24-a2** (~30k): V7, the put-back box (words, `preview_box` arm, over J, key order, `y` closes J), its capture.
- **24-b** (~70k): `view/journal.rs` and `journal_screen` (`RowKind`, breakpoints, legend, status row, per-row bar,
  command log, `cut_at_word`, aborted words), snapshots, walks, captures, and the old J captures re-shot. Needs 24-a2
  merged and the fixture order.
If any compacts, say so; the conductor splits the rest.

## NOTES (builder 24-a, 2026-09-29, base a91ae83c) — **blocked at V4**
Verify-first done; nothing built. V4 hits the ticket's own stop condition.

- **V4 — blocked: DropPutBack has no why.** `Operation::DropPutBack { stash: Oid }`,
  `crates/graft-ops/src/operation.rs:156-159`; `describe_in` sees only `stash` (:1437-1446). The one constructor,
  `crates/graft/src/update/resolver/resolved.rs:170`, has `clash.why` in hand but drops it. Fixing :847 needs a
  `why: PutBackWhy` field on the variant (graft-ops `operation.rs` beyond the one arm, plus `resolved.rs` and
  `update/operate/tests/edits_words.rs:14`, both outside the Files list). Spec change → conductor.
- **V1.** Stash `b`: the prompt opens over the overlay without touching `stash.shown`
  (`update/stash.rs:238-251`). `Esc` only clears the prompt (`update/name_prompt.rs:263`), so the overlay is back.
  `stash.shown = false` only when ⏎ creates (`name_prompt.rs:428`), set back if the preview fails (:444).
  J today: `b` is not bound in J; `name_prompt.rs` never reads `journal` (grep), so J is untouched by the prompt.
  Whether the prompt box draws over the J overlay: not checked (moot until J binds `b`).
- **V2.** No J frame in m6: `grep -rl "UNDO JOURNAL\|space command log\|J/esc close" docs/shots/m6` → nothing
  (183 files, `m6-contact-sheet-captions.tsv` included).
- **V3.** Undrawn. `crates/graft/src` has no reference to `Operation::Recover` or `FilesEffect` (grep), and no file
  in graft or graft-ui draws `after y` (grep; only resolver `three_panes` tests match). The conditional files come
  into scope: `crates/graft/src/update/preview.rs` and `crates/graft-ui/src/preview_box.rs` (+ `preview_box/`).
- **V5.** `ReflogRecord { refname: String, old: Option<Oid>, new: Option<Oid>, at: i64, subject: String }`
  (`crates/graft-git/src/reflog.rs:22-32`). Stable key: `refname`, `old`, `new`, `at`. **`new` is `Option<Oid>`**,
  so the signature's `Outside { reference, new: Oid, time }` must be `Outside { refname: String, old: Option<Oid>,
  new: Option<Oid>, at: i64 }` (`old` added: two records of one ref in one second can share `new`).

## Planner ruling (2026-09-29) on V3–V5 — folded into "Decided", "Files", "Signatures", "Size" above
- **V4:** add `why` (part 24-0, Sonnet). Dropping the fix leaves two names for one drop on every set-aside clash.
- **V3:** 24-a2 builds the box in graft on the existing `PreviewBox`, docked over J; graft-ui `preview_box.rs` is
  **not** in scope. Buttons kept (differs from the mock-up; consistency). No `N commits back` clause (gap row).
- **V5:** `RowKey::Outside { refname, old, new, at }`, the builder's proposal. A row with no `new` draws as any `$`
  row; `⏎` asks `recover_for`; the bar says `⏎ make <b> again` only when `old` is a branch's; `b` names `old`.
- **V1 correction:** J stays shown under the name prompt (the stash's real behaviour), closes on create.

## NOTES 24-0 (builder, 2026-09-29, base be89c351)
- **V6.** `Operation` derives only `Clone, Debug, PartialEq, Eq` (`graft-ops/src/operation.rs:58`), not `Serialize` →
  non-Option branch: `why: PutBackWhy`. Journal format unchanged. Entries already on disk keep their recorded words
  (`apply` stores the description string); an old set-aside drop still reads `drop stash@{n}`.
- **V6 `drop stash@{` sites (read-only files):** `graft-ops/src/apply/set_aside/returned.rs:180-194` a command
  (`stash drop stash@{at}`); `graft-ops/src/operation/stash.rs:85` a command (plan doc); `graft/src/view/journal.rs:347,357`
  a test fixture's given description (a pop's `StashDrop`), not made; `graft/src/update/stash/tests.rs:397` a command.
  No second source of a set-aside's words.
- **Red:** both tests written against the new signature; on old code they do not compile (no `why` field) — red by
  construction, not separately run red.
- **Scout:** none; ticket names every site, grep at be89c351 matched. `update/clash.rs`, `update/state.rs`,
  `m6_stash_put_back.rs`, graft `MAP.md` needed no change (state keeps `PutBackWhy` beside the job id;
  `m6_stash_put_back.rs:483` asserts a pop's undo words).
