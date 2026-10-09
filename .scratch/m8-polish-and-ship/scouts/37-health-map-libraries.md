# Scout 37 — MAP guard + library MAPs
Root main @ fc6972fe. Crate set: **graft** (new `tests/it/map_coverage.rs` + `main.rs` mod line) + MAP.md docs of graft-core, -term, -graph, -git, -fixtures, -capabilities. No source in those crates.

## MAP sizes now
core 86 (ticket 87), term 87 (88), graph 160, git 358, fixtures 50, capabilities **no MAP**.

## Rough gaps (my grep: `^\s*pub (fn|struct|enum|trait|type|const|static|mod) name` in src, name absent from MAP; includes cfg(test) items, so approximate)
| crate | pub items | absent | ticket-id-like lines | lines >240 |
|---|---|---|---|---|
| core | 94 | 0 | 16 | 7 |
| term | 78 | 6 (`as_str`, `Decoded`, `Decoder`, `feed`, `give_back`) | 2 | 5 |
| graph | 155 | 0 | 4 | 9 |
| git | 334 | 13 (`age`, `bare_repo`, `empty_dir`, `empty_repo`, `Fixture`…) | 89 | 68 |
| fixtures | 146 | 27 (`annotated_tag`, `BuildError`, `checkout_main`, `commit_ahead`…) | 15 | 19 |
| capabilities | 12 | 12 (`Capability`, `check`, `Decision`, `DecisionPath`…) | – | – |
Names present ≠ correctly described: not checked per item.

## Pins
`map_coverage.rs` does not exist yet (expected). Existing workspace-reading test to copy: `tests/it/seam_rule.rs`. No captures.

## Contradictions
- "Rewrite those five MAPs" + list of six crates: five rewrites + one new — consistent, just note.
- Line counts off by one (core, term).
