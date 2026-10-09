# Scout 38 — graft-ui MAP
Root main @ fc6972fe. Crate set: **graft-ui MAP.md** (doc) + **graft** `tests/it/map_coverage.rs` (`CHECKED`), which 37 creates.

## Now
`crates/graft-ui/MAP.md` = **459 lines** (ticket says 451). 160 lines match the ticket-id pattern; 134 lines exceed 240 chars.
Named items exist: `pub mod keymap` lib.rs:26; `pub mod hints` graph_screen/mod.rs:24; `pub enum HintContext` graph_screen/hints/context.rs:47.

## Count
`pub (fn|struct|enum|trait|type|const|static|mod)` lines in src: **636** (537 unique names; includes cfg(test) items, not filtered). **Over the ticket's ~400 split threshold** → expect `blocked: needs pass 2`.
Absent from MAP (sample): `body_len`, `body_row`, `BodyRow`, `FOCUS`, `section_at` (8 total).

## Pins
Only `map_coverage` (from 37). No captures.

## Contradictions / overlaps
- Depends on 40 (adds `Cells`) and 37 (creates the test) — both unlanded; `map_coverage.rs` absent.
- Edits `map_coverage.rs` `CHECKED`, same file as 37 and later map tickets.
