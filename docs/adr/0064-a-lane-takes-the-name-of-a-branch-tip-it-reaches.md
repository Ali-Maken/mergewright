# ADR-0064: A lane takes the name of a branch tip it reaches

Date: 2026-10-20 · Status: accepted

## Context
On a stack of ordinary branches (`feature/receipts` cut from `wallet-release`'s tip), the lower branch's commits
continued the upper branch's lane and identity (`assign_row.rs:62-63`), so they read `made on feature/receipts`, had
no branch-column name and drew as "this" (review `docs/reviews/ease-pass-07-14.md`, Major 2). The accepted mock gives
each stacked branch its own name on its rows (ADR-0063).

## Decision
When a continuing lane reaches another branch's tip, the rows from that tip down carry that branch's identity.
Exceptions: reserved lanes (trunk, integration), the lane's own remote copy, fork points (lowest lane still wins).
A merge's second-parent lane takes the tip's name from the merge row on.

Geometry: the line **runs straight on** in its column; thick `┃` turns thin `│` at the boundary and the branch column
names the lower branch. No new column. The alternative — bend into a new column per stack level — is what Sublime
Merge does, and its users have complained about the width since 2018 (sublime_merge#19,
`docs/research/graph-readability.md:12`). The user, asked, did not know; the conductor chose straight-on on the
planner's recommendation. The user judges the capture by eye.

## Consequences
DESIGN `03-graph-layout-algorithm.md:22`: identity is fixed until the lane reaches another tip, not for the lane's life.
A ref change that creates a branch deep in history renames rows below it. Spec: ticket 17.
