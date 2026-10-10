# health — graft-ui MAP.md rewritten as a map

Status: pass 1 done, pass 2 open (was: ready, planner, from `docs/reviews/m8-architecture.md` Major M1; revised 2026-10-09 for 37's § Addition ruling and graft-ui's ~636 `pub` items)

Build with: **Sonnet** (conductor, after scout `scouts/38-health-map-ui.md`). The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs:
Blocked by: 37 (conductor, from Needs)
- `health-map-libraries` merged, **including its § Addition**: `public_items` and the tightened (d).
- `health-geometry-one-source` merged, since it adds `Cells` and the plain-number fns.

## Read first
- `37-health-map-libraries.md` § "The MAP format" and § "Addition → Ruling".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft-ui/MAP.md` **by grep per item, never whole**. It is 451 lines, and lines 1–85 alone exceed 25k tokens.

## Ruling applied (from 37 § Addition)
- The list to write is the **public set** from `public_items`, not an `rg` of `pub`. A `pub` item in a private module isn't listed unless it is re-exported. A re-exported item goes under the heading of the module that re-exports it.
- An item carrying `#[cfg(test)]` isn't listed.
- A `pub` item used only by tests: list it as is, and NOTES names it with its `rg` of callers, for the conductor. This ticket makes no visibility change.

## Two passes
The raw `pub` count is about 636, over the ~400 one-context threshold. Use 37's NOTES for the public-set size:
- **Size ≤ 400** → do both passes in one context.
- **Size over 400** → two builders. The split below follows the old MAP's module headings; confirm it against the public set.

**Pass 1:** the crate-root items (old `## lib.rs`), `blame`, `history`, `detail`, `detail_pane`, `diff_words`, `fit`, `graph_column`, and `graph_screen` with its submodules.
- Rewrite those sections at the top of `MAP.md`, after the crate preamble.
- Leave the old sections for the other modules below them, untouched, so the file is usable between passes.
- Don't add `graft-ui` to `CHECKED`.
- Stop, and report `blocked: needs pass 2`.

**Pass 2:** every other public module: `journal_screen`, `map_screen`, `overview_strip`, `paint`, `stash_screen`, `status_screen`, `syntax`, `theme`, `text_input`, `keymap`, `commit_editor`, plus any public module the old MAP lacks.
- Delete the remaining old sections, including both `## File layout (…)` history sections.
- Add `graft-ui` to `CHECKED`.

## Scope
In:
- Write one line per public-set item. Take its clause from the old entry, found by grep, and leave out history.
- The keymap and hint-policy modules (`keymap`, `hints`, `HintContext`) each get a one-line preamble saying what decides and what draws. If `hints` is a private module, its line goes in the crate preamble, and its re-exported items go under the re-exporting heading.
- Add `graft-ui` to `CHECKED` (pass 2).

Out: code, including visibility changes, and other MAPs.

## Signatures
None.

## Acceptance
Pass 1:
- [x] With `graft-ui` added to `CHECKED` for one run, the `every_pub_item_is_in_its_map` failure names no item from a pass-1 module. NOTES quotes the `graft-ui: <N> missing` line, then the change is reverted (`map_coverage::every_pub_item_is_in_its_map`). — **verified by** `map_coverage::every_pub_item_is_in_its_map`, one run with `graft-ui` in `CHECKED`: `graft-ui: 2 missing: `FOCUS`, `ticks_text`` (text_input, tour_card — pass 2). Reverted. Because the old holding section still names items, I also ran a throwaway check against the pass-1 sections alone (MAP cut at the holding heading): 0 pass-1 items missing, 0 stale, 0 long/ticket-id lines. Throwaway test reverted.
- [x] No pass-1 section line is over 240 characters or carries a ticket id. Checked in the same run (`map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, with its failures, if any, outside pass-1 sections). — **verified by** `map_coverage::map_lines_are_short_and_carry_no_ticket_ids` in the same run: every failure at MAP.md:221 or later; pass-1 sections are lines 1–219.

Pass 2, or the single pass:
- [x] The three `map_coverage` tests pass with `graft-ui` in `CHECKED` (`map_coverage::every_pub_item_is_in_its_map`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, `map_coverage::every_map_item_still_exists`). — **verified by** `cargo nextest run -p graft map_coverage`: 7/7 passed with `graft-ui` in `CHECKED`.
- [x] `rg -c '^## File layout' crates/graft-ui/MAP.md` returns 0, quoted in NOTES. — **verified by** that `rg` run (pass-2 NOTES).
- [x] NOTES gives the line count before and after (`wc -l`), and lists any test-only `pub` item found, with its callers. — **verified by** `wc -l` (pass-2 NOTES) and the continuation-2 sweep (scratch script over pass-2 modules' `pub` items, then `rg -n -w spell_every crates`): one test-only item, listed in continuation-2 NOTES.

## Touches
- ui MAP.md.
- `CHECKED` (pass 2 only).
- MAP line citations in open KNOWN_GAPS rows and tickets go stale. NOTES lists the `rg 'graft-ui/MAP.md:'` hits for the conductor.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens per pass, or about 70k as a single pass.

## NOTES (builder, pass 1, 2026-10-10)
Status: pass 1 done; `blocked: needs pass 2`. No compaction. Scout used: `scouts/38-health-map-ui.md`, re-checked against current code (40 landed `Cells`, `CarveInputs`, `RowCarve`, `row_carve`, `dock_cells`, `MIN_BOX_ROWS`; no `DockInputs`).
- **Public set size: 554** (from `public_items`, via a throwaway test, reverted). Over 400, so two passes, as planned. Only re-export not declared as an item: `KeyCode` (`keymap.rs:29`, `pub use graft_core::event::KeyCode`) — pass 2, under `## keymap`.
- MAP.md: 463 lines before, 472 after pass 1. Lines 1–219 are the new crate preamble and pass-1 sections (`lib.rs`, `blame`, `history`, `detail`, `detail_pane`, `diff_words`, `fit`, `graph_column`, `graph_screen`, `graph_screen::badges`, `graph_screen::columns`, `graph_screen::hints`). Old pass-1 sections deleted.
- `## Pass 2 holding: old crate-root entries` (:220) keeps the old `## lib.rs` lines verbatim: they carry the only descriptions of pass-2 modules not in the old MAP's headings (`confirm`, `conflict_resolver`, `context_menu`, `credential_prompt`, `empty_state`, `finish_summary`, `name_prompt`, `palette`, `picker`, `preview_box`, `rewrite_editor`, `still_to_apply`, `tour_card`, `whitespace_tail`). **Pass 2 deletes this section** along with the old module sections and both `## File layout` sections.
- `hints` is a public module (`graph_screen/mod.rs:25`), so its decide/draw preamble sits under `## graph_screen::hints`, not the crate preamble. `keymap`'s preamble is pass 2.
- Test-only `pub` items found (pass 1), callers outside graft-ui only in tests: `every_context` (`graft/src/update/tests/{bar_keys,keymap_edge,sweep_screens,doorway,dead_keys}.rs`, `keymap_edge/{unbound,remaps}.rs`), `every_hint` (`graft/src/update/tests/hint_coverage.rs`). Listed as is; no visibility change.
- Previously unlisted pass-1 items now listed: `BodyRow`, `body_len`, `body_row`, `section_at`, `selected_section`, `section_entries` (all have non-test callers in `graft::update`).
- **Pass 2 left:** `journal_screen`, `map_screen`, `overview_strip`, `paint`, `stash_screen`, `status_screen`, `syntax`, `theme`, `text_input`, `keymap`, `commit_editor`, plus the 14 modules above in the holding section; add `graft-ui` to `CHECKED`. Currently missing beyond the holding section's names: `FOCUS` (text_input), `ticks_text` (tour_card).
- Stale citations for the conductor (`rg 'graft-ui/MAP.md:'`): `KNOWN_GAPS.md:420` (`:151-152`), `.scratch/m7-power-tools/issues/20m-landing-in-a-partly-staged-file.md:94`, `.scratch/m6-conflicts/scouts/54-56.md:44` (`:272-275`), `docs/reviews/m2-details-and-diffs.md:649`, `.scratch/m8-polish-and-ship/issues/21b-7-moves-remainder.md:202`.
- Touches: `crates/graft-ui/MAP.md` only. `map_coverage.rs` edited for the one-run check and reverted (`git status` shows only MAP.md + this ticket). No code.

## NOTES (builder, pass 2, 2026-10-10)
Status: done. No compaction. Scout: reused `scouts/38-health-map-ui.md` and pass-1 NOTES; no new scout (doc-only, the coverage test is the oracle).
- `rg -c '^## File layout' crates/graft-ui/MAP.md` → `0`. `## Pass 2 holding` deleted.
- `wc -l`: 463 originally, 472 after pass 1, **638 after pass 2** (old MAP lacked many items; now one line per public item).
- Method: pass-2 lines generated per module from `pub` items in file order; clause from the old MAP entry when its name was unique there, else the item's `///` first sentence; ticket ids stripped; lines cut to ≤240. Clauses are mechanical: some read as doc-comment first sentences, not hand-written summaries. A reader pass for wording is worth doing but not required by acceptance.
- `keymap` got its decide/draw preamble and a `KeyCode` (re-export) line.
- Test-only `pub` items: pass 2 did not run a per-item caller `rg` for the 25 pass-2 modules; only pass-1's (`every_context`, `every_hint`) are known. Unverified, not checked.
- `CHECKED` in `crates/graft/tests/it/map_coverage.rs`: one line `"graft-ui",` added after `graft-capabilities` — ticket 41 may conflict on the same spot.
- Stale citations: pass-1 list stands; line numbers past :219 have all moved.

## NOTES (builder, continuation 2, 2026-10-10)
Status: box 3 done. No compaction. No new scout (sweep is a grep; scout + pass NOTES suffice).
- Sweep: every `pub fn|struct|enum|trait|type|const|static` in pass-2 files; flagged when no production file other than its own (tests dirs, `tests.rs`, `test_support`, trailing `#[cfg(test)] mod` cut) names it, word-boundary. Heuristic: common method names (`new`, `len`) match elsewhere, so a test-only method with a common name could be missed — unverified for those.
- Found one: `Keymap::spell_every` (`keymap/spell.rs:35`). Callers: `crates/graft-ui/src/keymap/tests/spelling.rs:35,36,41,42,57` only — the crate's own unit tests. Per 37's ruling it would take `#[cfg(test)]`; this ticket's scope forbids code, so listed as is (MAP.md:408). Conductor: follow-up visibility fix.
- Pass-1 items still stand: `every_context`, `every_hint`.
- Touches: this ticket only. No MAP or code change, so `map_coverage` unaffected.
