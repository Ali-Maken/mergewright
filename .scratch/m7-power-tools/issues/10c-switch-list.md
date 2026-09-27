# 10c switch-list — 10's graft-ops read: the picker's rows, ordered and filtered

Status: draft
Build with: Sonnet. A pure shaping read over 10a's refs; every order, exclusion and ranking rule is decided in 10.
Needs: `graft_git::switch_refs` (10a, done); `graft_git::remote::list` (exists).
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `10-branch-picker.md` (ADR-0051 §1: additive graft-ops, stands alone). 10 keeps
the overlay, keys, draw, walks and captures.
Crates: graft-ops only.
Files: crates/graft-ops/src/switch_list.rs (new, pre-registered by the conductor's seam commit);
crates/graft-ops/tests/it/switch_list.rs (new, seam); MAP.md (graft-ops). Outside this list → `blocked: needs <file>`.
Scout: none (scout 10 covers the read).

## Read first
- 10 "Decided": **Rows and order**, **Filter** only. 10 "Signatures": the graft-ops block.
- `10a-switch-refs-read.md` Signatures. `crates/graft-ops/MAP.md` history reads; `crates/graft-git/MAP.md` `remote::list`.

## Signatures
Exactly 10's graft-ops block: `SwitchKind`, `Relation`, `SwitchRow`, `SwitchList`, `switch_list`,
`filter_switch_rows`.

## Acceptance (tick 10's graft-ops box in 10's file, each test named)
- [ ] The eight graft-ops tests of 10 "Acceptance", first box.
- [ ] MAP.md (graft-ops): the rows' order, the remote-name rule, a remote ref under no listed remote left out.
- [ ] Gates workspace-wide.

## Touches
New API only; no caller until 10. Check: nothing else in graft-ops changes (`git diff --stat`).

## Size
~25k.
