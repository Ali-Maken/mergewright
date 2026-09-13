# m1-the-graph/12b — The uncommitted-changes row, wired and drawn

Status: done
Build with: **Sonnet** (medium) — wiring a finished read into the app, one virtual row
Blocked by: 12 (partial: `graft_git::status::counts` done, commit 40163c3)
Leaves runnable: yes — a dirty working tree shows as a row above HEAD.

Split from 12 after it compacted. 12's "Notes (handoff, first attempt)" section is the spec for this ticket: use its scout findings directly, don't re-derive them.

## Read first
- Ticket 12: Scope, Acceptance, and Notes steps **1–4 and 7** only
- PROMPT.md §1.3 (uncommitted changes paragraph only)
- DESIGN.md §1.1 (the mockup's top row only)
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-core/MAP.md` before any source
- Code: `crates/graft/src/refs/mod.rs` (template), then only the lines 12's notes name

## Scope
In:
- 12 notes step 1–3: `Task::ReadWorkingTree`, `Command::ReadWorkingTree`, `Product::WorkingTree(Result<WorkingTreeCounts, String>)`, `AppState.working_tree`, dispatched on open and on Ctrl+R; errors reach the status line.
- 12 notes step 4: the virtual row above HEAD on HEAD's lane with staged / unstaged / untracked counts; absent when clean. Choose (a) or (b) after reading `graft-core`'s `Commit`. If (a), the sentinel commit must never reach anything but drawing (no ref comparisons, no selection target, no search). Say which you chose and why in the ticket.
- Attachment rule, including unborn branch; check it against DESIGN.md §1.1 and record the decision in the ticket.
Out:
- 16-colour separators (12c). Staging (M3). Lane identity at 16 colours (13).

## Signatures
- `crates/graft/src/working_tree.rs`: `pub fn read(store: &RepoStore) -> Result<graft_git::status::WorkingTreeCounts, String>`
- Uses `graft_git::status::counts(&RepoHandle) -> Result<WorkingTreeCounts, StatusError>` (exists)

## Acceptance
- [x] `graft`-level tests (state → view): clean (no row), staged only, unstaged only, untracked only, all three.
- [x] Unborn branch with staged files: attachment decided, snapshotted.
- [x] Ctrl+R and open-repository command-list tests updated.
- [x] MAP.md updated for every changed public item.
- [x] pty capture under `docs/shots/12b-uncommitted-row.txt` (reuse 11's pyte approach) on a dirty fixture; report says the user judges it by eye.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; before a second compaction stop at a clean seam with a handoff note.

## Notes (built)

**(a) vs (b): chose (a), synthesize a full sentinel `Row`/`Commit`.**
Ticket 12's notes recommended (a) because it reuses `GraphColumn`'s existing
connector-line geometry math instead of teaching the widget a second, parallel
code path for a rowless "extra line." Built a zero-`Oid` sentinel `Commit`
(`Oid::from_bytes(&[0; 20])`, never collides with a real object id) wrapped in
a `Row` with no parents and HEAD's lane, so the graph column draws it exactly
like any root commit. The sentinel never leaves rendering: no ref lookups, no
selection target, no search match against it — `graft-ui`'s `GraphLine` now
holds `Cow<'a, Row>` instead of `&'a Row` so a synthesized row can sit beside
borrowed real ones under the same lifetime, which is the one public-API
change this made to `graft-ui` (see its MAP.md).

**Attachment rule.** Show the row when `history`'s topmost visible commit id
equals `refs.head()`, or when `refs.head()` is `None` (unborn branch) — this
matches DESIGN.md §1.1's mockup, which always draws it at the very top with
no scroll condition attached. Added one case the ticket notes didn't spell
out: while `refs` itself hasn't loaded yet (`None`, i.e. right after opening,
before the refs job answers), the row is *not* shown. Showing it and then
possibly retracting it once refs arrive would flicker on every startup; a
clean first frame is the safer default. Flagging this as an implementation
default, not a spec change — happy to revisit if the user wants the row to
appear optimistically before refs resolve.

**Verified:** `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo test --workspace` all green (no regressions). 8 new
`graft`-level state→view tests cover clean/staged/unstaged/untracked/all
three, unborn-branch-with-staged, and viewport-budget interaction. Visually
confirmed by hand with a pty capture (pyte, reusing 11's approach) against
three throwaway fixture repos: a dirty two-commit repo (staged + unstaged +
untracked), an unborn branch with staged + untracked files, and a clean
one-commit repo (row correctly absent) — `docs/shots/12b-uncommitted-row.txt`.
User judges the render by eye.
