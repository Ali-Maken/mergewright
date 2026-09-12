# m1-the-graph/09 — The row: ref badges, columns, and the uncommitted-changes row

Status: ready
Build with: Opus
Blocked by: 04

Why: the graph column alone does not answer "what is going on here" — the badges do. PROMPT.md §1.3 names this precisely, and it is where repo-derived text crosses into `ui`, so it is also where the sanitiser boundary is enforced for real.

## Read first
- PROMPT.md §1.3 (columns, ref badges, uncommitted changes, merge labelling)
- DESIGN.md §1.1–§1.3 (the three widths and what each drops)
- ARCHITECTURE.md: `ui` renders from a plain view model `app` assembles; repo text crosses only as `term::SanitizedText`
- KNOWN_GAPS.md M0 rows: "`ui.icons` has no consumer", "Dark chrome flattens at 16 colours"

## Goal
Every graph row carries its badges, subject, author, date and short SHA, and degrades column by column as the terminal narrows.

## Scope
In:
- Columns per §1.3: graph · marker slot · ref badges · subject · author initials chip · relative date · short SHA, with lower-priority columns hiding at the DESIGN.md §1.2/§1.3 widths.
- Ref badges for local branches, remote branches, tags, `HEAD` and stashes. In sync → one merged badge (`main ⇄ origin`); otherwise ahead/behind (`↑2 ↓1`).
- Author initials chip, coloured per author, respecting `.mailmap`.
- The uncommitted-changes virtual row at the top, attached to HEAD, with staged/unstaged/untracked counts.
- The marker slot exists and is empty in M1 (Mistake Radar is M2) — reserve the column so M2 does not reflow the layout.
- `ui.icons` finally has a consumer: badges and node glyphs honour it (closes an M0 gap).
- **At `ColorDepth::Palette16`, chrome separation must come from a rule or box-drawing edge, not a background step** — M0 measured that `bg`/`surface`/`surface-raised` all collapse to index 0 (closes an M0 gap).
- Everything repo-derived arrives as `SanitizedText`; a ref name or subject with an escape in it renders inert (M0 already has the mechanism — use it, prove it).

Out:
- Scrolling, selection, hover (10).
- Containing-branch data ("Contained in: …") — that is the M2 detail pane.
- Diff, file tree, detail pane (M2).

## Acceptance
- [ ] Snapshots at 80×24, 120×40, 200×60, dark and light, showing exactly which columns survive each width.
- [ ] Badge cases covered: in sync, ahead only, behind only, diverged, detached HEAD, tag-only commit, stash, unborn branch.
- [ ] A branch name and a commit subject containing an escape sequence render inert (extend M0's test, don't re-invent it).
- [ ] `ui.icons = false` changes the rendering, proven by a snapshot pair.
- [ ] At 16 colours, panes are still visually separated — show it in a snapshot.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
