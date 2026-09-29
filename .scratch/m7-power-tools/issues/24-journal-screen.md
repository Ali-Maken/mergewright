# journal-screen — `J` shows the shell's steps and the lost commits as rows; `⏎` puts one back; he lands on it

Status: ready — respecified 2026-09-27 against `scouts/24.md` (main @ b90a49e4); **amended 2026-09-29** after 24-a's
verify-first (commit 80b72040): V3, V4, V5 decided below; a part **24-0** (the drop's why) and a part **24-a2** (the
put-back box) added. Scope unchanged. **Amended again 2026-09-29:** `u`/`U` in J are global, not swallowed (ruling at
the end; folded into "Keys", "Decided `u`", Part B, "Touches").
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
- **Not** `update/keys.rs` (the `u` ruling needs no code; if V8 finds `u` in J landing unlike `⏎` on the `← u` row →
  stop, `blocked: needs update/keys.rs`, naming the line).

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
| `u` swallowed in J | `u` global, handled before overlays (`update/keys.rs:212-216`, 24-a NOTES) | **Global stays** (ruling at the end). Spec amended; no code. |

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
  | `u` / `U` | global, as on every screen (§2.2, ADR-0002; `update/keys.rs:212-216` (NOTES)): undo / redo the journal's next entry — the `← u` / `← U` row — whatever row the cursor is on. Never a `$`, `✗` or lost-edit row. Lands as `⏎` on that row does (V8). Refused while an operation is live, as everywhere; the refusal on J's status row |
  | `Esc`/`J` | close |

  Every other key stays swallowed. The bar is per row, from the row alone (no facts): `⏎ undo this` / `⏎ redo this` /
  `⏎ revert to before this` / `⏎ put <b> back to it` / `⏎ make <b> again` / `⏎ put your edit back` /
  `⏎ switch back to <b>`, or none (V5). The bar does not add `u`: the legend line already says what `u` touches.
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
- **`u`** undoes the put-back (it's a journal entry). **In J, `u`/`U` are the global undo/redo** (amended 2026-09-29,
  ruling at the end): they act on the journal's `← u`/`← U` row, never on a `$`, `✗` or lost-edit row, whatever the
  cursor is on, and land as `⏎` on that row does.
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
- **V8 (24-b)** Where `u` pressed in J lands, against `⏎` on the `← u` row (which closes J:
  `enter_on_the_next_undo_row_is_u_and_closes_the_browser`). file:line for each path (`update/keys.rs:212-216` (NOTES)
  and J's `⏎` arm in `update/journal.rs`). Same (J closed, 03's undo landing) → test it and go on. Differs → stop,
  `blocked: needs update/keys.rs`, naming the line; don't change either path from 24-b's files.

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
// `u`/`U` in J: no new signature; the global route in update/keys.rs, unchanged.
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
- [x] **Verify-first:** V1–V5 in NOTES (done, 80b72040). — NOTES "builder 24-a, base a91ae83c".
- [x] **Identity:** `the_cursor_stays_on_its_row_when_outside_rows_arrive` (red),
  `the_cursor_stays_on_its_row_when_lost_rows_arrive` (red), `a_vanished_row_falls_to_the_newest`,
  `space_expands_by_row_not_by_index` (red), `two_steps_of_one_ref_in_one_second_are_two_rows`. — all five in
  `crates/graft/src/update/journal/tests.rs`, pass; driven through `outside_landed`/`lost_landed` (the handlers; the
  jobs are not dispatched yet, NOTES). **Not red-first** (NOTES): mutation-checked instead.
- [x] **Stages:** `j_opens_with_graft_rows_at_once`, `outside_rows_arrive_second`,
  `lost_rows_arrive_third_and_the_title_says_while`, `a_failed_scan_says_why_in_the_title`. — all four in
  `crates/graft/src/update/journal/tests.rs`, pass; seen red (NOTES remainder). The "title" is checked as the state it
  draws from (`list.scan`, `JournalState::finding_lost()`); the drawn suffix words are 24-b's. End to end over the real
  pool: `tests/it/journal_put_back.rs` (`journal_with_every_stage`, both walks).
- [x] **Preselect** (Ruling): `the_newest_lost_row_is_preselected` (red), `a_fetch_above_does_not_stop_the_preselect`,
  `no_preselect_when_a_graft_row_is_newer`, `a_lost_row_under_a_graft_row_is_never_preselected`,
  `no_preselect_after_a_key`, `n_goes_to_the_next_lost_row`. — all six in `update/journal/tests.rs`, pass. Not red-first
  (NOTES).
- [x] **Enter:** `enter_on_the_next_undo_row_is_u_as_before`, `enter_on_the_next_redo_row_is_redo_as_before`,
  `enter_on_a_graft_row_is_the_revert_as_before`, `enter_on_a_lost_commit_opens_the_put_back_preview` (red; J still
  shown), `enter_on_a_gone_branch_opens_make_it_again`, `enter_on_an_outside_row_puts_that_ref_back`,
  `enter_on_a_switch_row_switches_back`, `enter_on_a_lost_edit_puts_the_file_back`,
  `esc_on_the_put_back_preview_returns_to_j_on_the_same_row`, `moved_on_shows_the_words_and_b_prefills_the_name`,
  `b_on_any_row_opens_the_name_prompt`, `b_on_a_step_with_no_new_names_its_old`,
  `b_on_a_step_with_no_commit_says_so`, `esc_on_the_name_prompt_returns_to_j_on_the_same_row`,
  `creating_from_j_closes_j`, `open_name_prompt_with_prefills_and_ctrl_u_clears`. — in `update/journal/tests.rs`
  unless named. The three "as before" under their old names: `enter_on_the_next_undo_row_is_u_and_closes_the_browser`,
  `enter_on_the_next_redo_row_is_still_capital_u`, `enter_on_an_older_row_opens_a_preview_of_reverting_to_it`.
  `enter_on_a_lost_commit_opens_the_put_back_preview` (24-a, via the answer) plus
  `enter_on_a_lost_commit_asks_what_it_puts_back` (the key → `RecoverFor`; red seen). `enter_on_a_gone_branch_opens_make_it_again`,
  `enter_on_an_outside_row_puts_that_ref_back`, `enter_on_a_lost_edit_puts_the_file_back`: key → `RecoverFor` with the
  on-screen list and row, then a scripted answer → the preview over J (red seen). The rest as named, written by 24-a;
  `creating_from_j_closes_j`, `open_name_prompt_with_prefills_and_ctrl_u_clears` in `update/name_prompt/tests.rs`. The
  real `recover_for` → preview: `tests/it/journal_put_back.rs::u_undoes_the_put_back`. All pass.
- [x] **Landing:** `landing_of_names_every_kind`,
  `put_back_lands_on_the_restored_commit_with_tags_and_words` (red),
  `a_put_back_of_an_unchecked_branch_lands_on_the_restored_commit_not_head`,
  `a_file_put_back_lands_on_the_changes_pane_on_the_file`, `b_from_j_lands_on_the_new_branch`,
  `b_from_the_graph_lands_where_it_was`, `the_tags_clear_on_the_next_operation`, `u_undoes_the_put_back`,
  `u_never_acts_on_outside_rows`. 03's and 10's landing tests green, named. — the first seven in
  `update/operate/tests/landing.rs` (`b_from_j_lands_on_the_new_branch` added by the remainder: green on first run,
  **not** seen red — the code was 24-a's). `u_undoes_the_put_back`, `u_never_acts_on_outside_rows` in
  `tests/it/journal_put_back.rs` (real pool, real git; `main` read back with `rev-parse`). 03's:
  `update/operate/tests/lands_on_head.rs` (5) and `tests/it/land_on_result.rs`; 10's:
  `update/switch/tests.rs::{a_switching_create_lands_on_the_new_branch, switch_words_on_landing}`. All green in the full
  run (4831/4831). `u_never_acts_on_outside_rows` asserts what the code does, not "u swallowed in J" (NOTES remainder).
  **Amended 2026-09-29:** what the code does is now the spec (ruling at the end); the test stands as written.
- [x] **Carried:** `the_first_commit_reads_committed` (red; graft-ops), `a_refusal_in_j_sets_the_status` (red). —
  `graft-ops/src/journal_list/tests/words.rs::the_first_commit_reads_committed` (red seen by 24-a);
  `graft/src/update/journal/tests.rs::a_refusal_in_j_sets_the_status` (passes; **not** seen red, 24-a NOTES).

Part A2 (24-a2): **done** (builder 24-a2, 2026-09-29; NOTES 24-a2 at the end)
- [x] **V7** in NOTES, file:line. — NOTES 24-a2.
- [x] **Words:** `the_put_back_box_says_now_and_after` (red; J1's MoveRef lines),
  `the_box_says_unchanged_when_not_checked_out`, `a_make_again_box_says_the_branch_is_gone`,
  `a_file_put_back_box_says_only_your_files`, `the_box_names_the_sha_when_the_subject_is_unknown`,
  `files_words_joins_the_clauses_that_apply` (one case per clause, the `+N more` plural, none → `unchanged`),
  `no_files_line_when_the_plan_has_none`. — all seven in `crates/graft/src/update/recover/tests.rs`, pass; seen red
  against empty stubs (fail-fast stopped after the first two failures; the rest not individually seen red).
- [x] **Box:** `the_box_footer_is_y_put_it_back` (`again`, `confirm_key` false, choices `Cancel`/`Put back`, cursor 0),
  `keys_go_to_the_put_back_box_over_j` (red: `j` moves nothing in J, `y` confirms), `y_on_the_put_back_box_closes_j`
  (red), `a_refused_plan_closes_the_box_onto_js_status_row`, `the_put_back_box_draws_over_j` (render: J's title row
  above, the box at the bottom, J's rows between). — all five in `crates/graft/tests/it/put_back_box.rs` (real pool,
  real git), pass. Red seen: `y_on_the_put_back_box_closes_j`, `the_put_back_box_draws_over_j`.
  **`keys_go_to_the_put_back_box_over_j` was green on 24-a's code** (not red): `preview_key` already runs in
  `boxes_key` (`update/keys.rs:69`) before `journal_key` (:264) — no routing site changed.
  `a_refused_plan_…` hands the refusal to `preview_produced` directly (NOTES).
- [x] Green, named: the stash box's tests (`update/stash/tests.rs`, e.g. `a_refused_stash_box_offers_only_esc_back`,
  `an_index_only_stash_names_its_staged_paths_in_the_box`; `tests/it/m6_stash_put_back.rs`), every `preview_box` test
  in graft (`update/preview/tests.rs`, `tests/it/preview.rs`, `view/tests.rs` preview-box tests), 10's
  `return_to_journal` tests (`update/journal/tests.rs`, `update/tests.rs`), rebase-preview-keeps-drops' `R` tests
  (`update/preview/tests.rs`, `tests/it/m7_walk/rebase_line.rs`). — full workspace `cargo nextest run`, 4843/4843.
- [x] **Capture** `docs/shots/m7/put-back-preview-187x50.txt` (≤ 200 KB) from `the_put_back_box_draws_over_j`, Right
  line: `J's rows above; the box titled "Preview: put main back to <sha7> "<subject>""; now / after y / your files: /
  u undoes it; footer starts "y put it back"`. — 2.3 KB, checked by `the_put_back_box_draws_over_j`; Right line in
  TESTING.md "M7: the put-back box over J"; caption row appended to `m7-contact-sheet-captions.tsv`, sheet rebuilt
  (31 captures). Colour unverified.

Part B (24-b):
- [ ] **Drawing:** `an_aborted_entry_reads_aborted_by_you` (red; M6/68, replaces the `failed: aborted` test in
  `view/journal.rs`), `a_cut_row_cuts_at_a_word` (red; :848), `a_refusal_in_j_shows_on_its_status_row` (red; :217),
  `no_status_row_when_nothing_is_wrong`, `the_command_log_uses_the_dotted_rule` (red; replaces `journal_screen.rs`'s
  `$ git` tests), `the_bar_names_what_enter_does_on_each_row`, `no_enter_hint_on_a_step_with_neither`,
  `j_again_shows_a_graft_row_and_no_lost_row`, `put_back_and_was_here_tags_draw`,
  `the_name_prompt_draws_over_j` (V1 left it unchecked).
- [ ] **`u` in J (ruling at the end):** V8 in NOTES, file:line. `u_in_j_undoes_the_next_entry_from_any_row`
  (`tests/it/`: cursor on a `✗` row, `u` → the `← u` entry undone, the `✗` row's ref unchanged by `rev-parse`; lands
  as `enter_on_the_next_undo_row_is_u_and_closes_the_browser` does), `u_in_j_during_an_operation_shows_the_refusal_on_js_status_row`
  (frame: `finish or abort the <kind> first · ⇥ banner` on the status row). Not red-first: the code exists (say so).
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
- **Global `u`/`U` in J** (ruling at the end): no code change. `02-keymap.md` §2.11 gains a row (the conductor applies
  it); `u_never_acts_on_outside_rows` and 24-b's two `u` tests are the check. Any J hint-bar or help text that lists
  swallowed keys: grep `swallow` in `view/journal.rs`, `journal_screen.rs`, `tests/it/hint_bar.rs`.
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
  command log, `cut_at_word`, aborted words), snapshots, walks, captures, and the old J captures re-shot, V8 and the two
  `u` tests. Needs 24-a2 merged and the fixture order.
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

## NOTES 24-a (builder, 2026-09-29, base db5ac0fc) — **partial: blocked at the job seam; compacted once**
**Compacted mid-ticket** (context ran out once; resumed from the summary). Conductor: split the rest as below.

**Blocked: needs `crates/graft/src/update/events/command.rs`** (held by rebase-preview-keeps-drops), plus
`update/events.rs` (`Task`/`Product`) and `event_loop/label.rs` — none in 24-a's list. The Files list says the
`Command`/`Product` variants go in `jobs/mod.rs`; they do not live there (`Command` is `update/events/command.rs`,
`Product`/`Task` `update/events.rs`). Signatures wanted:
- `Command::ReadOutside { id: JobId }` → `Product::Outside(Result<Arc<Vec<OutsideRecord>>, String>)`
- `Command::FindLost { id: JobId, outside: Arc<Vec<OutsideRecord>> }` → `Product::LostFound(Result<Arc<jobs::journal_list::LostFound>, String>)`
- `Command::RecoverFor { id: JobId, list: Arc<JournalList>, row: usize }` → `Product::RecoverFor(Result<Recover, String>)`

**Exact next step:** add the three variants + labels; dispatch in `jobs/runner.rs` to `jobs::journal_list::{read_outside,
find_lost, recover_for}`; route the products in `update/mod.rs` to `journal::{outside_landed, lost_landed,
recover_answered}` (on `outside` Ready → ask `FindLost`); in `journal_enter` set `outside = Pending(id)` and push
`ReadOutside`; in `journal::enter`'s `Outside | Lost | LostEdit` arm replace the placeholder
`"putting this back is not built yet"` with `recovering = Some((id, key))` + `RecoverFor`. Then remove the six
`expect(dead_code)` markers (they say "the seam 24-a stopped at"). Then write `outside_rows_arrive_second`,
`enter_on_a_gone_branch_opens_make_it_again`, `enter_on_an_outside_row_puts_that_ref_back`,
`enter_on_a_lost_edit_puts_the_file_back` (key → command), and the Landing box's `b_from_j_lands_on_the_new_branch`,
`u_undoes_the_put_back`, `u_never_acts_on_outside_rows` (need the jobs / `it/` walks).

**Built and tested (unit, scripted state; nothing observed in a running app):**
- graft-ops `words`: `commit (initial): X` → `committed "X" on main` (`the_first_commit_reads_committed`, red first).
  `tests/it/lost_work.rs`'s last line updated to match (Touches: "its test").
- graft-ui `RowTag::PutBack` / `WasHere { branch }` + draw (`put_back_and_was_here_tags_draw_after_the_subject`).
- `update::journal`: RowKey cursor/expanded, `rebuild` per stage, `preselect`, `n`/`N`, `b` (prefill from `offered`),
  `⏎` on a `$` switch row → `switch_into`, `recover_answered` (Run → preview with J shown; OfferBranch → sticky + `b`
  prefill; Err → sticky). Tests in `update/journal/tests.rs` (34, incl. the 12 old ones adjusted to RowKey).
- `update::recover`: `done_status`, `put_back_tags`, `tag_rows`, `subject_in` (`update/recover/tests.rs`, 6).
- Landing: `operate::Landing`/`landing_of`, `AppState::lands`, `HeadLanding::on`, `banner::finished_lands_on` /
  `lands_on_changes` (`update/operate/tests/landing.rs`, 6: `landing_of_names_every_kind`,
  `put_back_lands_on_the_restored_commit_with_tags_and_words`,
  `a_put_back_of_an_unchecked_branch_lands_on_the_restored_commit_not_head`,
  `a_file_put_back_lands_on_the_changes_pane_on_the_file`, `b_from_the_graph_lands_where_it_was`,
  `the_tags_clear_on_the_next_operation`). 03's `lands_on_head.rs` and 10's landing tests green (full nextest).
- `open_name_prompt_with`; create from J closes J (`open_name_prompt_with_prefills_and_ctrl_u_clears`,
  `creating_from_j_closes_j`, `esc_on_the_name_prompt_returns_to_j_on_the_same_row`).
- Also written, pass, **not ticked** (their box needs the dispatch): `j_opens_with_graft_rows_at_once`,
  `enter_on_a_switch_row_switches_back`, `enter_on_a_lost_commit_opens_the_put_back_preview` and
  `esc_on_the_put_back_preview_returns_to_j_on_the_same_row` (via `recover_answered`, not the key),
  `moved_on_shows_the_words_and_b_prefills_the_name`, `b_on_any_row_opens_the_name_prompt`,
  `b_on_a_step_with_no_new_names_its_old`, `b_on_a_step_with_no_commit_says_so`, `a_refusal_in_j_sets_the_status`.
  The old `enter_on_the_next_undo_row_is_u_and_closes_the_browser` / `..._redo_..._is_still_capital_u` /
  `enter_on_an_older_row_opens_a_preview_of_reverting_to_it` cover the three "as before" names, under their old names.

**Red-first: not kept.** The graft-side code was written before its tests (only `the_first_commit_reads_committed` and the
RowTag test ran red). Instead, a mutation (cursor always falls to row 0; preselect off) failed 7 of the journal tests;
the landing tests start the cursor on `0x05` and assert `0x04`/`0x03`/`0x02`, so they fail unless the seek happens.

**Deviations (say if wrong):** `recover_for` job returns `Result<Recover, String>` (`Refusal` has no failed-read
variant); `LostFound.subjects` added (a lost tip is on no graph row); `JournalState.recovering` added (stale `⏎`
answers dropped by id); `open_name_prompt_with` returns `bool` like `open_name_prompt`; `finished_lands_on` keeps
`&[Command]`, `Changes` goes through `lands_on_changes`; `row_commit` of a `●` row is `None` (`b` says "this step names
no commit"); the Done tick follows `what` (`put back ✓ · main has "S" again · u undoes`, `Status::Done`'s order); `/`
find in J not built; `LostScan::Running` while stage 3 was not asked.

**Files outside the list (all forced by a changed signature or a fixture that bypassed the new path):**
`view/journal.rs` (24-b's: `selected`/`expanded` now `selected_graft_row()`/`is_expanded`, 3 lines),
`update/tests.rs` + `view/tests.rs` (fixtures set `journal.entries` directly → now call `journal::rebuild`, as
`journal_event` does; `capture_m3_journal`'s shots re-checked **unchanged**), `update/banner.rs` (one re-export line),
`update/name_prompt/tests.rs`, `update/operate/tests/{lands_on_head,staging_not_kept}.rs` (`lands` rename),
graft-ui `graph_screen/render.rs` + `tests.rs`, graft-ops `tests/it/lost_work.rs`.

**Scout:** 24's (`.scratch/m7-power-tools/scouts/24.md`), plus targeted reads; no new scout.

## NOTES 24-a remainder (builder, 2026-09-29, base daa1e1d2 merged) — **Part A done**
No compaction. Stages, Enter, Landing, Carried ticked (tests named inline).

- **Wired:** `Command::{ReadOutside, FindLost, RecoverFor}` (`update/events/command.rs`), `Task::` and
  `Product::{Outside, LostFound, RecoverFor}` (`update/events.rs`), labels (`event_loop/label.rs`), runner arms +
  `read_outside`/`find_lost`/`recover_for` (`jobs/runner.rs`), routing in `update/mod.rs` (`answer`; the `started`
  and `took_fact` exhaustive lists). `journal_enter` asks `ReadOutside`; `outside_landed` (now takes `commands`,
  `next_id`) asks `FindLost` with its records; `⏎` on `$`/`✗`/lost edit asks `RecoverFor` with the list on screen
  (a second `⏎` cancels the first). All six `expect(dead_code)` gone; the "not built yet" status is gone.
- **Decided here (say if wrong):** stage 2 failing does **not** run stage 3; `lost` is set
  `Failed("couldn't read git's own records: <why>")` so the title says why there are no `✗` rows rather than
  claiming none. `JournalState::finding_lost()` added (`pub`, for 24-b's title suffix and the `it/` walk).
  `jobs::journal_list::LostFound` made `pub` (it rides in a `pub` `Product`; `private_interfaces` lint).
- **Outside the granted files (forced):** `update/place/write.rs` (`is_write`'s exhaustive `Command` list: the three
  are reads → `false`) and an `#[expect(clippy::too_many_lines)]` on it and on `update/mod.rs::took_fact` (61/60 each,
  exhaustive lists with no catch-all, the reason `answer` already carries). New `tests/it/journal_put_back.rs`
  (+ `mod` line in `tests/it/main.rs`). MAP.md (graft) updated.
- **Red:** 11 failing on 24-a's code with only the `outside_landed` signature changed: the 3 new stage tests,
  `j_opens_with_graft_rows_at_once` (amended to expect `ReadOutside`), the 4 new Enter key → `RecoverFor` tests,
  `closing_j_cancels_a_put_back_still_being_asked`, and `capital_j_…`/`escape_closes_it_…` (amended); then green.
  `esc_on_a_pending_journal_read_does_not_rerequest` went red after the build (Esc now cancels stage 2) and was amended. `b_from_j_lands_on_the_new_branch` and the two `it/`
  walks were not seen red.
- **Verified by the `it/` walk (real pool, real git):** shell `commit` + `reset --hard HEAD~1` → `J` → stages 2
  and 3 answer → the preselected `✗` → `⏎` → the real `recover_for` opens a preview whose plan lands `Ready` → `y`
  → `main` is the lost commit (`rev-parse`) → `u` → `main` back. **Not verified:** anything drawn (24-a2/24-b); the
  put-back box's words; `y` closing J (the walk presses `Esc` if J is still shown — 24-a2's `confirm_preview`).
- **Question (rule 7), spec vs code:** "In J, `u` stays swallowed" (Decided, **`u`**) does not match
  `crates/graft/src/update/keys.rs:212-216`: `u` is global (ADR-0002), handled before overlays, so in J it runs
  `Command::Undo` on the journal. It still never acts on a `$`/`✗` row (the walk: "nothing to undo", `main`
  unchanged, no journal entry). `u_never_acts_on_outside_rows` asserts that. Swallowing `u` in J would be a keymap
  change (keys.rs, not in 24-a's files) — planner's call.
- **Touches:** `Command` exhaustive matches — runner, `place/write.rs::is_write` (build is the check, grep
  `Command::ReadSwitchList` found both); `Task`/`Product` matches — `label.rs`, `update/mod.rs` (build); J's `Esc`
  now also cancels stage 2/3 — `escape_closes_it_…` and `esc_on_a_pending_journal_read_does_not_rerequest` updated to
  expect `CancelJob(stage 2)`, the launch read still untouched; 03/10 landings, m3/m5/m6 walks, hint-bar and
  J snapshots — full nextest 4831/4831; captures: none re-shot (no drawing code changed; J's draw is 24-b's).

## Planner ruling (2026-09-29) on 24-a's `u` question — folded into "Keys", "Decided `u`", V8, Part B, "Touches"
**Keep `u`/`U` global in J. Amend the spec; no code change.**
- **Why (UX heuristic 4, consistency: "same key, same meaning, every pane"; `docs/research/ux-principles.md:54`):**
  `u` is Undo on every screen (`02-keymap.md` §2.2, ADR-0002). A screen where it silently does nothing breaks that,
  and J is the one screen that *shows* what `u` will undo (the `← u` row). J's own legend, already decided, says
  `● done in Graft: u can undo it · $ done outside Graft: u never touches it`; swallowing `u` in J would make the
  legend false on the screen that draws it.
- **What "swallowed" was protecting** — `u` never acting on a `$`/`✗` row — holds under the global route: it acts on
  the journal's next entry only (`keys.rs:212-216` (NOTES); `u_never_acts_on_outside_rows`, real git).
- **Where he lands:** as `⏎` on the `← u` row (J closes, 03's undo landing). V8 checks the two agree; they differ →
  blocked, not patched from 24-b.
- **Refusal:** `u` while an operation is live refuses as everywhere (§2.2 ¶"refused while an operation is live"); the
  words on J's status row.
- **Keymap doc (conductor applies; `docs/design/02-keymap.md` §2.11, a row after `n` / `N`):**
  `| `u` / `U` | Undo / redo, global as everywhere (§2.2): the journal's next entry (the `← u` / `← U` row), whatever row the cursor is on; never a `$` or `✗` row. Lands as `Enter` on that row (M7) |`
  And the amendment line under the table gains: `u/U in the browser ruled global 2026-09-29 (m7 24-a question).`
- **Design note** (`m7-power-tools.md`, wherever pass 2b says `u` is swallowed in J): conductor, same wording.

## NOTES 24-a2 (builder, 2026-09-29, base 3038f05c) — **Part A2 done**
No compaction. No scout: 24's scout plus the ticket's named sites; each read directly (V7 below).

- **V7.** `draw_stash` docks the box by calling `draw_preview(state, frame, area, theme)` after the list
  (`view/frame/overlays.rs:65` at 3038f05c), into the whole overlay area: `preview_box_area` puts it at the bottom,
  above the frame's last row (`graft-ui/src/preview_box.rs:216-232`). S's bar: blanked by the list itself —
  `StashView::box_open = state.preview.is_some()` (`view/stash.rs:50`), and `StashScreen::footer` returns `""` when
  it is set (`graft-ui/src/stash_screen.rs:177-179`). **J copies it** in `draw_journal` (same file): `draw_preview`
  after the journal widget. J's bar is blanked by a small `WithoutBar` wrapper in `overlays.rs` (the widget's
  bottom row set to spaces while a preview is up), because J's view/widget (`view/journal.rs`, graft-ui
  `journal_screen.rs`) are 24-b's files. **24-b:** move it into the view (`JournalView::box_open`, as S) when it
  rewrites J's bar, and drop `WithoutBar`.
- **Built:** `update/recover.rs` `preview_title`, `preview_lines`, `files_words`; `update/preview.rs`
  `put_back_box` (the `Recover` arm), the J1 title set in every box state (planning / ready / refused, as
  `take_title_of`), `footer_keys`' `Recover` branch (`again = y put it back`, `confirm_key` false, `keep` false),
  `confirm_preview` closes J (`journal::close`, which also cancels J's pending stage jobs) before launching a
  `Recover`.
- **Differs from the ticket's line list (say if wrong):** the box also shows the plan's own git lines
  (`view.commands`, e.g. `git reset --hard <sha>`), as the stash box and every other box do (KNOWN_GAPS 229: the
  lines that run are shown). Title for several files reads `put back your edits to <p> +N more` (plural, as
  `done_status` says).
- **`a_refused_plan_closes_the_box_onto_js_status_row`:** the refusal (`MovedOnSince`) is handed to
  `preview_produced` directly — making the real plan refuse needs `main` to move between the `recover_for` answer
  and the plan job, and the pool has no seam there. It checks: box closed, J shown, J's frame identical to before
  `⏎` (same row), `Status::Sticky`. J's status row itself is 24-b's; the refusal is on `AppState::status`.
- **Capture:** J's rows in it are 24-a's drawing — `nothing yet` (this repo has no `●` rows; `$`/`✗` rows are
  24-b's to draw). The box is the point of the capture; 24-b re-shoots it. Fixed commit/reflog dates (`@1578000000`)
  and a fixed clock keep the sha (`fc48213`) and the frame stable.
- **Also edited:** `tests/it/journal_put_back.rs::u_undoes_the_put_back` — its "Esc if J still shown" fallback
  (waiting on 24-a2) replaced by `assert!(!state.journal.shown)`.
- **Verified:** the it/ tests drive the real app (real `recover_for`, real plan, real `git reset`); `main` read back
  with `rev-parse`. **Not verified:** by eye; colour; mouse clicks on `Put back` over J (the click path is the
  shared `preview_click`, untested for this box).

## NOTES 24-b (builder, 2026-09-30, base 26bd74d2) — **blocked at V8: needs `update/keys.rs`**
No compaction. Nothing built: V8 is the ticket's stop condition ("Differs → stop"). Scout: `scouts/24b-harness.md`
(harness only; kept for the resumed part).

- **V8 — differs (observed, not only read).**
  - `u` in J: `crates/graft/src/update/keys.rs:212-216` — the global `is_undo_key` arm calls `undo::undo_key` and
    returns. Nothing on that path closes J (`journal.shown = false` is set only at `update/journal.rs:664` (`close`),
    `:801` (time machine), `preview.rs:822`, `name_prompt.rs:313`; grep).
  - `⏎` on the `← u` row: `crates/graft/src/update/journal.rs:809-810` — `close(next, commands)` **then**
    `undo::undo_key` (pinned by `update/journal/tests.rs:207`
    `enter_on_the_next_undo_row_is_u_and_closes_the_browser`).
  - So after `u` in J, J stays shown over the result, and the undo's landing is dropped under it — the code says so
    on purpose: `update/undo.rs:1665-1670` `under_a_modal_the_landing_is_dropped` ("`u` works under the journal
    browser … the landing is dropped").
  - **Probe run** (real pool, real git, `.scratch/m7-power-tools/probes/24b-v8-u-in-j.rs`, the ticket's
    `u_in_j_undoes_the_next_entry_from_any_row`): put-back via `J ⏎ y`, then the shell loses a second commit, `J`,
    stages land, `u`. Result: `main` back to before the put-back ✓ (rev-parse), the `✗` row not put back ✓, **J still
    shown ✗** (`assert!(!state.journal.shown)` failed, :182 of the probe as appended). Not committed in `tests/it/`
    (red by design); append it to `journal_put_back.rs` to re-run.
- **What unblocks it (conductor/planner's call, not built):** in the `is_undo_key` arm, close J before
  `undo_key` when `journal.shown` (as `journal.rs:809` does), or have `undo::undo_key` do it. Either is outside 24-b's
  files. Cost of leaving it: `u` in J undoes but leaves him in J, with no landing on the graph — unlike `⏎` on the same
  row (UX consistency, the ruling's own reason). `under_a_modal_the_landing_is_dropped` would need its J half
  re-read against the ruling (it asserts the landing is dropped under J).
- **Remaining 24-b (all of Part B), unchanged:** drawing, snapshots, walks, captures, re-shots, MAP. Nothing in it
  depends on V8 except the two `u` tests; the conductor may split them off and resume the rest.
- **Scout findings to carry** (`scouts/24b-harness.md`): journey ids are kebab-case (`lost-to-shell-reset` 3,
  `deleted-branch-back` 4, `edit-in-undo-backup` 4; note :1260-1271), not the ticket's snake_case. The app tests
  (`journal_put_back.rs`, `put_back_box.rs`) do **not** use `graft_fixtures::lost_work`; they build a Trivial repo +
  commit + `reset --hard`. `lost_work::Graft` (fetch / discard / `undo_forced`) is implemented only in graft-ops'
  `tests/it/lost_work.rs:25,55` — the walks need an app-side impl (driving the real app's fetch/discard/undo).
  Captures: `check_shot`, rewritten with `GRAFT_UPDATE_SHOTS=1`. Unconfirmed: which code writes the `m3-08` / `m4-07`
  J captures (`coverage.rs:232` only checks they exist).
