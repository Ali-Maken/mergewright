# m1-the-graph/28 — Wrap: manual checklist, checkpoint script, gaps sweep

Status: ready
Build with: **Sonnet** (medium) — mechanical: a script, docs, a sweep
Blocked by: 26, 27d, 29, 30c, 30d, 35c, 36, 37, 38, 39, 40, 41, 42, 43
Leaves runnable: yes — one command per checkpoint scenario.

Why: M1's checkpoint is the user opening `git/git` and the merge-mistake fixtures and judging whether the history shape is obvious at a glance. This makes that one command each and makes sure nothing was quietly left half-done. (Retired ticket 17, text in 35bec3e.)

## Read first
- KNOWN_GAPS.md — every row whose owner column says M1
- PROMPT.md §7.1 "Manual" row; §8 M1 row
- TESTING.md "The manual terminal checklist" and "Getting `git/git`"
- `docs/reviews/m0-foundations-round-2.md` "Unverified" section, for the shape of an honest report

## Scope
In:
- TESTING.md's manual matrix complete for M1: Ghostty, Kitty, WezTerm, iTerm2 (pixel); Terminal.app, Alacritty, VS Code, Zed (Unicode); tmux inside each; one SSH session. Pass condition per PROMPT.md §7.1: correct graph **or** clean fallback, never a broken screen or stray escapes. Merge the rows 08, 24, 25 and 29 filled in.
- `scripts/m1-checkpoint.sh <scenario>`: locates or clones `git/git`, builds the merge-mistake fixtures, prints and runs the command for each; says what to look at (default branch a straight line, where a merge went wrong, lanes distinguishable).
- KNOWN_GAPS sweep: every M1-owned row closed, or re-owned to a named later milestone with a reason. No row silently carried.
- The heavy-code clippy line (`--features heavy-tests`) added to `.claude/agents/reviewer.md`'s checks; closes that KNOWN_GAPS row.
- README and CHANGELOG say what M1 actually does.

Out:
- Fixing anything the checkpoint finds.

## Signatures
Expose: `scripts/m1-checkpoint.sh <git-git|criss-cross|foxtrot|octopus|list>` — exit non-zero with a message on a missing prerequisite, never a silent skip.
Call: `scripts/clone-git-git.sh`; `cargo run -p graft-fixtures`; `cargo run -- <path>`.

## Acceptance
- [ ] Every TESTING.md M1 row has a pass condition and a result slot.
- [ ] Each checkpoint scenario verified to open the graph in the builder's own terminal — say which.
- [ ] KNOWN_GAPS.md has no M1-owned row undecided.
- [ ] Gates clean.
- [ ] **The checkpoint question stays unanswered until the user answers it.**

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
