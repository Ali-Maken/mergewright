# Mergewright: Design — Graph layout algorithm

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Keymap](02-keymap.md) · Next: [Terminal-capability detection](04-terminal-capability-detection.md)

---

## 3. Graph layout algorithm

Lives entirely in the `graph` crate: a pure library with no terminal code
(PROMPT.md §1.2), taking a commit stream in and producing rows with lane
assignments and edges out. Three renderers (pixel/unicode/ascii, §4) consume
its output; none of them feed back into it.

### 3.1 Data model

```
Commit         { id, parents: [CommitId; ≤N], author, committer, date, message }
BranchIdentity { stable hash of a branch name — the source of lane colour,
                 independent of which column the branch currently occupies }
Lane           { index: usize,                 // the column
                 identity: BranchIdentity,      // for colour; fixed until the lane reaches another branch's tip (ADR-0064)
                 waiting_for: CommitId,          // the next commit that continues this lane
                 reserved: bool }                // true for lane 0 and integration-branch lanes:
                                                  // never handed to the free pool
Row            { commit: Commit,
                 lane: usize,                    // this commit's own column
                 incoming: [usize],              // lanes that converge on this row (closing edges)
                 outgoing: [(usize, CommitId)],   // lanes leaving this row, one per parent
                 marker: Option<MistakeRadarKind> }
LayoutCursor    { active: [Option<Lane>], waiting: Map<CommitId,[usize]>,
                  free_pool: [usize], rows_emitted: usize }
                                                  // lane state only: an opaque, resumable snapshot
                                                  // of everything below. ~1.6 KiB whatever the
                                                  // depth — this is what a checkpoint is (§3.6)
EmittedOrder    { ids: [CommitId], row_of: Map<CommitId, usize> }
                                                  // the commit on every row so far; one copy,
                                                  // shared by the frontier and every checkpoint.
                                                  // The one O(history) structure: handles, not
                                                  // Oids, measured 9-12 B/commit (m1-the-graph/56c)
walker_state                                      // §3.2 step 2's walk frontier, held by the
                                                  // streaming layout beside the cursor — never by
                                                  // a checkpoint. With a commit-graph: the
                                                  // generation-ordered queue + frontier in-degrees,
                                                  // bounded by graph parallelism (ADR-0008); after
                                                  // promotion: an index into EmittedOrder. Dense
                                                  // by handle, since the graph is read, not the odb
```

### 3.2 Algorithm overview (prose)

**1. Resolve the default branch and any integration branches**, per
ADR-0001's ordered precedence. This runs once per repo, off-thread, and is
cached. The default branch's tip seeds lane 0; each integration branch's tip
seeds its own reserved lane adjacent to lane 0. Both kinds of lane are marked
`reserved`, so the general allocator in step 3 never hands them to another
branch even when they're temporarily idle.

**2. Walk commits in topological order** (date order and first-parent-only
are alternate walk strategies selected by the same interface, per PROMPT.md
§1.2). "Topological" means: a commit is only emitted once every one of its
children that's part of the visible walk has already been emitted. Multiple
commits can be ready to emit at once (parallel branches); break the tie by
commit date (most recent first) and, if dates tie exactly, by commit id, so
the order is fully deterministic.

A naive in-degree topological sort has to see the whole visible range
before it can emit a single commit — first paint would wait on a full
history walk. So which walk runs depends on the repo's **commit-graph
file** (ADR-0008):

- **Commit-graph present** — the common case: `git gc` and
  `fetch.writeCommitGraph` both write one, and any commit-graph written by
  git ≥2.38 (Mergewright's minimum) carries generation numbers, which gix
  exposes via `gix-commitgraph`. Use git's own incremental topo-sort:
  explore commits through a queue ordered by generation number,
  descending. A child's generation always exceeds its parent's, so once
  every commit still awaiting exploration sits at or below a commit's
  generation, nothing unexplored can turn out to be its child — its
  in-degree is final and it can be emitted. Emission starts after
  exploring a small frontier, not the whole range; `walker_state` is that
  queue plus the frontier's in-degree map, bounded by the graph's
  parallelism rather than history size. It lives beside the `LayoutCursor`
  in the streaming layout and is never part of a checkpoint (§3.6): a
  rebuild re-reads emitted commits, it never re-walks. This path is what
  meets the 150 ms first paint.
- **No commit-graph:** first paint uses **commit-date order** — the
  rev-walk's natural streaming order, already a first-class walk strategy,
  through the same `layout_chunk` machinery — and still lands inside
  150 ms. Two things then happen off-thread: a full topo-order walk of the
  visible range runs as a background job, and Mergewright offers — **once per
  repo**, remembered if declined — to run `git commit-graph write
  --reachable` in the background so every later open takes the fast path.
  When the background topo walk finishes, the view **promotes** to strict
  topo order in one step: layout re-runs from the topo walk, rows are
  swapped behind the next frame, and the selection and scroll anchor are
  re-found by commit id, never by row index. The promoted layout keeps the
  topo walk's output as a compact commit-id list (`EmittedOrder`, §3.1)
  and its `walker_state` is an index into that list: rows are rebuilt by
  re-reading the commits at those indices, never by walking again. So a
  post-promotion checkpoint is the same lane-state snapshot as one on the
  commit-graph path — measured at 1,578 bytes at 10k rows and 1,694 at
  40k, identical on both paths — O(1) in remaining history, not
  O(remaining history) (m1-06; this was KNOWN_GAPS R2-1). Until
  promotion, a one-line status notice says the order is provisional. The
  150 ms budget survives,
  with its meaning stated honestly: without a commit-graph it buys a
  date-order first paint, and strict topo order arrives at promotion.

**3. For each commit, as it's emitted, assign it a lane:**

- Look up every lane whose `waiting_for` is this commit's id (`waiting` map).
  There's normally exactly one; there's more than one exactly where two or
  more branches share this commit as an ancestor before diverging further
  back — a fork point, read backwards. There's zero for a new root reached by
  a fresh tip that hasn't merged into anything seen yet (open a new lane).
- The commit's own lane is the lowest-indexed lane among those found, with
  one override: if the default branch is among them, its lane (0) always
  wins, keeping lane 0 straight for its entire life as PROMPT.md §1.2
  requires. Every other lane found here **closes into** this row — drawn as
  a converging edge, then not carried further forward under its own column.
- Assign outgoing edges, one per parent:
  - The **first parent** continues in the commit's own lane. This is what
    keeps a first-parent chain in one column for its whole life.
  - Each **additional parent** (a merge) gets a lane: reuse the
    lowest-indexed column in `free_pool`, or open a new column (current
    max + 1) if the pool is empty. Never a `reserved` lane.
  - Update `waiting_for` on each of these lanes to the corresponding parent
    id (or close the lane immediately if the parent was already emitted
    earlier in the walk — happens when a lane's target turns out to be an
    already-visible commit, e.g. two merges into the same ancestor).
- Any lane with no more work (a root commit, or a parent already satisfied)
  is freed — but placed in a **pending** free list, not `free_pool` itself.

**4. End of row: promote pending frees into `free_pool`.** This is what makes
lane reuse never happen on the same row that frees it (PROMPT.md §1.2): a
lane vacated while processing row N only becomes available starting row
N+1's allocation step.

**5. Compaction.** If the number of concurrently open lanes exceeds
`graph.max_lanes` (unset: 8 lanes under 100 columns, 10 at 100–159, 12 at
160+, ADR-0022), fold columns into a rail (§3.5) rather than growing the
graph wider.

### 3.3 Pseudocode

```
fn resolve_lanes(default_tip, integration_tips, tips_to_show) -> initial state:
    active[0] = Lane { identity: default_branch_identity, waiting_for: default_tip, reserved: true }
    for (i, tip) in integration_tips.enumerate():
        active[1 + i] = Lane { identity: hash(tip.branch_name), waiting_for: tip, reserved: true }
    for tip in tips_to_show where tip not in (default_tip, integration_tips):
        push_to_heap(tip)   # opened lazily, on first emission, in step below
    waiting = reverse_index(active)   # commit id -> [lane indices]
    return LayoutCursor { active, waiting, free_pool: [], rows_emitted: 0 }

fn layout_chunk(cursor, walker, chunk_size) -> [Row]:
    rows = []
    pending_free = []
    for _ in 0..chunk_size:
        commit = walker.next()                 # per §3.2 step 2's walk strategy; None when exhausted
        if commit is None: break

        incoming = waiting.remove(commit.id) or []
        lane = min(incoming) if incoming else open_new_lane(commit)
        if default_branch_lane in incoming:
            lane = 0                             # lane 0 always wins

        for l in incoming where l != lane:
            pending_free.push(l)                 # these edges converge into `lane` this row

        outgoing = []
        first, rest = commit.parents[0], commit.parents[1..]
        active[lane].waiting_for = first
        outgoing.push((lane, first))
        if first is None:
            pending_free.push(lane)              # root commit: lane ends here

        for parent in rest:                       # merge parents
            l = pop_lowest(free_pool) or allocate_new_column()
            active[l] = Lane { identity: branch_identity_for(commit, parent), waiting_for: parent, reserved: false }
            waiting[parent].push(l)
            outgoing.push((l, parent))

        marker = mistake_radar.classify(commit, lane, incoming, outgoing)  # see PROMPT.md §1.4

        rows.push(Row { commit, lane, incoming, outgoing, marker })
        cursor.rows_emitted += 1

    free_pool.extend(pending_free)                 # promote frees — not usable until the *next* chunk
    return rows
```

Streaming and virtualization build on this directly (§3.6): `layout_chunk` is
the unit of incremental work, and a `LayoutCursor` is the thing checkpointed
and resumed.

### 3.4 Merge commits and branch-name recovery

**Ease pass (ADR-0063; user approved 2026-10-17)** — lines are coloured by **role** relative to the cursor row's branch, not by branch identity: `┃` this branch, `│` built on, `║` trunk, `┊` grey other; the role moves with `j/k`. The second-parent curve takes the joining branch's role. A deleted branch's label also recovers `Merged PR <n>` as `PR <n>` (Q13 a). Supersedes the identity-colour rule below.

- The **second-parent curve** is drawn in the merged branch's `identity`
  colour (from the lane opened for that parent in step 3), not the primary
  lane's colour, so a merge visually reads as "this colour flowed into that
  one."
- **Labelling a deleted branch:** the merge commit's own message is the only
  surviving source once the branch ref is gone, so the label is recovered by
  matching, in order: `Merge branch '<name>'`, `Merge branch '<name>' into
  <target>`, `Merge pull request #<n> from <name>`, `Merge remote-tracking
  branch '<name>'`. If none match, the label falls back to the merged
  parent's short SHA and the row is not treated as a labelling failure —
  it's just less specific.
- **Evil-merge, accidental-pull-merge, backward-merge, foxtrot-merge and
  force-pushed-remote** detection all run as a classification step over the
  `Row` a commit already produced (comparing `incoming`/`outgoing` lanes,
  parent order, and — for evil merges — a lazy `--remerge-diff` computed only
  once a merge is selected, per PROMPT.md §1.4). They annotate `Row.marker`;
  they never change lane assignment. Duplicate-commit detection (patch-id
  matching) runs as a separate, bounded, off-thread pass over the visible
  window and attaches cross-references rather than a marker on layout itself.

### 3.5 Compaction (the fold rail)

**Ease pass (ADR-0063; user approved 2026-10-17; tickets 06b, 06u)** — a branch gets a lane only while it has a commit in view (main mock :90), and the header's `+N lanes` becomes `lanes drawn: N of M`, shown only while something is not drawn.

**Which columns can be drawn.** Before ranking, `Fold::plan` sorts the columns open in the window into three kinds:
- a **node column** holds a commit of the window;
- a **connector** holds none, but carries an edge whose two ends are both commits in the window (merging main into a feature branch, a criss-cross, a merge of an ancestor);
- a **pass-through** is any other open column: every edge it carries leads out of view.

Pass-throughs are dropped at any cap, whatever their priority: their edges draw as stubs, they never sit behind the rail and never count as folded. Lane 0 is kept while open, whatever its kind, so trunk scrolling out of view does not shift every lane left. A column open in no row of the window, lane 0 included, is dropped.

**The cap is a budget, admitted by priority, not a set of vetoes** (ADR-0025). `Fold::plan` draws at most `max_lanes` columns, rail included, admitting in this order until the budget is spent:
1. lane 0 (always fits: the cap is ≥ 2);
2. node columns — the selected commit's own column, reserved integration lanes, the selection's ancestry thread's columns, then the rest;
3. connectors, in the same order.

Within a class the least recently touched goes first. A node column past the budget **folds**: its node draws on the rail on its row in place of `⋮`, its edges as stubs beside it (ADR-0025's residual, built in 06b). A connector past the budget, or one whose end commit sits on the rail, is **dropped** like a pass-through, never folded: the rail marks hidden commits only. `graph.max_lanes` is clamped to what the screen can hold at its width (unset: 8/10/12 across the §1 breakpoints, ADR-0022). Expanded (`+`) plans with no budget: nothing folds, and pass-throughs are still not drawn.

**An edge into a folded or dropped column is a stub, not a curve to a marker** (ADR-0022): the gap
cell right of a kept node draws `▾` (an outgoing edge — the folded branch
continues below) or `▴` (incoming — it forked here), in the folded branch's
identity colour; several folded edges on one row share the first edge's
colour. Nothing crosses a kept lane to reach the fold, and nothing ends in
open space. A folded thread is still legible by its stub colour, not lost.

**The marker column is a rail (`⋮`), not a counter.** It appears only while a node column folds. The header counts at the top line's right end, after every other tail item: `lanes drawn: N of M` while N < M (N = real columns drawn, rail not counted; M = columns open in the window, pass-throughs included), then `K merged branches folded` while K > 0 (K = closed fold marks on rows in view), joined ` · `. They are the first tail items to give way, right phrase first, each whole and never cut with `…`, and they give way before the name of the branch `HEAD` is on. `+` expands and collapses every fold at once; a click on the rail to expand one row is not built (ADR-0022).

This is a rendering-time decision, not a layout-time one: the underlying
`Row` data keeps every lane's real column, so expanding `+` is instant and
never re-runs layout.

### 3.6 Incremental, streaming and virtualized layout

- **Streaming:** `StreamingLayout::advance(budget, stop)` is the unit of
  incremental work. It pulls commits from a `CommitSource` (§8.2) at most
  one chunk (`chunk_rows`, default 500) at a time, lays them out at the
  frontier, and stops at `budget` rows, at the source's end, or when `stop`
  says so — checked before every pull. The caller sizes the first call to
  hit the 150 ms first-paint budget — strict topo order when a commit-graph
  file exists, date order until promotion when it doesn't (§3.2 step 2,
  ADR-0008); later calls run in the background between frames. A source
  that returns nothing without finishing is a `Stalled` error, not a spin.
- **Row cache with eviction — never an append-only buffer.** Laid-out rows
  live in a chunked cache inside `graph`'s streaming layout, one entry per
  chunk, keyed by chunk index. Protected at any moment: the chunks covering
  `[nearest checkpoint at or before viewport_top − buffer, viewport_bottom
  + buffer]`; everything else is evictable, oldest-touched first, once the
  cache passes its budget (`resident_chunks`, default 16) — and only then;
  eviction is a memory decision, not a tidy-up. Scrolling back into an
  evicted range rebuilds those chunks by re-reading their commits from the
  retained order and replaying from the nearest earlier checkpoint. Layout
  is deterministic (§3.7), so a rebuilt chunk is identical to the one
  evicted — and that is checked, not assumed: every eviction leaves a
  fingerprint (row count + hash) behind, and a rebuild that does not
  reproduce it is a `RebuildDiffers` error, never a display. This
  eviction, not virtualization alone, is what makes memory grow with the
  viewport rather than history (PROMPT.md §1.2). Measured (m1-06, an
  in-memory 100k-commit history, default configuration): scrolling all
  100k rows held at most 16 chunks = 8,000 rows at any point; checkpoints
  now thin to an even grid rather than evict LRU (m1-the-graph/60), so a
  far jump at 1M replays ≤ 16,000 rows (13–15 ms), not the whole history;
  the emitted order (`EmittedOrder`, §3.1) measures 9-12 B/commit
  (handles, not `Oid`s, m1-the-graph/56c) and is the one term that grows
  with history — at 1M without a commit-graph, resident pack pages are
  now the larger term (m1-the-graph/56c). The heavy tier's peak-memory
  test (PROMPT.md §7.1:
  load and scroll the 100k-commit fixture, fail above 300 MB by
  `getrusage`) is the gate that proves it on a real repository — including
  after a jump to the bottom and a scroll back up (m1-16).
- **Rendering virtualization:** only rows in `[viewport_top − buffer,
  viewport_bottom + buffer]` are rasterized/drawn. Lane occupancy is
  stateful, so the walk must still pass *through* everything above the
  viewport — but passing through means advancing the `LayoutCursor` and
  letting the rows be evicted, not retaining them.
- **Pass-through chunks (m1-the-graph/52b, ADR-0026 item 4):** `advance`
  reads commits only for chunks meeting `[viewport_top − buffer,
  viewport_bottom + buffer]` (plus the rest of a chunk already held at the
  frontier). Every other chunk *passes through*: the cursor lays its links
  out into `RowShape`s, the order and checkpoints are recorded, and a
  fingerprint of the shapes (row count + hash; merge lane identities hash
  their handles) is kept — no commit object is read. Eviction keeps the same
  fingerprint and drops the rows. `rows(range)` over a chunk not held
  replays links from the nearest checkpoint, checks every replayed chunk's
  shapes against its fingerprint (`RebuildDiffers` otherwise), and reads
  commits only for the chunks asked for — each run of adjacent ones as one
  batch, plus one read per merge lane open at the run's first row that the
  run does not contain.
- **Checkpoints** make deep jumps (search hit at row 900,000) cheap: every
  `checkpoint_every` rows (default 2,000) the streaming layout keeps a
  clone of the `LayoutCursor` — lane state only, about 1.6 KiB whatever
  the depth (§3.1), on the commit-graph path and the promoted path alike.
  A checkpoint holds no walker state: a rebuild resumes the cursor at the
  checkpoint and feeds it the commits the retained order says come next,
  re-read from the repository, so nothing is ever walked twice. A jump
  into evicted rows replays from the nearest earlier checkpoint rather
  than row 0 (measured: a jump to row 15,000 of a 20k-row layout replayed
  300 rows from the checkpoint at 14,800 and pulled nothing from the
  walk). No checkpoint is taken at the very end of history — nothing would
  resume from it. Checkpoints are pure functions of the commit sequence up
  to that point, so they're cache-only (bounded at `max_checkpoints`,
  default 64: past the cap the stride doubles and every checkpoint off the
  new, sparser grid is dropped, so spacing stays even across all of
  history instead of clumping near wherever was visited last — at 1M rows
  16,000 apart, m1-the-graph/60) and always rebuildable — nothing about
  them needs to be durable.
- **A cold deep jump is a background job.** `G` on a freshly opened repo,
  or a search hit far past the frontier, still needs the walk to advance
  that far before the target row can exist. That is `advance` called in
  budgets as a cancellable job through the standard machinery (§8.3): the
  UI thread never blocks, the status area shows progress ("laying out…
  412k/1.2M commits" — `rows_emitted` against `expected_rows`, known once
  the walk is done or the order is promoted), input — including cancelling
  the jump — keeps working, and the viewport stays put until the job lands
  and the view jumps to real rows. The checkpoints laid down along the way
  survive the rows' eviction, so the next deep jump in either direction is
  cheap.
- **Promotion (§3.2 step 2)** is a second streaming layout built over the
  finished topo walk's id list (`from_order`) and advanced until the
  anchored commit has a row again (`reanchor`: that commit's new row, less
  the offset it had on screen); then it replaces the provisional layout
  behind the next frame, and the rows below the anchor fill in as it
  catches up without moving it. Its source never walks — it only re-reads
  by id — and its checkpoints are the size they were before promotion.
- **Merged branches fold by default** (PROMPT.md §1.3, ADR-0063). The walker
  in step 2 emits the first-parent closure of the keep tips, of every tip
  nothing points at, and of each opened merge's second and later parents; a
  merged branch's commits stay behind their merge until `space` opens it (an
  opened set, laid out again from the first row it changes). A merge emitted
  before the walk knows its second parent is shown is repaired late: layout
  resumes from that merge's row. **First-parent view** is the same machinery
  with another *inclusion predicate*, "only follow `parents[0]`," which keeps
  everything in lane 0. Under every inclusion `Row.commit` is git's commit
  with every real parent; `Row.drawn_parents` is the view's trimmed set, and
  only drawing reads it.

### 3.7 Invariants → tests

Each invariant in PROMPT.md §1.2 maps to a specific property the pseudocode
above is designed to satisfy, so the M1 property tests assert directly
against it:

| Invariant | Where it's enforced |
|---|---|
| Every parent edge is drawn | Every element of `commit.parents` produces exactly one `outgoing` entry |
| No edge passes through a node it isn't connected to | Only lanes in a row's `incoming`/`outgoing` bend toward that row; every other open lane draws a straight pass-through segment |
| Deterministic output | Heap tie-break by date then commit id; no unordered iteration in lane allocation; and checked at run time — a rebuilt chunk must match the fingerprint its evicted copy left (§3.6) |
| A lane never changes column mid-life | A lane's column is fixed at open time; only `waiting_for` mutates until the lane closes |

### 3.8 Strip cache (pixel tier) and the 16 ms frame budget

PROMPT.md §1.1 fixes the cache keys ("row range, layout hash, theme and
cell size"); this fills in the mechanism that makes 60 fps on 1M+ commits
and input-to-frame under 16 ms (PROMPT.md §6.3) achievable in the pixel
tier.

- **What's cached:** rendered strips — `RasterFrame`s covering fixed-height
  bands of rows, aligned to multiples of the strip height (start at 32
  rows; tune in M1's performance pass). Alignment is what makes a scrolled
  viewport re-hit existing strips instead of missing on every new offset.
- **Where it lives:** inside `graph`'s pixel renderer, as renderer-private
  state (§8.4 — an internal seam, invisible to callers). Three LRUs
  (backdrops, and for iTerm2 emphasis images and composites; Kitty's
  emphasis pieces are not cached in Mergewright, see the key below) under one **byte budget**
  (`ByteBudget`, 96 MiB, `event_loop/graphics.rs` `STRIP_BUDGET`), evicting
  the least recently used entry of any layer until the cache weighs no more
  than it (m8 04). Counted strips were not a bound: a strip weighs lanes ×
  cell, ~4.3 MB at 17×37 on MONOREPO and ~8.6 MB at 24×52, so 16 of each
  layer was 108–216 MB. **The viewport's floor wins over the budget:**
  each layer keeps at least `capacity_for(strip, viewport, buffer)` entries
  whatever they weigh, so a huge cell or a tall pane can hold the cache over
  its budget (`StripCache::minimum_wins` says when; at 24×52 it did on half
  the frames, 113 MB). Strips are drawn into buffers lent by a `RasterPool`
  that eviction refills, and Kitty's transmit keeps its straight-alpha,
  deflate and base64 buffers between uploads, so a held key reuses memory
  instead of churning malloc's large blocks. **Every pooled buffer is one
  size, the widest strip since the last flush**, so any spare serves any
  draw; an emphasis image cropped to a third of its strip still holds (and
  is weighed at) the full strip's buffer. The cost: an entry weighs a full
  strip, so 96 MiB holds about 8 backdrops + 8 emphasis images at 17×37 on
  MONOREPO (5.6 MB each), and at a 24×52 cell the floor (5 + 5 × 11.2 MB)
  is over budget. Chosen over buffers sized to need (m8 04 review, Major 2):
  tried, crops of many widths leave no spare that fits, and the fast test's
  hold made 17–31 new buffers in 115 `j`s where one size makes none — the
  churn the pool exists to stop. Revisit with a quiet measurement of both;
  uploads are remembered by
  `UploadKey`, not by holding the frame. `term`'s encoder may additionally reuse
  already-transmitted image ids where the protocol allows (Kitty); that's
  an encoder-internal optimisation, not part of this cache.
- **The key:** `(strip row range, layout hash, theme id, cell size)`.

  Since the M7 scroll fix (m7-power-tools/41–47) the frame's backdrop alpha
  joins the key: every strip in a frame is drawn at the lowest alpha any of
  its rows needs, so a cursor move that keeps it leaves every strip as it
  was. The selection's paint (lit thread, dimmed lanes, cursor) is not in
  the strip: it goes above it as **emphasis pieces** of three kinds — a
  **run** per lane column (its straight edges), an **arc** per column and
  row (the arcs drawn there), a **node** per lane column (nodes, stubs,
  tails of rows on that lane; the rail goes with the marker column's node)
  — each cropped to the whole cells its pixels touch, in both axes
  (ADR-0053, amendment 2026-10-10, amended after continuation 1). A piece
  is **keyed on its content**: kind, cell size, backdrop alpha, and each
  element's geometry relative to the piece's anchor, its colour and alpha;
  no strip key, no absolute position. So one picture anywhere on screen is
  one upload, placed as often as it appears (placement ids `p=1..=n`), and
  a `j` or a fold that leaves a piece's pixels alone re-places it by id;
  only pieces whose key the terminal lacks are rasterised and sent, and
  their pixels are dropped once sent. **Stacking is explicit:** pieces
  stack as the whole pass draws (runs and arcs in its edge order, then
  nodes), and each place carries that rank (`Place::stack`, sent as Kitty
  `z = 1 + rank`), so the screen order never depends on the order commands
  are written or on image ids. Pieces the
  frame no longer places stay in the terminal up to 64 MiB of decoded
  pixels (`KittyImages`, least recently placed freed first), so moving back
  sends nothing. iTerm2 cannot stack images: it still gets one composited
  band per strip, its emphasis cached on the strip key plus the selection's
  paint (the only emphasis LRU left). On a graph wider than the lane cap the fold follows the
  selection (ADR-0025), so a `j` that moves the thread can still re-send
  every strip on screen: that is a layout change, not paint. Kitty uploads
  go deflated (`o=z`) when the terminal answers the probe for it; without
  deflate the held-key budget is not met (known gap
  M7/46-kitty-no-deflate).
- **The layout hash** is computed by the renderer from the layout-relevant
  content of the strip's rows — per row: commit id, `lane`, `incoming`,
  `outgoing`, `marker` — plus the set of pass-through lanes crossing the
  strip (index and `BranchIdentity`, since identity drives colour). No
  field is added to `Row`; the hash is derived from rows the renderer is
  consuming anyway. Layout is deterministic (§3.7), so a chunk evicted and
  rebuilt (§3.6) hashes identically — row-cache eviction never costs strip
  cache hits.
- **Invalidation** is mostly implicit: a refresh after a commit, fetch or
  ref move changes row content, so the layout hash changes and the stale
  strip simply stops being hit (and ages out of the LRU). Explicit
  wholesale flushes on: theme change, cell-size change or capability
  re-detection (§4.5), and terminal resize (strip pixel width changes).
  **Scrolling is not an invalidation** — that is the entire point.
- **Why this hits the budgets:** a scroll frame changes only the row range,
  so it blits resident strips and renders at most one new strip entering
  at the viewport edge; the strip height is chosen so one strip render
  fits well inside 16 ms. The common frame rasterizes nothing at all —
  lookup and placement only — which is what sustains 60 fps regardless of
  history depth. Validated against the heavy tier's 100k-commit fixture in
  M1's performance pass, alongside §3.6's memory gate.

The Unicode and ASCII tiers don't need this: they emit styled text cells,
ratatui already diffs frames cell-by-cell, and there's no rasterization
cost to amortize.

---
