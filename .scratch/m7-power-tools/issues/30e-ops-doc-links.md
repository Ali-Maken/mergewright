# 30e ops-doc-links — 30c's Minor 3 check: graft-ops docs build with no broken intra-doc link

Status: draft
Build with: Sonnet. Five doc-comment links, each located in 30c's NOTES; no code change.
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md` (after 01a/01b and 03b, which hold `apply.rs`
and `apply/error.rs`).
Split by: planner, 2026-09-27, from 30c's open Minor 3 box (NOTES "Minor 3 check"). Not covered by any later ticket.
Crates: graft-ops only (doc comments).
Files: crates/graft-ops/src/{apply/error.rs, apply.rs, rewrite/dependents.rs} — doc comments only.

## Scope
30c NOTES: `RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc --no-deps -p graft-ops` fails with 5 errors,
all older than 30c: `apply/error.rs:95`, `:120` (`resume`), `apply.rs:80` (`inside_live_operation`), `apply.rs:93`
(`snapshot::chain_backup`), `rewrite/dependents.rs:66` (`plan`). Line numbers are at 30c's base; re-find by the named
item. Point each link at the item it means (a path that resolves), or unlink it if the item is private. No wording
change beyond the link.

## Acceptance
- [ ] The command above exits 0 (quote its last line in NOTES). Then tick 30c's Minor 3 box in 30c's file naming
      that command.
- [ ] Gates workspace-wide.

## Size
~10k.
