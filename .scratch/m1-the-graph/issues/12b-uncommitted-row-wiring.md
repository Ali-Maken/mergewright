# m1-the-graph/12b — The uncommitted-changes row, wired and drawn

Status: ready
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
- [ ] `graft`-level tests (state → view): clean (no row), staged only, unstaged only, untracked only, all three.
- [ ] Unborn branch with staged files: attachment decided, snapshotted.
- [ ] Ctrl+R and open-repository command-list tests updated.
- [ ] MAP.md updated for every changed public item.
- [ ] pty capture under `docs/shots/12b-uncommitted-row.txt` (reuse 11's pyte approach) on a dirty fixture; report says the user judges it by eye.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; before a second compaction stop at a clean seam with a handoff note.
