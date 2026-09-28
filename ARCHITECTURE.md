# Graft: Architecture

A one-page map of the crates: what each one owns, what it may depend on, and
where to start reading. Read this before adding a module or moving code
between crates — and update it in the same ticket if that ticket changes the
crate map. The full reasoning behind every choice here is
[`DESIGN.md` §8](DESIGN.md#8-architecture); this file is the short version.

"Graft" is a codename (see `CLAUDE.md`). Domain vocabulary — lane, default
branch, rendering tier, Ghost Preview, Mistake Radar, marker, ref badge,
operation, undo journal, Command Log, job — is fixed in `CONTEXT.md`; this
file uses those terms rather than redefining them.

## The shape

A pure core, a thin shell (PROMPT.md §6.1). The parts that *decide* — graph
layout, diff/porcelain parsing, Mistake Radar, undo planning, `update` — are
plain functions: data in, data out, no terminal, no filesystem, no clock, no
threads. Only the outer layer touches any of those. Data flows one way:

```
Event ──▶ update(state) ──▶ (new state, [Command]) ──▶ view(state)
```

`update` never does I/O; it returns `Command`s. The job runner executes each
`Command` off the UI thread as a cancellable `Job` and reports the result back
as an `Event`. The UI thread only ever waits on the next `Event` — never on a
`Job` — which is why input stays responsive during a two-minute fetch.

## Where to start reading

- **Changing how history is drawn** (lanes, compaction, a new rendering tier)
  → `graph`. Start at `DESIGN.md` §3 for the algorithm.
- **Changing what Graft reads from a repo** (status, log, diff parsing) →
  `git`. Reads are `gix`-primary; `git2` backs single named gaps only
  (ADR-0011), never a second full adapter.
- **Changing what a write operation does, or how undo works** → `ops`. Start
  at `DESIGN.md` §6, then `graft-ops/src/operation.rs` (one enum, four
  properties: `check`, `plan`, `touches`, and `undo::inverse`). The
  execution half is `apply.rs` (the §6.2 protocol: read, check, back up,
  journal, run, journal again), `snapshot.rs` (both trees from one copy of
  the index — never the real one), `restore.rs` with `restore/outcome.rs` (the inverse's plan loop, with
  `restore/steps.rs` for the one `git` per step and `restore/rollback.rs` for
  putting back what a cancelled undo had already done) and
  `undo/execute.rs` (the §6.7 drift rail in front of them). **The seam
  rule: `ops` decides, `app` wires.** `app` never builds a `GitInvocation` for
  a write, never touches the journal file, never chooses a snapshot; a key
  becomes an `Operation` and `ops` does the rest. Guarded by
  `crates/graft/tests/it/seam_rule.rs`: `GitInvocation` under `graft/src/`
  only inside `jobs/` (reads); the journal's `render_*` and the inverse's
  `Step::`/`PathScope::` constructors nowhere in `app`.
- **Changing fetch, pull or push** → `graft/src/update/remote.rs` for the
  `f`/`p` keys and the facts; `update/remote/push_box.rs` for `P` and its
  box, `after_push.rs` for `u` on pushed work, `in_flight.rs` for `Esc`,
  progress and the status-line words;
  `graft/src/jobs/remote.rs` for the remote facts read; `ops::operation` for
  what actually runs.
- **Changing terminal detection or an image protocol** → `term`. Start at
  `DESIGN.md` §4, then `graft-term/src/detect/`: `environment` reads the env
  vars, `probe` writes the queries, `reply` parses what comes back, `tmux`
  owns the passthrough round, and `mod` is the decision that turns all of it
  into a `Capabilities`. `detect::tty` writes the queries to the real
  terminal and reads replies through the input mux, so everything above it
  is tested against scripted replies.
- **Changing what `y` sends to the clipboard** → `graft-term/src/clipboard.rs`
  builds the OSC 52 request; `graft/src/event_loop` writes it through the
  session's `Transport` (ADR-0023).
- **Changing how keys, mouse or terminal replies are read** →
  `graft-term/src/input/`. `mux` is the one thread that owns stdin for the
  whole run; `decode` is the pure bytes → events/replies decoder, where every
  key encoding lives. A reply goes to every open `ReplyWaiter`; anything else
  is an event queued in arrival order. `app`'s `event_loop::input` only
  forwards those events into the loop.
- **Changing what counts as hostile repository text** (a new escape or
  invisible-character class) → `graft-term/src/sanitize/`. One function and
  a small set of entry points; `DESIGN.md` §8.2 sets the minimum classes and
  the replacement policy, and the module doc is the authority on what is
  actually neutralised (it takes all of Unicode's `Cf` category, which is a
  superset).
- **Changing a screen or a keybinding** → `ui` for the widget, `app` for the
  key-to-`Command` mapping. Start at `DESIGN.md` §1–§2.
- **Changing the detail pane, a diff view or the Explain panel** →
  `ui::detail_pane` for the widget (header, file list, unified and
  side-by-side diff bodies, the Explain panel's own render), `app::view::detail`
  for its view model, `app::update::detail` for state, keys and mouse hit
  areas. A diff itself is read and parsed in `git::diff`
  (m2-details-and-diffs).
- **Changing blame or the places trail** → `ui::blame` for the widget,
  `app::update::place` (with `place/{answer,blame_state,from,keys,rows,trail,write}.rs`)
  for the trail, its keys, mouse and job answers,
  `app::view::frame::overlays::draw_place` for drawing it over the graph,
  `app::jobs::detail` for the blame job (m7-power-tools/20).
- **Changing the history place** (a line's or a file's history, from blame's
  `L` or the pane's `F`) → `graft-ui::history` for the picture (pure draw, no
  state), `app::update::place` with `place/history.rs` (state),
  `place/history/open.rs` (`open_history`, `f_key`), `place/history/view.rs`
  (view, title, row words), `place/history_keys.rs` (its keys and mouse) and
  `place/history_answer.rs` (trace and whole-file answers);
  `app::jobs::detail` (`jobs/detail/history.rs`) for the trace job
  (working-tree lines mapped to `HEAD`, the enclosing function, else the
  block) and the whole-file read (m7-power-tools/21).
- **Changing how a switch launches** → `app::update::switch`: the one
  function every switch goes through — detached target → preview; changed
  tracked files → preview with the set-aside line; otherwise at once
  (m7-power-tools/10b).
- **Changing the Map view or the overview strip** → `ui::map_screen` /
  `ui::overview_strip` for the widgets, `app::map` for the per-branch bucket
  walk, `app::view::map` / `app::view::strip` for their view models,
  `app::update::map` for the Map view's keys (m2-details-and-diffs/18–20).
- **Changing `u`/`U`, the journal browser or the stash overlay** →
  `app::update::undo` for the keys and the drift box, `app::update::journal` /
  `app::update::stash` for the overlays' state and keys, `app::jobs::undo` for
  the journal read and the call into `ops::undo`, `app::jobs::stash` for the
  stash list, `app::view::journal` / `app::view::stash` for their view models,
  `ui::journal_screen` / `ui::stash_screen` for the widgets
  (m3-working-tree-undo-and-stash/06). `app::jobs::undo` is the second module
  in `app` that opens the journal — through `ops`, never by hand, like
  `app::jobs::operate`.
- **Changing the credential prompt** → `app::update::credential` for the
  state, the modal keys and the `Answer` that carries the secret,
  `app::jobs::askpass` for the waiting job's `AskPass`, `app::cli` for the
  binary's askpass mode (`GRAFT_ASKPASS_SOCKET`), `app::view::credential` for
  the view model and `ui::credential_prompt` for the widget. The protocol
  itself — the socket, the `Secret`, the helper — is `git::run::askpass`, and
  nothing in `app` speaks it by hand (m4-remotes/05–06).
- **Changing what the Mistake Radar detects, or one of its background jobs**
  → `ops::mistake_radar` for detection itself (`rules`, `probes`, `explain`);
  `app::jobs::radar` for the jobs that run off the UI thread (`RadarContext`,
  the force-push and duplicate-commit probes); `app::update::radar` /
  `app::view::radar` for the state that holds their answers and the per-row
  lookup a frame reads (m2-details-and-diffs/13–17).
- **Changing put-in (`a` into an older commit) or absorb** →
  `app::update::amend_into` (`update/amend_into.rs`) for the key and preview,
  `app::jobs::rewrite` (`jobs/rewrite.rs`) for the job, `app::update::row_tags`
  for the `changed` tag it leaves on rows (m7-power-tools/09).
- **Changing how branch config is journaled** (upstream set by Track, undone
  by `u`) → `graft-ops/src/branch_config.rs` for the reads and writes,
  `Step::SetBranchConfig` in `graft-ops/src/undo.rs` for the inverse.
- **Changing pull's fetch-first path** → `graft-ops/src/apply/pull.rs`.
- **Adding a new kind of `Command` or wiring a new job** → `app`. It's the
  only crate allowed to see every other crate, and `update` and the job
  runner both live there. **Three commands are the exception: they are
  carried out by the frame loop itself, never by a job** — see the hand-over
  rule below.
- **Running another program in Graft's terminal** (`$EDITOR`, a `SIGTSTP`
  suspend) → `app::event_loop::outside`. See the hand-over rule below.
- **Anything that needs the open repository from a job** → `app/src/repos.rs`.
  It owns the one open repository for the run; see the seam below before
  opening one yourself.

## Handing the terminal over

`Ctrl+E`, `e` and `Ctrl+Z` run another program — the user's editor, or the
shell they suspended to — inside the terminal Graft is drawing on. Their
three commands (`EditCommitMessage`, `OpenExternalEditor`, `Suspend`) are the
only ones **carried out by the frame loop rather than queued as jobs**, and
that is not a shortcut: the job pool's threads do not own the session or the
input mux, and only the thread that owns both can give the terminal up. A job
that tried would leave two readers on one file descriptor and the user's
keystrokes split between Graft and their editor.

The rule is `run_outside_terminal(session, mux, f)` in
`app::event_loop::outside`:

1. `InputMux::pause` first, and refuse the whole hand-over if it will not
   stop — a reader still on the terminal is worse than no editor.
2. `TerminalSession::suspend`: every mode Graft turned on, back off, in the
   reverse order, alt screen last so the editor draws over the shell.
3. Run `f`.
4. Re-enter through a guard, so no path out of the function can skip it:
   modes back first, then the reader, then a full redraw — the editor has
   scribbled over every cell Graft thought it knew.

The loop answers with `Product::ExternalEditor` / `Product::HandedOver`, so
`update` sees the outcome as it sees any other product and stays pure.

**The one git spawn this path needs** — `git var GIT_EDITOR` — goes through
`git::editor::git_editor`, not a bare `Command::new("git")`, so it gets the
fixed environment, the output cap and the stop policy every other git spawn
gets. It lives in `git` rather than `app` because `seam_rule.rs` allows a
`GitInvocation` under `app/src` only inside `jobs/`, and this one has to run
on the frame thread.

## The crates

Every package is named with a `graft-` prefix — `graft-core`, `graft-graph`,
`graft-git`, `graft-ops`, `graft-term`, `graft-ui` — because a bare package
called `core` collides with Rust's built-in `core`. `app` is the binary, and
its package is simply `graft`. This page uses the short names throughout; the
prefix exists only in `Cargo.toml` and in `use` paths (`graft_core::…`).

`app` is a library (`crates/graft/src/lib.rs`) with a thin binary
(`main.rs`) on top of it. Everything lives in the library so that `app`'s own
tests, its integration tests and the manual `terminal_probe` example all drive
the same code the binary runs; `main.rs` only starts it. This is a target
split inside one package, not a new crate: the dependency table below is
unchanged.

| Crate | Owns | May depend on |
|---|---|---|
| `core` | Plain data only — no behaviour, no algorithms: commit and ref identity, events, the record shapes other crates read and write, errors for its own parsing | nothing internal |
| `graph` | The pure layout engine and the pixel/Unicode/ASCII renderers | `core` |
| `git` | Repository reads (`gix`; `git2` only per named gap, ADR-0011), the typed git command runner, output and diff parsers | `core` |
| `ops` | Operations, the undo journal, Mistake Radar detection, Ghost Preview planning | `git`, `core` |
| `term` | Terminal capability detection, image-protocol encoding, escape-sequence sanitising | `core` |
| `ui` | Widgets, screen layout, theme, view models | `graph`, `term`, `core` |
| `app` | The binary: event loop, `AppState`, `update`, the job runner, config, wiring | everything |

As layers — each row may depend only on row(s) above it:

```
core
graph   git   term        (each: + core)
        ops                (+ git, core)
ui                          (+ graph, term, core)
app                          (+ everything above)
```

`graph`, `git` and `term` are siblings and may not depend on each other. `ui`
never imports `git` or `ops`: every screen renders from a plain view model
`app` assembles, not from a live repository handle. Repo-derived strings
(commit messages, author names, file names, file contents, ref names) cross
into `ui` only as `term::SanitizedText` — a newtype only `term::sanitize`
can construct, applied by `app` at the view-model boundary (`DESIGN.md`
§8.2), so escape-sequence injection is a compile error, not a review catch.

The same one-way rule holds *inside* `app`, between its own `update` and
`view` modules, with one test-only exception: production `update` code never
imports `view` — the diagram above is `Event ─▶ update ─▶ view`, not the
other way round — but `update`'s own tests may read `view`, when the
alternative is a second, hand-kept copy of `view`'s geometry that nothing
checks against the real one (`update/tests.rs`'s
`the_geometry_mirror_matches_the_rects_the_frame_hands_out`, which sweeps
`update::geometry::detail_split` against `view::frame::detail_rects` itself,
made `pub(crate)` for exactly this). Production code crosses neither
direction, whichever module it sits in.

### The open repository: `SharedRepo` and `RepoStore`

Opening a repository is expensive — discovery walks up the tree, then the
whole config stack is parsed — and every job needs one. Three constraints
meet here:

- `gix::Repository` is not `Send`, so a job thread cannot be handed one;
- `AppState` is cloned on every event and must stay pure, so the state
  cannot hold one either;
- opening once per job is what M0 did, and it is the wrong answer.

The seam is two types:

- `git::repo::SharedRepo` — the `Send + Sync` half: the parsed config and the
  object database. `handle()` turns it into a thread-local
  `RepoHandle` for the cost of a clone of some `Arc`s, and sets the object
  cache. This is why `gix`'s `parallel` feature is on
  (`graft-git/Cargo.toml`): without it `gix-features` uses `Rc`, and
  `SharedRepo` is not `Send`.
- `app::repos::RepoStore` — one per run, owned by the job runner and never by
  the state. It opens the repository the first time a job asks, keeps it, and
  caches the answer to ADR-0001's "which branch owns lane 0", whose last rule
  counts merges across the whole of history. `Ctrl+R` drops the cached
  conclusion and keeps the open repository.

A job therefore takes `Arc<RepoStore>`, calls `shared()?.handle()`, and reads.
`RepoStore::opens()` exists so a test can assert the count stays at one.

### The layout and the walk: `graph::CommitSource`

`graph` and `git` are siblings, and the streaming layout
(`graph::layout::stream`) needs commits from a repository twice over: the
next chunk at the frontier, and a re-read of commits it already emitted when
it rebuilds an evicted chunk (`DESIGN.md` §3.6). The seam is one trait,
`graph::CommitSource` — `next(limit, stop)` and `read(ids)` — and `git`
offers exactly what an implementation needs: `history::Walk::next_chunk`
(plus `Walk::full_topological`, ADR-0008's background job) and
`history::read_commit`. `app` owns the product implementation,
`app::history::WalkSource`, beside the seed builder (`history::seeds_for`) and
the driver that hands each laid-out chunk to the UI thread
(`history::stream`, m1-07). The test one in
`graft-graph/tests/it/streaming.rs` sits on that crate's dev-dependency edge
to `git`, which ADR-0006's rule does not count.

Inside `graph`, `layout.rs` is the cursor's own file — its state and public,
checkpoint-facing API (`new`, `extend`, `rows_emitted`, `active_lanes`) — and
`layout/` holds what streams it and its internals: `assign_row` (deciding one
row's lane, edges and columns — the algorithm itself), `order` (the emitted
commit ids — the one structure that grows with history), `source` (the trait
above), `cache` (chunked rows, eviction, fingerprints), `checkpoint` (cursor
snapshots) and `stream` (`StreamingLayout`, the one type callers see).

### `fixtures`: test-only, outside the table

`graft-fixtures` (`crates/graft-fixtures/`) is the whole test harness, for
all three tiers ([`TESTING.md`](TESTING.md)):

- the named fixture corpus — one small repository per history or ref shape
  (PROMPT.md §7.2) — handed to a test as a path in two lines;
- behind the `heavy-tests` feature, the synthetic repository generator with a
  size parameter, which is where PROMPT.md §7.1's 100,000-commit fixture comes
  from;
- the peak-memory measurement the 300 MB gate will assert on (ADR-0015), and
  the case count that makes `proptest` run 64 times in the fast tier and 1,000
  in the heavy one.

It is **not** a row in the table above, because nothing in the shipped binary
may reach it: it is `publish = false` and appears only in
`[dev-dependencies]`. The `cargo metadata` edge test deliberately ignores
dev-dependency edges (ADR-0006), so the enforced edge set is unchanged by it.
`graft-term` depends on it that way for the case count, `graft-git` and `app`
for the corpus, and each re-exports the feature as its own `heavy-tests` so
one `--features heavy-tests` turns the tier on across the workspace.

It depends on no other Graft crate, in either direction. That is the rule
that keeps it usable: the corpus can be built before the crate that reads it
exists, and a change to `core` or `git` can never make the fixtures fail to
build. It shells out to real `git` and owns nothing a production code path
would want.

### `capabilities`: repo metadata tooling, outside the table

`graft-capabilities` (`crates/graft-capabilities/`) is the feature database:
it parses `docs/capabilities/*.md`, checks every citation in it (a test
name, a capture path, an ADR or design note) against the repository on
disk, and generates `CAPABILITIES.md` and
`docs/generated/capabilities.html` from the same parsed data. Schema and
workflow are `docs/capabilities/README.md`.

It is **not** a row in the table above: nothing it produces ships in the
binary, and nothing in the binary reads it back. It has no `[dependencies]`
on another Graft crate and — unlike `graft-fixtures` — no other Graft crate
depends on it either, not even as a dev-dependency: it is not test
infrastructure, it is a standalone checker that happens to live in the
workspace so `cargo nextest run` picks up `tests/checker.rs` without a
separate script anyone has to remember to run. `crates/graft` and
`crates/graft-ops` in particular must never gain a dependency on it.

### The acceptance walks: `crates/graft/tests/it/`

Every integration test of the app crate is a module of one binary
(`tests/it/main.rs`, PROMPT.md §7.1). The milestone acceptance walks live
there too, and each has a dev-only example (`examples/m3_walk.rs`,
`examples/m4_walk.rs`, `examples/gallery.rs`) that includes its pure half by
`#[path]` and drives it from a command line. The modules they share are
split so that **an example compiles only what it runs**:

- `app_harness.rs` — the headless app: `App` drives `update` over the real
  job pool.
- `repo_truth.rs` — git as evidence: `RepoTruth`, `git_truth`, `diff_truth`,
  the isolated `run_git`, `fsck_clean`, `keys_label`. Every walk judges by
  this, never by `AppState`.
- `coverage.rs` — the coverage matrix (`docs/shots/35-coverage-matrix.txt`):
  the gallery's two tables plus one table per walk. The walks' capture stems
  are **constants here** (`M3_STEMS`, `M4_STEMS`), not read from their
  scenarios, so `gallery --matrix` needs no walk and the three `--matrix`
  commands write the same file through one `write_matrix`. Each walk's
  `captures_are_named_keyed_and_listed_in_the_matrix` test keeps its
  scenarios' stems equal to the list.
- `m3_walk.rs` — the M3 walk, one file (its precedent).
- `m4_walk/` — the M4 walk by seam: `fixture` (the remote and its work
  clone), `truth` (the bare's refs, journal, status lines and the remote's
  counters on top of `RepoTruth`), `steps` (`Step`, `Wait`, `Scenario`,
  `Capture`), `run` (the driver and `no_secret_leaked`), `vocabulary` (the
  step and assertion helpers the scenarios are written in), the scenarios by
  promise (`fetch_pull`, `push`, `credentials`, `hangs_and_undo`), and
  `every_scenario` (one `#[test]` per scenario, `#[cfg(test)]`). `mod.rs`
  is the list and the CLI helpers. The next walk (M5) copies this shape, not
  `m3_walk.rs`.

## Shared types worth knowing where they live

`core` holds plain data only — no behaviour, no algorithms. Laying out a
graph, parsing `git` output, detecting a Mistake Radar pattern, replaying a
journal entry, probing a terminal: all of that lives in the crate that owns
the behaviour, never in `core` (ADR-0007). A few types read naturally as
part of one crate's algorithm but, checked against the table above, can
only compile in `core` — because two crates that may not depend on each
other both need the same shape. The full list and reasoning is `DESIGN.md`
§8.2; the ones that come up most:

- `Commit`, `Oid`, `CommitId`, `RefName` — produced by `git`, consumed by
  `graph`, `ops` and `ui`.
- `MistakeRadarKind` — the finding kind `ops::mistake_radar` detects
  (`rules::classify_commit`/`classify_refs`, `probes`). **Not** attached to a
  `graph::Row`: `graph` never calls into `ops` and carries no radar data at
  all. `app::update::radar`/`app::view::radar` classify each row live,
  against a `RadarContext` — the default branch's first-parent ancestry as a
  `HandleSet` over its own `Arc<CommitTable>` (ADR-0026, housekeeping/02) —
  that a background job (`app::jobs::radar::run_radar_context`) builds once
  per refresh; `AppState.radar` caches the per-commit lookup so a frame's
  marker/detail-line read is O(1) per row, not a walk (M2 review C1). `ui`
  only ever sees the finished `Finding`s, as a marker or a detail line.
- `Capabilities`, `RenderingTier` — detected by `term`, consumed by `graph`'s
  renderer choice and `ui`'s diagnostics.
- `RasterFrame` — `graph`'s pixel renderer output; `term` encodes it into
  Kitty/iTerm2/Sixel bytes. Drawing and protocol-encoding are deliberately two
  different crates.
- `JournalEntry`, `OperationKind` — written by `ops::UndoJournal`, read by
  `app` to build the undo journal browser's view model for `ui`.

`Command` is the one exception: it is **not** a `core` type, even though it
carries `Event`s' name in PROMPT.md §6.1. A command like "run this fetch" has
to name a `git::GitInvocation`; `core` can't depend on `git`. `Command` lives
in `app`, the only crate that can see what it needs to name.

## Dependency rule and enforcement

Cargo rejects cycles, which stops the graph above from ever growing a loop.
It does **not** stop a new acyclic edge that breaks the intended layering —
`ui` adding `git` as a dependency compiles fine and violates the rule anyway.
A fast-tier test closes that gap: it parses `cargo metadata` and asserts the
edge set above matches exactly, so a stray dependency fails the fast tier
instead of waiting for review (`DESIGN.md` §8.5, ADR-0006).

## Keeping crates deep

Every crate here should be checkable against one question: can a caller get
everything they need through the handful of functions it exposes, without
reaching into its internals? `graph::layout` is the model case — one call
replaces lane assignment and incremental checkpointing; `graph::fold` beside
it is compaction as a lens over the rows a screen is about to draw, so
expanding `+N` never re-runs layout; `graph::inclusion` is first-parent
view, hidden stashes and folded merges as a `CommitSource` wrapper in front
of the same streaming layout, so a toggle lays out again only from the first
row it changes. Where a
crate needs more than one concrete implementation of something (`term`'s
per-protocol encoders, or a `git` read that a named `gix` gap forces onto
`git2` — ADR-0011), that split is an *internal* seam, private to the crate —
not a reason to widen its public interface. Default to `pub(crate)`; a type
becomes `pub` only when another crate in the table actually needs it.

## Elsewhere

- `DESIGN.md` — the full design: view mockups, keymap, the graph layout
  algorithm, terminal-capability detection, the undo journal, this
  architecture in more detail, and proposed deviations from PROMPT.md.
- `CONTEXT.md` — the glossary. Use its terms, not synonyms.
- `docs/adr/` — decision records, one per significant choice.
- `KNOWN_GAPS.md` — everything unfinished, stubbed or deferred.
