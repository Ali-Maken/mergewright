# Scout 41 — graft-ops MAP (HEAD 523f6c59)

## Guard
- `CHECKED` at `crates/graft/tests/it/map_coverage.rs:22-29`, six crates; graft-ops absent. One-line add.
- Public-set rules: `map_coverage/public_set.rs` doc (pub mod chain from lib.rs; `pub use` names; `pub fn/const` in inherent impl of a public type; no `#[cfg(test)]`).

## graft-ops public set — exact count could not confirm (no cargo run allowed)
- `lib.rs:10-36`: 25 `pub mod`; private `mod branch_config` (:12), `mod objects` (:26); one re-export `pub use apply::EditsAt` (:38).
- Upper bounds by grep: 405 lines `pub <kind> name` across `src/` (includes private modules/submodules, excludes impl methods); 69 indented `pub fn` (impl methods, rule 3 counts those on public types). Realistic set is likely 300–450 — **near the ticket's 400 "pass 2" threshold**.
- MAP.md: 469 lines, 376 `- ` bullet lines.
- `pub(crate)` occurrences in src: 205.

## What 39 left in MAP
- 9 lines mention `seam_rule`/`m8/39`; e.g. `MAP.md:186` `take` "pub for examples/snapshot_cost.rs:218 and tests/it/snapshot_raw.rs" — a stay-pub test/example-only item, the NOTES case in the ticket.

## Ticket vs code
- MAP is 469 lines, ticket says 453.
- Contract line numbers stale: trailer is `MAP.md:176` (not :160) and is now **`Tributary-Link:`**, `Graft-Link:` read-only legacy; also `SnapshotError::UnknownLink` :190 still says `Graft-Link:`. Backup identity :184 (not :169), now `tributary`/`tributary@backup`, item `BACKUP_AUTHOR_NAME/EMAIL`. plan.json contract :369 (not :352), on crate-private `rewrite::saved` → preamble per ticket rule. Ghost seed :341 `ghost_oid` (not :324); `preview/ghost.rs` re-exported per :449.
- M1's crate-private line list (:20, 39, …) predates 34/39 edits; numbers likely shifted — could not confirm each.

## Size / Build with
Grep-driven, ~60-80k tokens; risk is the set exceeding 400. **Build with: Sonnet** acceptable (test judges); Opus-low if set >400 proves real.
