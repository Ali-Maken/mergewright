# m1-the-graph/17 — Wrap: manual checklist, gaps sweep, checkpoint script

Status: ready
Build with: Opus
Blocked by: 16

Why: M1's checkpoint is not a test run — it is the user opening `git/git` and the merge-mistake fixtures and deciding whether the history shape is obvious at a glance. This ticket makes that possible in one command and makes sure nothing was quietly left half-done.

## Read first
- KNOWN_GAPS.md — every row, not just M1's
- PROMPT.md §7.1 Manual tier, §8 M1 row
- `docs/reviews/m0-foundations-round-2.md` "Unverified" section for the shape of an honest report

## Goal
The user can run one command per scenario, see the graph, and answer the checkpoint question. Everything unfinished is written down.

## Scope
In:
- `TESTING.md` completed for M1: the manual matrix with a pass condition per row — Ghostty, Kitty, WezTerm, iTerm2 (pixel); Terminal.app, Alacritty, VS Code, Zed (Unicode); tmux inside each of those; one SSH session. Pass condition per PROMPT.md §7.1: correct graph **or** clean fallback, never a broken screen or stray escapes.
- The checkpoint script: clone/locate `git/git` (`--filter=tree:0`, from ticket 02), plus the merge-mistake fixtures, and the exact commands to open each. Include what to look at: is the default branch a straight line, can you see where a merge went wrong, do the lanes stay distinguishable.
- A KNOWN_GAPS.md sweep: every row with expiry M1 is closed, or re-expired to a named later milestone **with a reason**. No row silently carried.
- README and CHANGELOG updated for what M1 actually does.
- The report to the conductor separates measured from assumed, and lists what only the user can verify.

Out:
- Fixing anything found at the checkpoint — that is the next round, after the user has looked.

## Acceptance
- [ ] `TESTING.md` M1 section complete; every row has a pass condition and a place to record the result.
- [ ] Checkpoint commands verified to run (the graph opens) on the builder's own terminal at least.
- [ ] KNOWN_GAPS.md has no M1-expiry row left undecided.
- [ ] Gates clean on the whole workspace.
- [ ] **The user's checkpoint question is unanswered until the user answers it** — this box stays unticked.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
