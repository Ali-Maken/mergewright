# health — graft-ui MAP.md rewritten as a map

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1; revised 2026-10-09 for 37's § Addition ruling and graft-ui's ~636 `pub` items)

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
- [ ] With `graft-ui` added to `CHECKED` for one run, the `every_pub_item_is_in_its_map` failure names no item from a pass-1 module. NOTES quotes the `graft-ui: <N> missing` line, then the change is reverted (`map_coverage::every_pub_item_is_in_its_map`).
- [ ] No pass-1 section line is over 240 characters or carries a ticket id. Checked in the same run (`map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, with its failures, if any, outside pass-1 sections).

Pass 2, or the single pass:
- [ ] The three `map_coverage` tests pass with `graft-ui` in `CHECKED` (`map_coverage::every_pub_item_is_in_its_map`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, `map_coverage::every_map_item_still_exists`).
- [ ] `rg -c '^## File layout' crates/graft-ui/MAP.md` returns 0, quoted in NOTES.
- [ ] NOTES gives the line count before and after (`wc -l`), and lists any test-only `pub` item found, with its callers.

## Touches
- ui MAP.md.
- `CHECKED` (pass 2 only).
- MAP line citations in open KNOWN_GAPS rows and tickets go stale. NOTES lists the `rg 'graft-ui/MAP.md:'` hits for the conductor.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens per pass, or about 70k as a single pass.
