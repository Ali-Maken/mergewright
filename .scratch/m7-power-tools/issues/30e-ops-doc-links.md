# 30e ops-doc-links — 30c's Minor 3 check: graft-ops docs build with no broken intra-doc link

Status: done
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
- [x] The command above exits 0 (quote its last line in NOTES) — verified by running it, see NOTES. Then tick 30c's
      Minor 3 box in 30c's file naming that command — done.
- [x] Gates workspace-wide — fmt --check, clippy --all-targets -D warnings, nextest (4615 passed, 5 skipped),
      test --doc, all green.

## Size
~10k.

## NOTES
- `resume` (error.rs x2) → `super::resume`; `snapshot::chain_backup` → `crate::snapshot::chain_backup`;
  `plan` (dependents.rs) → `super::plan`; `inside_live_operation` is private (`apply/fold.rs:101`) → unlinked.
- Command last line: `Generated .../target/doc/graft_ops/index.html and 1 other file` (exit 0). Private-item link
  warnings remain (`restart_source`, `merge_msg`, `is_operation`); not errors under this lint, out of scope.
- No scout: five one-token doc edits located by rustdoc's own errors.
