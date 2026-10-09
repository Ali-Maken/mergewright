# Scout 34 — split graft-ops `operation.rs`
Root main @ fc6972fe. Crate set: **graft-ops only** (+ `over_400.txt`, which does not exist yet; see below).

## File
`crates/graft-ops/src/operation.rs` = **1917 lines** (ticket says 1912). Already has children: `operation/{commit,conflict,finish_merge,history,recover,remote,rewrite,stage,stash}.rs`, `operation/tests.rs` (2006 lines).
- :15–23 `mod` children; :25–40 `use`; :42–56 re-exports.
- :60–173 `pub enum Operation` (114).
- :174–413 support types: `FactsScope`:174, `Target`:184, `PatchSide`:207, `CommitParams`:217, `StashParams`:230, `StashScope`+Default:241/252, `Invocation`+impl:264/274, `SetAside`:305, `RepoFacts`:313, `Incoming`:357, `amends_at_edit_stop`:370, `RewriteKey`+impl:379/386 (~240).
- :414–826 `pub enum Refusal` (**413 lines, one item**).
- :827–1164 `impl Refusal` (`explain`:838, `explain_rungs`:1147) (338).
- :1165–1313 words: `RefusalWords`:1165, `in_the_way_rungs`:1178, `in_the_way_words`:1244, `commits_them`:1280, `dashed`:1285, `ref_words`:1293, `name_paths`:1297 (~150).
- :1314–1341 `opens_in_progress`, `runs_in_progress` (pub(crate)).
- :1342–1915 `impl Operation` (574): queries `kind`:1345…`describe`:1608; `check_conflict_state`:1656, `check`:1715, `plan`:1761, `is_remote`:1798, `remote_names`:1805, `preview`:1819, `touches`:1833.
- :1916–1917 `#[cfg(test)] mod tests;`

## Seams / counts
1. types (:174–413) ~240. 2. `Refusal` enum ~413 → **cannot reach ≤400 without cutting the enum body: stop candidate**. 3. `impl Refusal` + words ~490 → two files (impl :827–1164 ~338; words :1165–1341 ~180). 4. `impl Operation` 574 → split queries (:1342–1655 ~313) / check+plan+preview (:1656–1915 ~260), two `impl` blocks.

## Pins
`operation/tests.rs` (2006 lines, unit tests via `super::*`). Paths re-exported; consumers in graft use `graft_ops::operation::…`. No captures.

## Contradictions
- "Inline tests move out first": **already out** (:1916–1917 → `operation/tests.rs`).
- `over_400.txt` and `file_size::every_file_over_400_lines_is_listed_with_its_count` **do not exist** on main (`find crates -name 'over_400*'` empty); ticket 33 not landed.
- Ticket 39 (ops-surface-narrowed) edits this file and the ops MAP; serialise.
