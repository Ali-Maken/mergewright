# m1-the-graph/39 — Compaction is wallpaper on merge-heavy history

Status: draft
Build with: set by design pass
Blocked by: M1 graph design pass (decision 1: merge-heavy default)

Finding (user gallery walk, 2026-09-13, git/git in Ghostty, screenshot `docs/shots/user-37-git-git-pixel-after-scroll.png`): "Compaction is wallpaper: every row shows +54. The 8-lane default is wrong for merge-heavy history."

## Scope, signatures, Build with
Filled in by the M1 graph design pass (planner-fable, user 2026-09-13), which decides the approach first. The conductor does not build this ticket until the pass has written its scope and the user has approved the ADRs it depends on.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`. Any visual result ends with a capture under `docs/shots/<NN>-*`; user judges by eye.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
