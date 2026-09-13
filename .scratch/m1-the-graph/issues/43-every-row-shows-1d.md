# m1-the-graph/43 — Every row shows "1d": verify dates against git log

Status: ready
Build with: **Sonnet** (medium) — verify, then fix if wrong
Blocked by: 45
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
- [x] Comparison recorded; verdict stated.
- [x] Wrong → test red → green; right → evidence from deeper rows.
- [x] pty capture under `docs/shots/43-*` showing the date column; user judges by eye.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens.

## Notes (verdict: correct, not a bug)

Graft's date column uses `Commit::date()` = the **committer** timestamp
(`crates/graft-core/src/commit.rs`), matching `git log`'s default (`%cd`) and
`--date-order`. `relative_date` (`crates/graft-ui/src/graph_screen/columns.rs`)
floor-divides elapsed seconds into `m`/`h`/`d`/`mo`/`y`; "now" comes from
`SystemTime::now()` (`crates/graft/src/view.rs::now_seconds`). No wrong field,
no "now" bug, no timezone bug.

**Comparison** — `git --git-dir=~/.cache/graft/git-git.git log -1 --format='%h author=%ad committer=%cd' --date=iso-strict <sha>`, "now" = 2026-09-13 20:41:39 UTC:

| sha | committer date | elapsed | Graft shows | git log agrees |
|---|---|---|---|---|
| 848ec2e | 2026-09-11 22:36:54 UTC | ~46h | `1d` | yes |
| a184f46 | 2026-09-11 22:34:16 UTC | ~46h | `1d` | yes |
| 37cba83 | 2026-09-11 22:34:04 UTC | ~46h | `1d` | yes |
| c5f1475 | 2026-09-11 22:34:00 UTC | ~46h | `1d` | yes |

**Root cause of the "every row 1d" look:** git/git's `seen`/`next`/`jch`
integration branches are dozens of topic-branch merges Junio does in one
sitting — real commits genuinely made minutes apart, ~46h before the walk.
`elapsed/86400` floors anything in `[24h, 48h)` to `1d`, so ~40 consecutive
rows land in the same bin. That's a real, dense burst in git/git's own
history, not a Graft defect.

**Deeper rows do vary** (`G` to jump to the repo's oldest commit,
`docs/shots/43-git-git-oldest.txt`): 21y at git/git's root commit
(`e83c516`, 2005). Mid-repo and `graft-self` rows show `1m`/`2m`/`9m`/`1h`/`2h`
(`docs/shots/43-graft-self-varied.txt`) — the column is not stuck.

**Gallery fixtures** (`trivial`, `skewed-dates`, …) also show one uniform
unit per stop (`6y`) — same cause: `graft-fixtures` gives each shape a tight
commit-time window (correct for testing graph *shape*, not dates). This
makes the date column visually inert while walking the gallery, which could
read as "broken" the way the user's finding did.

**Decision needed from the user** (not made here — a fixture/gallery design
choice, not a bug fix): should `graft-fixtures` spread a shape's commit
times further apart (e.g. days/months) so the gallery's date column
demonstrates its own range? Recommend yes for the `real repos` group's
framing text if so, but this is a new ticket, not this one — out of this
ticket's scope (date *format* is explicitly out; this would be fixture
*data*, arguably the same kind of "ask first" change).

**Captures** (never cat'd; pyte captures of `cargo run -p graft --example
gallery`, 80x24, unicode tier):
- `docs/shots/43-git-git-top-cluster.txt` — top of git/git, the `1d` cluster.
- `docs/shots/43-git-git-oldest.txt` — after `G`, git/git's root commit, `21y`.
- `docs/shots/43-graft-self-varied.txt` — this checkout, showing `1m`…`2h`.

No code change. No test added: nothing was wrong to pin down with a red
test — the acceptance criteria's "wrong → test" branch doesn't apply.
