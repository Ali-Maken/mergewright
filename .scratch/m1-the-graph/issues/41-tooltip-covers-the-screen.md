# m1-the-graph/41 — Tooltip covers half the screen

Status: draft
Build with: set by design pass
Blocked by: M1 graph design pass (decision 2: tooltip), user approval of its ADR

Finding (user gallery walk, 2026-09-13, same screenshot): "Tooltip covers half the screen, truncates the subject column, no border, no visible dismiss." User proposal under evaluation: remove the hover popup, reveal a truncated subject in the hint bar, details move to M2's detail pane, SHA copy moves to a key and the context menu. That changes PROMPT.md §1.3 and DESIGN.md: needs an ADR and the user's approval. Ticket 36 (pin shows wrong commit) is on hold for the same decision.

## Scope, signatures, Build with
Filled in by the M1 graph design pass (planner-fable, user 2026-09-13), which decides the approach first. The conductor does not build this ticket until the pass has written its scope and the user has approved the ADRs it depends on.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`. Any visual result ends with a capture under `docs/shots/<NN>-*`; user judges by eye.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
