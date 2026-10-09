# M8 architecture pass: crate interfaces

Ticket: `.scratch/m8-polish-and-ship/issues/25-fable-architecture-pass.md`. Reviewer: planner (Fable). Date: 2026-10-09.

**No source file was opened.** The pass read `crates/**/*.rs` and `prototypes/` not at all. Inputs read:
- ARCHITECTURE.md.
- PROMPT.md §6.1–6.2.
- Every `crates/*/MAP.md`. `graft/MAP.md` (1158 lines) was too big to read whole inside the ticket's size, so it was read in ranges and by grep.
- KNOWN_GAPS row `M8/files-over-400-unrowed`.

Two inputs could not be used:
- **CLAUDE.md has no "Standing rules → Models" section.** Ticket 25 and the planner brief both cite it.
- **No M2/M5/M7 `/improve-codebase-architecture` review files exist** in `docs/reviews/`.

Line refs are `MAP.md` lines at `51ab16e5`.

## Verdict

- The layering holds, and so does the dependency table:
  - core sits at the bottom.
  - graph, git and term each depend on core only.
  - ops depends on git and core.
  - ui depends on graph, term and core.
  - app depends on all of them.
- Nothing in any MAP shows an edge the table forbids, so there are **no Critical findings**.
- The strain is at three seams:
  1. **The docs meant to make the interfaces legible have become a change log.** The MAPs total about 2,700 lines.
  2. **graft-ops makes its internals `pub`** and relies on a grep test to keep the app out.
  3. **Layout arithmetic is copied by hand** from graft-ui into `graft::update`, and sweeps keep the copies equal.

  All three are fixable with a few small tickets, and none blocks shipping.

## Critical

None.

## Major

### M1. MAP.md files are change logs, not maps (all crates; worst: graft, graft-ui, graft-ops)
- **Size.** graft 1158 lines, graft-ops 453 (~68k tokens), graft-ui 451 (lines 1–85 alone exceed 25k tokens), graft-git 359.
- **Who it fails.** A builder told "MAP before source" cannot read the MAP inside a ticket's budget. This pass could not read graft's whole.
- **What the entries hold besides items.** Ticket ids, review findings, measured numbers, rulings, test names and file-split histories. Examples:
  - ops MAP:160 runs to a paragraph per item.
  - graft MAP:957 and :971 are design essays.
- **Crate-private items** (MAP's job is the public surface):
  - ops MAP:20, 39, 95, 104, 187, 229–231, 245, 252, 276, 280, 342, 355, 366
  - git MAP:42–44, 270
  - graph MAP:114–115, 121
- **Duplicated sections.** graft MAP `## history` at :57 and :1141; `## event_loop` at :94 and :995.
- **Missing MAP.** `graft-capabilities` has no MAP.md at all.
- **Test harness and examples listed as items** (graft MAP:25–35).
- **Suggested change.**
  - One line per `pub` item: name, signature, one clause.
  - For the app crate, one line per module, plus the items that cross a module boundary.
  - The history already lives in `docs/log/` and the ADRs.
  - A guard test fails when a `pub` item is missing from its MAP, when a listed item no longer exists, when a line runs over 240 characters, or when a line carries a ticket id.
- **Candidate tickets:** `health-map-libraries` (the guard, plus core, term, graph, git, fixtures and a new capabilities MAP), `health-map-ops`, `health-map-ui` and `health-map-app`. These run after 27, because 27 rewrites MAP entries that name env vars and the binary.
  - **Signatures touched:** none; documentation plus one test.

### M2. graft-ops' execution internals are `pub`; the seam is held by grep, not by the compiler (graft-ops)
- **Exported but forbidden to the app:**
  - `snapshot::{take, commit_trees, move_backup_ref, chain_backup, raw_record}` (ops MAP:161–172)
  - `journal::{render_pending, render_outcome}` (:110)
  - `Journal::append_*` (:149)
  - `restore::execute` (:288)
- **Why it matters.** ARCHITECTURE.md:467–468 says "Default to `pub(crate)`". Instead, ARCHITECTURE.md:50–55 has `crates/graft/tests/it/seam_rule.rs` grep the app for `render_*`, `Step::` and `PathScope::`. So the "ops decides, app wires" rule is one regex away from silently breaking, and every item an agent sees as `pub` reads as permitted.
- **What the app really uses**, per graft MAP:
  - `snapshot::index_state` (:1098)
  - `apply::{apply, resume, Context}` (:1092)
  - `conflict`, `finish`, `preview`, `undo` and `switch_list` entry points
- **Suggested change:**
  - Narrow every item the seam rule forbids, and every item only ops' own tests use, to `pub(crate)`.
  - Move the tests that exercise them in-crate.
  - Keep `seam_rule.rs` for the enum-constructor clauses only, since visibility cannot forbid `Step::X` on a public enum.
- **Candidate ticket:** `health-ops-surface-narrowed`.
  - **Signatures touched:** visibility only. No signature changes shape.

### M3. Layout arithmetic is mirrored by hand between graft-ui and `graft::update` (graft-ui, graft)
- **Mirrors:**
  - `update::geometry::detail_split` mirrors `graft_ui::detail_pane::split_area` (ui MAP:121; ARCHITECTURE.md:267–276 grants a test-only exception just to compare them).
  - `chrome_rows` exists "so every layout mirror in `graft` agrees" (ui MAP:161).
  - `tour::card_rows` mirrors `layout::tour_rows` (graft MAP:352).
  - `banner::pending_rows_taken` mirrors `view::frame`'s carve (graft MAP:496).
  - `MIN_BOX_ROWS`/`BoxRect` mirror `preview_box`'s crate-private constant and dock (graft MAP:590–591).
- **What holds them equal.** Each copy is a place a click lands one row off from what is drawn. Render-and-read-back tests keep them equal. ADR-0058:63 already had to enumerate "every geometry mirror" when chrome rows changed.
- **The fix already exists in the codebase:**
  - `graft_ui::picker::picker_box((x, y, w, h))` gives `update` the box in plain numbers (ui MAP:50).
  - `view::hit` calls the renderer's own packing instead of copying it (graft MAP:957).
  - The purity guard forbids only `ratatui` inside `update/` (graft MAP:957).
- **Suggested change:**
  - graft-ui exposes each of these as a plain-number fn over `type Cells = (u16, u16, u16, u16)`.
  - `update` calls it, and the mirrors and their equality-only tests are deleted.
  - `pending_rows_taken` becomes the one fn that both `view::frame` and `update` call.
- **Candidate ticket:** `health-geometry-one-source`.
  - **Signatures touched:** in graft-ui, `Cells`, `detail_pane::split_cells`, `preview_box::{MIN_BOX_ROWS, dock_cells}` and `layout::tour_rows` (visibility). The `graft::update` mirrors are removed.

### M4. On-disk names are format contracts that 26/27 must treat as such (graft-ops, graft-git, graft-core)
This is not a new ticket. It is input to `26-rename-on-disk-migration` and `27-rename-names-sweep`. Neither ticket's Decisions names the first two items.

- **The `Graft-Link: step|resume` trailer in backup commits** (ops MAP:160): "No such line … reads as `Step`".
  - If the token is renamed, every existing `resume` link reads as `Step`.
  - That changes `current_step_chain` and the ConflictSet (ops MAP:92, :164–165) for backups already on disk.
  - 27's grep test (27:33) would flag the token as a user-visible `graft` string, because backup commits show in `git log`.
  - **Recommendation:** readers accept both tokens forever, and writers switch. Or keep the token and allow-list it in 27's grep test.
- **The live rewrite's plan file** `<git dir>/graft/rewrite/<entry-id>/plan.json` (ops MAP:352) is "a format contract — a relaunch reads what an older Graft left". 26's migration must move it *before* the relaunch reads it, and a test must cover an upgrade with a rewrite stopped mid-way.
- **Names to check by the "does anything read it back?" test:**
  - `label graft-fork-<n>` in a live todo (ops MAP:336)
  - the fixed `graft` identity on backup commits (:169)
  - the `graft-ghost` hash seed (:324): renaming it changes every ghost id
  - `.graft-*` temp files (git MAP:104)
  - `<common_dir>/graft/commit-graph-offer` (git MAP:79)
  - `GRAFT_ASKPASS_SOCKET` and `graft-askpass-<pid>` (git MAP:286)
  - `GRAFT_INTERNAL` = `refs/graft/` (core MAP:83)
  - the `duplicate_walk` allow-list (ops MAP:395)
  - `APP_NAME` (ui MAP:164)
  - Already renamed: the tour's `PRACTICE_PREFIX` is `tributary-practice-` (ops MAP:214).
- **The rule for each:** if anything reads it back, the reader takes the old name and the new one. If nothing reads it back, rename freely.

## Minor

1. **Dead field across a ruled boundary** (graft-graph MAP:24). `Row::marker` exists, "always None from graph". ARCHITECTURE.md:419–424 says graph "carries no radar data at all". Remove the field; the app sets `GraphEntry::marker` already.
2. **graft-git writes** (git MAP:76, :80, :104, :200). The crate is described as reads plus the runner, but it has:
   - `commit_graph::write`
   - `remember_declined`
   - `conflict::write_back`
   - `forecast_merge` writes objects

   Either move `write_back` to ops, or amend ARCHITECTURE.md's table to "reads, plus writes no journal entry covers". The second is cheaper and probably true.
3. **graft-ops forms user sentences** (ops MAP:6–10 and the `explain`/`words`/`moved_words`/`came_back_words`/`askpass_words`/`doing` fns). `KeyNames`/`OpKey` keep key names out of the crate, but the wording is in. This is a ruled design; recommend the set not grow. New wording goes in graft-ui or the app.
4. **graft-ops does a second job** (ops MAP:212–221): `tour` (practice repo, `tasks_done`, `tour_progress`), plus `init`/`clone`. These are first-run concerns, not operations on history. Leave them for M8. If the tour grows, it becomes its own module tree under the app.
5. **Type-name collisions across the surface.** Each one makes a `use` line ambiguous to a reader without the source:
   - ops `operation::RepoFacts` vs `mistake_radar::RepoFacts`; `operation::Target` vs `preview::Target`
   - core `journal::RefMove` vs ops `preview::RefMove`
   - git `status::Change` vs ops `finish::Change`
   - git `ParseError` ×3 and `ReadError` ×2
   - `Selection` in both `git::diff` and `graph::raster`
   - ui `RowMark` ×3 and `FileListView` ×2
   - `OpenError` in both git and ops

   Rename these at the next touch, not as a sweep.
6. **Test-only items are `pub`** (git MAP:219, :249, :268): `Mailmap::from_bytes`, `TipStats`, `GitInvocation::stops_hooks_first`. Use `#[cfg(test)]`, or move them under a `testing` module.
7. **App `Product`s carry errors as `String`** (graft MAP:168, :1074–1100). Some were already re-typed one at a time (`OpenFailure`, `TakeFailure`, `Operated::Failed.refused`). The rule worth writing into ARCHITECTURE.md: a product's error is typed exactly where `update` branches on it, and stays a string where it is only displayed.
8. **graft-ui does two jobs**: widgets, and keymap/config parsing plus hint policy (`HintContext`, ~118 coverage slots). The per-context key sets are held equal by sweeps that span crates. This is an observation only. Revisit if `HintContext` grows past what one file can list.

No tickets are written for the Minor findings. Each is a next-touch change, or one line in ARCHITECTURE.md for the conductor.

## KNOWN_GAPS `M8/files-over-400-unrowed` (→ 25): disposition

- **Rank.** The production hubs matter, and the test files don't.
  - A >1,400-line production file is a structural problem: one context cannot hold it, and every builder touching it pays for it. Examples: ops `operation.rs` 1912, graft `jobs/runner.rs` 1457 and `update/mod.rs` 1406.
  - A long test file costs a reader much less. 21b-7 ruling 2 already declined to trim test files one at a time.
- **Proposal:**
  1. **`health-file-size-ratchet`** gives an exact list and stops growth. It adds one test plus `over_400.txt`, listing every `.rs` over 400 lines with its exact count. A file that grows, an unlisted file over 400, and a stale row all fail the test. That turns "~100 + ~80, not counted" into a number, and closes "unrowed": every over-size file is then rowed by the list.
  2. **Three move-only splits for the worst production hubs:**
     - `health-split-operation` (ops)
     - `health-split-job-runner` (graft `jobs/runner.rs`)
     - `health-split-update-hub` (graft `update/mod.rs`)
  3. Everything else stays on the ratchet and shrinks when touched.
- **Row update for the conductor:** status `broken`, scope "tracked by `file_size` test, N files". N comes from the ratchet's first run. Close the row when N hits 0, or reword it to "test files accepted over 400" if the user agrees that's acceptable.

## Order relative to 26 / 27 / 28

- **Nothing here blocks 26/27/28** apart from M4's notes. 26 and 27 must take the trailer and the plan file as format contracts.
- Recommended order:
  1. `health-file-size-ratchet`, at any time.
  2. 26, then 27.
  3. Then the code tickets: `health-ops-surface-narrowed` → `health-split-operation`; `health-geometry-one-source`; `health-split-job-runner` → `health-split-update-hub`. After 27, a move-only diff stays move-only.
  4. `health-map-libraries` → `health-map-ops`, `health-map-ui`, `health-map-app`, last before 28, so the MAPs describe the final surface.
- If time is short, the splits and the MAP tickets can move to after M8 without risk to the promise. `health-ops-surface-narrowed` and `health-geometry-one-source` are the two that remove a live hazard.
