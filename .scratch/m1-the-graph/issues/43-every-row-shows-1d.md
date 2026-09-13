# m1-the-graph/43 — Every row shows "1d": verify dates against git log

Status: ready
Build with: **Sonnet** (medium) — verify, then fix if wrong
Blocked by: M1 graph design pass (user order: design decisions land first)
Leaves runnable: yes — the date column matches git.

Finding (user gallery walk, 2026-09-13, git/git, screenshot `docs/shots/user-37-git-git-pixel-after-scroll.png`): "Every row shows '1d' — verify against git log."

## Read first
- PROMPT.md §1.3 (what each row shows: which date, relative format)
- `crates/graft-ui/MAP.md`, `crates/graft-git/MAP.md`; scouts for source

## Scope
In:
- Compare, for the top ~50 rows of `~/.cache/graft/git-git.git` and one small fixture, the date Graft shows against `git log --format='%h %ad %cd'` (author and committer). Record the table (short) in the ticket notes.
- If Graft is wrong (wrong field, wrong "now", clock or timezone bug, rounding): test red, fix the cause, green.
- If Graft is right (e.g. those merges really are a day old): say so, and check that older rows further down show older dates.
Out: changing the date format (that's §1.3, ask first).

## Acceptance
- [ ] Comparison recorded; verdict stated.
- [ ] Wrong → test red → green; right → evidence from deeper rows.
- [ ] pty capture under `docs/shots/43-*` showing the date column; user judges by eye.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens.
