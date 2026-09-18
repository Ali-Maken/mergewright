# m1-the-graph/12 — The uncommitted-changes row and 16-colour chrome

Status: split — graft-git read done (40163c3); remainder in 12b (wiring + row) and 12c (16-colour chrome)
Build with: **Sonnet** (medium) — a read, a virtual row, a separator rule
Blocked by: 10
Leaves runnable: yes — a dirty working tree shows as a row above HEAD.

Why: "what have I not committed" is a daily question, and at 16 colours the panes currently melt into one background.

## Read first
- PROMPT.md §1.3 (uncommitted changes paragraph only)
- docs/design/01-view-mockups.md §1.1 (the mockup's top row) and docs/design/04-terminal-capability-detection.md §4.6
- KNOWN_GAPS.md M0 row "Dark chrome flattens at 16 colours"
- Code: `crates/graft-ui/src/theme/` (depth mapping), any existing status read in `crates/graft-git/src/` (grep `status`), 10's row widget

## Scope
In:
- A virtual row above HEAD's row, on HEAD's lane, with staged / unstaged / untracked counts. Absent when the tree is clean.
- Counts read off the UI thread, refreshed whenever the app re-reads history or refs. No file watcher.
- At `ColorDepth::Palette16`, pane separation comes from a rule or box-drawing edge, not a background step. Closes the M0 gap.

Out:
- Staging anything (M3). Lane identity at 16 colours (13).

## Acceptance
- [ ] Fixture tests: clean (no row), staged only, unstaged only, untracked only, all three.
- [ ] Unborn branch with staged files: decide where the row attaches, snapshot it.
- [ ] Snapshot at 16 colours shows the panes visibly separated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (handoff, first attempt)

This session already compacted once before this note was written (the
builder resumed from a compaction summary). The scope turned out bigger
than one context: reading the app-layer wiring (`update.rs`, `jobs/mod.rs`,
`event_loop/mod.rs`, `view.rs`, `history/mod.rs`) plus `graft-ui` rendering
plus tests/snapshots/pty-capture does not fit alongside the `graft-git`
read. Stopping here at a clean, tested seam rather than risk a second
compaction mid-edit.

**Done and committed:** `graft_git::status` — reads staged/unstaged/
untracked counts off `gix::Repository::status()`.
- `crates/graft-git/src/status.rs`: `WorkingTreeCounts{staged,unstaged,
  untracked}` (+ `is_clean()`), `StatusError`, `pub fn counts(handle:
  &RepoHandle) -> Result<WorkingTreeCounts, StatusError>`. Bare repo →
  `default()`. Unborn HEAD handled by gix's own empty-tree fallback (tested).
  Ignored files excluded (gix default). Rename tracking off by default, so
  `Item::IndexWorktree(Rewrite)` is unreached but counted as staged with a
  comment explaining why, kept exhaustive on purpose.
- `crates/graft-git/src/status/tests.rs`: 8 fixture tests — clean, staged
  only, unstaged only, untracked only, all three, unborn branch + staged
  (counts as staged), bare repo (clean), ignored file (not counted).
- `crates/graft-git/Cargo.toml`: added gix feature `status`, comment updated
  (was explicitly listed "still off"; now justified for this ticket).
- `crates/graft-git/src/lib.rs`: `pub mod status;`.
- `crates/graft-git/MAP.md`: new `## status` section.
- Gates green workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings`, `cargo test` (workspace, fast tier) — all pass.

**Not started:** the UI/app half. Exact wiring, found by a scout this
session (do not re-derive, use directly):

1. `crates/graft/src/update.rs`:
   - `Task::ReadWorkingTree` (new variant, alongside `ReadRefs`).
   - `Product::WorkingTree(Result<graft_git::status::WorkingTreeCounts, String>)`.
   - `Command::ReadWorkingTree`.
   - `AppState.working_tree: Option<graft_git::status::WorkingTreeCounts>`.
   - `update()`: add `Task::ReadWorkingTree` to the non-blocking `Started`
     match arm (same line as `OpenRepository | CheckSystemGit | ReadRefs`);
     add `Product::WorkingTree(Ok(counts)) => next.working_tree = Some(counts)`
     and an `Err` arm mirroring `Product::Refs(Err(_))` (status line, not
     silently dropped).
   - `is_refresh` (Ctrl+R block, ~line 383-391): push `Command::ReadWorkingTree`
     alongside `OpenRepository`/`ReadHistory`/`ReadRefs`. Same list is
     duplicated in a test (`control_r_asks_for_the_repository_to_be_re_read`,
     ~line 699) — update it too.

2. `crates/graft/src/jobs/mod.rs`: add a `Command::ReadWorkingTree` arm next
   to `Command::ReadRefs` (~line 496), same shape, no `AheadBehindCache`
   needed:
   ```rust
   Command::ReadWorkingTree => {
       let repos = Arc::clone(&self.repos);
       self.announce(Task::ReadWorkingTree, move |cancel| {
           if cancel.is_cancelled() { return Outcome::Cancelled; }
           Outcome::Produced(Product::WorkingTree(working_tree::read(&repos)))
       });
   }
   ```
   Add a small `crates/graft/src/working_tree.rs` (or a fn in `refs`'
   sibling style) with `pub fn read(store: &RepoStore) -> Result<WorkingTreeCounts, String>`
   mirroring `refs::read`'s first two lines (`store.shared()` →
   `repo.handle()` → `graft_git::status::counts(&handle).map_err(|e| e.to_string())`).

3. `crates/graft/src/event_loop/mod.rs`, `opening_repository` (~line 212):
   add `jobs.dispatch(Command::ReadWorkingTree);` alongside `ReadRefs`. A
   test at ~line 638 asserts the exact command list — update it too.

4. `crates/graft/src/view.rs`, `graph_view`/`row_entry` (~line 64-175): the
   hard part. `GraphLine.row` (graph-ui, `graph_column/mod.rs`) is
   `&'a Row` (graft-graph), non-optional — there is no rowless slot for a
   virtual line yet, even though `NodeKind::Uncommitted` already exists
   (glyph `'◌'`, `graph_column/mod.rs`). Two options, undecided:
   (a) synthesize a full `Row`/`Commit` for the virtual row (sentinel id —
   candidate: the null oid, so it never collides with `refs.head()`
   comparisons — same `lane`/`lanes` as the real HEAD row, `outgoing:
   vec![(lane, head_row.commit.id)]` so the connector line draws down into
   HEAD, `incoming: vec![]`), then override subject/author/when/sha in
   `GraphEntry` to be blank/summary-text only (author/when/sha columns stay
   blank per docs/design/01-view-mockups.md §1.1's mockup — only the glyph + a summary subject
   line render). This reuses all of `GraphColumn`'s existing connector-line
   math for free but fabricates a `Commit`.
   (b) add a separate `GraphView.uncommitted: Option<UncommittedRow>` field
   (counts + lane index) and draw it directly in `GraphScreen::body()`,
   bypassing `GraphColumn`'s full per-row geometry. Less invasive, matches
   the mockup's simplicity, but hand-draws lane pass-through lines instead
   of reusing `Geometry::of`.
   Recommendation: try (a) first — the `Commit` synthesis is probably a
   handful of fields (check `graft-core`'s `Commit`/`Identity` structs
   before writing it) and it buys correct lane-line continuity for free.
   Attachment rule (needed either way): show the row when `state.history`'s
   first visible row's commit id equals `refs.head()`, OR when
   `refs.head()` is `None` (unborn branch) — i.e. only when HEAD is the
   topmost visible row. Consumes one row of `state.graph_rows()`'s budget:
   `History::visible(height)` (`crates/graft/src/history/scroll.rs`) knows
   nothing about a virtual row, so request `visible(state.graph_rows() -
   1)` when the row will draw.

5. `crates/graft-ui/src/theme/mod.rs`: no `ColorDepth` accessor on `Theme`
   yet — needed so `GraphScreen`/`draw_row` can tell when to apply the
   Palette16 separator. Add `Theme::color_depth() -> ColorDepth` (store it
   in `Theme::new`), or thread `ColorDepth`/`Capabilities` into
   `GraphScreen::new` separately.

6. `crates/graft-ui/src/graph_screen/mod.rs`, `draw_row` (uses
   `Token::SurfaceRaised` background fill for `RowPaint::Highlighted`): at
   `ColorDepth::Palette16`, replace or supplement that fill with a rule/
   box-drawing edge (e.g. a left-edge `│` in `Token::Accent`) — this closes
   the KNOWN_GAPS.md M0 "Dark chrome flattens at 16 colours" row.

7. Tests/acceptance still open: fixture tests for clean/staged/unstaged/
   untracked/all-three at the `graft`-crate level (state → view, not just
   `graft_git::status::counts` — already covered in graft-git); unborn +
   staged with the attachment decision, snapshotted; a 16-colour snapshot
   showing panes visibly separated. Plus the mandatory pty capture under
   `docs/shots/` per CLAUDE.md (reuse `docs/shots/11-ref-badges.txt`'s
   approach) — not yet done, no visual result exists yet to capture.

Next builder: start from `crates/graft/src/refs/mod.rs` as the template
(job/cache/pure-view split) — the working-tree read is simpler (no cache
needed). Read `crates/graft-core`'s `Commit`/`Identity` types before
deciding (a) vs (b) above.
