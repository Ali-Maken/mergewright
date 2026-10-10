# health — graft-ops MAP.md rewritten as a map

Status: done (builder, 2026-10-10). Was: ready (planner, from `docs/reviews/m8-architecture.md` Major M1; revised 2026-10-09 for 37's § Addition ruling)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs: `health-map-libraries` **including its § Addition** (the guard, the format, `public_items` and the tightened (d)), plus `health-ops-surface-narrowed` and `health-split-operation`, all merged. The last two change what is public and where it lives.
Blocked by: 37, 39, 34 (conductor, from Needs)

## Read first
- `37-health-map-libraries.md` § "The MAP format" and § "Addition → Ruling".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft-ops/MAP.md` **by grep per item, never whole**. It is 453 lines, about 68k tokens.

## Ruling applied (from 37 § Addition)
- The list to write is the **public set** from `public_items`, not an `rg` of `pub`. Get it by adding `graft-ops` to `CHECKED` and reading the `every_pub_item_is_in_its_map` failure.
- A `pub` item in a private module isn't listed unless it is re-exported.
- The crate-private entries M1 names (old ops MAP :20, 39, 95, 104, 187, 229–231, 245, 252, 276, 280, 342, 355, 366) go. The tightened (d) fails on any that stay.
- A test-only `pub` item is `health-ops-surface-narrowed`'s job. If one survives, list it as is, and NOTES names it with its `rg` of callers, for the conductor. No visibility change here.

## Scope
In:
- Write one line per public-set item. Take its clause from the old MAP entry, found by grep. Leave out history, rulings and ticket ids.
- **Keep the format contracts** as one clause each, because 26/27 and future builders need them. If the item a contract lives on isn't in the public set, the clause goes in the crate preamble.
  - the `Graft-Link:` trailer and how a missing one reads (old :160)
  - the rewrite `plan.json` path and that it is a contract (:352)
  - the backup-commit identity (:169)
  - the ghost hash seed (:324)
- Add `graft-ops` to `CHECKED`.

Out: code, including visibility changes, and other crates' MAPs.

## Signatures
None.

## Acceptance
- [x] The three `map_coverage` tests pass with `graft-ops` in `CHECKED` (`map_coverage::every_pub_item_is_in_its_map`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, `map_coverage::every_map_item_still_exists`). Verified by workspace `cargo nextest run`, all three pass with `graft-ops` in `CHECKED`.
- [x] NOTES gives the line count before and after (`wc -l`).
- [x] The four format-contract clauses are present. NOTES quotes each line (checked by `rg` per contract; the commands are quoted).
- [x] NOTES lists any surviving test-only `pub` item with its callers, or says "none" with the `rg` that shows it.

## Touches
- ops MAP.md.
- `map_coverage.rs`'s `CHECKED`.
- 26/27 NOTES that cite `graft-ops/MAP.md:<line>`: the line numbers go stale, so NOTES lists the old→new line for the four contract entries.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens. If the public set is over about 400 items, stop after half the modules, in source order, and report `blocked: needs pass 2` with the split point.

## NOTES (builder)
- Scout used: `.scratch/m8-polish-and-ship/scouts/41-health-map-ops.md`. No compaction.
- Line count (`wc -l crates/graft-ops/MAP.md`): before 469, after 372. The public set fit one pass (no pass 2).
- Format contracts, each found by `rg -n '<pattern>' crates/graft-ops/MAP.md`; old → new line:
  - `rg -n 'Tributary-Link'` → :317 (old :176): "- `LinkKind` (enum) — `Step` or `Resume`, the backup commit's last line `Tributary-Link: step|resume`; `Graft-Link:` still read; no line reads as `Step`."
  - `rg -n 'plan.json'` → :4, preamble (old :369; `rewrite::saved` is crate-private): "Format contract (crate-private `rewrite::saved`): a rewrite's plan lives at `<git dir>/<names dir>/rewrite/<entry-id>/plan.json`; a relaunch reads what an older build left."
  - `rg -n 'tributary@backup'` → :324 (old :184): "- `BACKUP_AUTHOR_NAME` / `BACKUP_AUTHOR_EMAIL` (consts) — `tributary` / `tributary@backup`: author and committer of every backup commit."
  - `rg -n 'graft-ghost'` → :271 (old :341): "- `ghost_oid` (fn) — `sha1("graft-ghost\0<subject>\0<n>")`: a ghost row's stable synthetic id, never written to the object database."
  - Spec drift (per scout): the trailer is now `Tributary-Link:` (spec said `Graft-Link:`, still read); backup identity is `tributary`.
- Surviving test/example-only `pub` items (39's stay-pub set; listed as is, no visibility change):
  - `journal::render_pending` / `render_outcome` — `grep -rlE "render_outcome|render_pending" crates/graft-ops/tests crates/graft-ops/examples` → `tests/it/abort_merge.rs`, `tests/it/unmerged_index.rs`.
  - `snapshot::raw_record`, `snapshot::take` — `examples/snapshot_cost.rs`, `tests/it/snapshot_raw.rs`.
  - `restore::execute` — 6 files in `crates/graft-ops/tests/it/` (`undo_bytes`, `unmerged_index`, `undo_cancel`, `error_rows`, `octopus_merge`, `finish_merge`); `rg restore::execute crates/graft/src` hits doc comments only.
- Dropped as not in the public set: `Context::advance`, `Context::remote_run` (both `pub(crate)`), `PATHS_CAP`, `amends_at_edit_stop`, graft-git's `LostRun`/`Unreached` (used, not re-exported), every crate-private entry, the file-layout and examples sections. Added (were missing): `Operation::facts_scope`, `ForkStop`, `Replaced::extend`, `Journal::unresolved`.
- Clauses were condensed from the old entries; measured git behaviour and rulings now live only in source docs, `docs/log/` and the ADRs. Clause content was not re-verified against source beyond the names the guard checks.
- Touches: `crates/graft-ops/MAP.md`; `CHECKED` in `crates/graft/tests/it/map_coverage.rs` (one line; ticket 38 appends there too). 26/27 NOTES citing `graft-ops/MAP.md:<line>` are stale; the four contract lines' new numbers are above.
