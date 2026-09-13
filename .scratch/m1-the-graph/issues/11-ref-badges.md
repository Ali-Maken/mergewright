# m1-the-graph/11 — Ref badges and ahead/behind

Status: ready
Build with: **Opus** (high) — git reads with edge cases (detached, unborn, diverged) off the UI thread
Blocked by: 09
Leaves runnable: yes — branches, remotes, tags, HEAD and stashes show on their commits.

Why: badges are what answer "what is going on here".

## Read first
- PROMPT.md §1.3 (ref badges paragraph only)
- DESIGN.md §1.1 (badges in the wide mockup) and §8.2 (grep `Branches`)
- KNOWN_GAPS.md M0 row "`ui.icons` has no consumer"
- Code: `crates/graft-git/src/branches/`, 10's row widget, the M0 job runner's public API

## Scope
In:
- Badges for local branches, remote branches, tags, `HEAD` and stashes. In sync → one merged badge (`main ⇄ origin`); otherwise `↑2 ↓1`.
- Ahead/behind computed off the UI thread, cached per (local tip, upstream tip), recomputed when either moves.
- `ui.icons` consumed: badges and node glyphs honour it. Closes the M0 gap.
- Badges truncate before the subject disappears at 80 columns.
- Add a corpus shape only if a badge case below has none (e.g. a stash).

Out:
- 50k-ref scale (30). "Contained in" (M2).

## Acceptance
- [ ] One test each: in sync, ahead only, behind only, diverged, detached HEAD, tag-only commit, stash, unborn branch.
- [ ] A branch name containing an escape sequence renders inert.
- [ ] `ui.icons = false` changes the rendering — snapshot pair.
- [ ] Ahead/behind never runs on the UI thread (asserted through the job runner).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
