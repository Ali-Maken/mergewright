# graft — map (package `mergewright`, lib `graft`, bin `mwr`)
The application (ARCHITECTURE.md `app` layer), the only crate that may see every other one: event loop, `AppState`, the pure `update`, the job runner, config, and the view models `ui` draws.
A map of modules: one `##` heading per module to depth 2, path from the crate root; a purpose line; the items other top-level modules enter through. `main.rs` is the thin binary over `lib.rs`.
Tests: `tests/it` (one binary, with the pty harness); `examples/` holds the terminal probes, the gallery, the budgets and the milestone walks.
## cli
The command line: one optional path and the flags; askpass mode when git asks for a credential. Hand-parsed.
- `COMMAND` (const) — `Generation::CURRENT.command()` (`"mwr"`), the binary's name: usage, `--version`, stderr prefix, crash text, man page.
## cli::config_keys
Every config key as the man page and README describe it, paired both ways with the loader's own list.
## cli::flags
The one table of flags that `--help`, the man page and the README's flag list are read from.
## cli::man
`--generate-man`: the roff man page, written from the flag, key and config tables the program runs on.
## config
The config file under `${XDG_CONFIG_HOME:-~/.config}/<Generation::CURRENT.dir()>/` (`mergewright/`): `load` turns what is on disk into a typed `Config` plus warnings.
- `Config` (struct) — the typed file: `[ui]`, `[graph]`, `[diff]`, `[radar]`, `[keys]`, `[theme]`; read once at startup.
- `Loaded` (struct) — a load's result: the `Config`, its path and the warnings worth telling the user.
- `keymap_from` (fn) — `(&Loaded) -> (Keymap, Vec<String>)`: the keymap and every config problem as words.
## config::folder
The config folder's one-time move from each earlier generation's folder (`graft/`, `tributary/`) to the current one, newest first, under a lock, before the first load reads it; an older folder meeting the new one takes the byte rule.
## config::keys
`[keys]`: the suspend switch and the per-context remap sub-tables, read off the raw table.
## config::load
Reading a config file off disk into a `Loaded`.
## config::theme
`[theme.<kind>]`: colour overrides over a built-in theme, one token at a time.
## config::types
The typed `[ui]`/`[graph]` sections and their value enums.
## crash
The panic hook: restore the terminal first, then write a crash report to the state folder, then say where it went.
- `install` (fn) — installs the hook; called once by `main` before the terminal is touched.
## empty_state
What Graft does outside a repository: the start screen's keys and view model. Pure: a key in, an `OutsideStep` out.
- `OutsideState` (struct) — the start screen's state: recent list, prompts, palette.
- `OutsideStep` (enum) — what a key asks the loop to do: open a repository, start a clone, quit.
## empty_state::clone
The clone prompt (`c`): its state, the clone it asks for, the job events that drive it, keys and drawing.
## empty_state::open_prompt
The open prompt (`o`): typed path resolved against `~` and the shown folder, with `⇥` directory completion.
## empty_state::palette
The palette on the start screen: opening it, its keys, a paste into its query, drawing it over the list.
## event_loop
The loop that turns a terminal into an application: wake, `update`, draw. Every wake arrives down one channel.
- `Wake` (enum) — what woke the loop: input, a signal, a job's event, a redetect reply.
- `Exit` (enum) — why the loop stopped: quit, a handed-back key, a signal, a fatal error, a write stopped half-way.
## event_loop::exit_wait
After the terminal is given back: stop the job pool, wait a bounded time for an unstoppable write, say what happened.
## event_loop::graphics
The pixel tier's output: Kitty or iTerm2 images in the graph column, and the terminal's replies about them.
## event_loop::headless
Runs Graft to one finished frame with no terminal, when stdout is not a tty.
## event_loop::input
Carries what the terminal input mux decoded into the loop's wake channel; detects a hung-up terminal.
## event_loop::label
A short name per wake kind for the frames trace, never its payload.
## event_loop::outside
Handing the terminal to `$EDITOR`, a mergetool or `SIGTSTP`, and taking it back with the same modes.
## event_loop::pace
At most one unanswered frame in the pipe: each frame ends with a fence, the next waits for its answer.
## event_loop::places
Where Graft is: a repository or outside one. `locate`s the path, opens one `Terminal`, `drive`s place after place.
## event_loop::redetect
Re-asking the terminal while running: cell size on resize, background on focus, on a worker of its own.
## event_loop::rest
When the cursor has stopped: per-row work starts only for the row he stops on, not on every held `j`.
## event_loop::row_clear
A backend whose full-screen clear erases row by row, so a resize does not free the pixel tier's images.
## event_loop::running
The loop's entry points: `pump` and its test and pixel forms, idle prefetch, the paced turn, the free at exit.
## event_loop::signals
Leaving cleanly on `SIGTERM`, `SIGINT`, `SIGHUP`, `SIGTSTP`: the terminal is restored however Graft stops.
## event_loop::startup
Sets up the terminal and runs the interactive loop, or hands off to `headless` when stdout is not a terminal.
## first_launch
The first-launch line: for his first launches, until he opens the palette, the status row says where help is.
- `FirstLaunch` (struct) — whether the line is shown this launch, and the count it reads and writes.
## history
Reading history for the screen: `graft-git`'s walk meets `graft-graph`'s layout here, laid out as it arrives.
- `History` (struct) — the walk's history as the screen holds it: window, cursor, viewport, phase.
- `Window` (struct) — the laid-out rows the screen currently holds.
- `HistoryUpdate` (enum) — what the history worker reports back to `update`.
- `RowRequest` (struct) — the rows a move needs, sent to the worker.
- `Phase` (enum) — where the walk is: streaming, complete, and so on.
- `preview_seeds` (fn) — ghost lanes' colours from the refs, for Ghost Preview.
## history::scroll
Moving through history: cursor, viewport, and the `RowRequest` a move needs. Plain arithmetic on `History`.
## history::seeds
The lanes a layout starts with, from the repository's refs.
## history::serve
The history worker's loop: answers each `RowRequest`, lays out ahead between requests, parks when done.
## history::source
The product's `CommitSource`: `graft-git`'s walk behind `graft-graph`'s seam.
## history::state
`History` and the requests and updates that flow between it and the worker.
## history::stream
Walking a repository's history: opening it and working out the strict topological order it promotes to.
## history::views
The layouts one walk is shown through: opened on first ask, promoted to topological order, worked on when idle.
## jobs
The job runner: `update` returns `Command`s, this runs them on workers and sends `Event`s back. Decides nothing.
- `JobRunner` (struct) — the pool as callers see it: start, submit or dispatch work, cancel, shut down.
- `Dispatch` (trait) — the seam the loop hands commands through, so pool and loop test apart.
- `Delivery` (enum) — what became of a delivered event: received, or the loop already gone.
## jobs::askpass
A fetch, pull, push or clone's askpass: git's question goes to `update`, the answer comes back through the runner.
## jobs::clone
A clone as a pool job for the start screen's `c`, stoppable, with a fetch's progress line and credential prompt.
## jobs::commit_message
What the commit editor needs: `commit.template`'s contents and `HEAD`'s message for an amend.
## jobs::conflict
The resolver's jobs: open a conflicted path, take one side of one block. `graft_ops::conflict` decides.
## jobs::detail
The detail pane's reads: changed files, one file's diff, which refs contain a commit; view models built here.
## jobs::finish
The finish summary's reads: what the merge about to be committed changes, and one staged file's diff.
## jobs::fold_count
How many commits a folded merge brought in, behind its `+N`; kept for the session.
## jobs::forced
Which unstoppable writes a stop was forced on, and where each stop left git.
## jobs::journal_list
The reading half of `J`'s list: the shell's steps, what nothing holds any more, what `⏎` puts back.
## jobs::kinds
The work behind each `Command`: one function per job kind, each returning a closure a worker runs.
## jobs::map
The Map view's job: one `build_map` run off the loop's thread.
## jobs::operate
`Command::Operate`'s job: opens the undo journal, builds the ops context and calls `graft_ops::apply`.
## jobs::palette
The palette's finds: branch and path reads once per `⌃P`, and the per-keystroke name match.
## jobs::pool
The pool's bookkeeping: what is queued, what is running, and the worker threads between them.
## jobs::preview
The reading half of Ghost Preview: plans and lays out what an operation would do, off the loop.
## jobs::radar
The Mistake Radar's context: the default branch's first-parent chain and its previous reflog tips.
## jobs::remote
The remote facts behind `f`, `p` and `P`: configured remotes, `HEAD`'s branch and what it pulls from.
## jobs::rewrite
The reads behind `a` into an older commit and absorb: plan the rewrite, word its preview, find what it changed.
## jobs::rewrite_open
The read behind `R`: the rows and following branches the rewrite editor opens on.
## jobs::runner
`JobRunner`: the pool as callers see it.
## jobs::stash
The stash list read behind the `S` overlay.
## jobs::stash_only
Which commits only a stash entry reaches, behind the graph's "only in a stash" marks.
## jobs::switch_list
The branch picker's read, off the loop's thread.
## jobs::types
What work looks like, how it ends, and the `JobEvents` and `Dispatch` seams.
## jobs::undo
`u`, `U` and the journal read: opens the undo journal and hands it to `ops`, which plans the inverse.
## layout
The detail pane's saved split: right-pane width and below-strip height, remembered apart.
- `save_split` (fn) — writes the split to the app's own folder.
- `load_split` (fn) — reads it back at startup.
## map
The Map view's per-branch bucket counts: one bounded walk per local branch, built on toggle-on, dropped on toggle-off.
- `MapData` (struct) — the built rows `view::map` draws.
- `MapRequest` (struct) — what the view asks a build for.
## map::rows
Every row `build_map` draws: one per local branch, in the order the walk fixed.
## map::walk
One local branch's bounded walk into time buckets.
## recent
The repositories opened lately, for the start screen's RECENT list: one line each, newest first.
- `Recent` (struct) — the list as read, and the write after an open.
## refs
Which refs sit on which commit and how far each branch is from its upstream: everything a ref badge needs.
- `RefBadges` (struct) — refs per commit, `HEAD`, upstreams, stashes, tags.
- `RefLabel` (enum) — one ref before sanitising: detached `HEAD`, branch, remote, tag, stash entry.
- `badges` (fn) — the pure half of `read`: refs, `HEAD`, upstreams, stashes → `RefBadges`.
- `Upstream` (enum) — where a local branch stands against its upstream.
## rename
Where the app meets the one-time move to the new on-disk names: every repository is settled the moment it opens.
- `open_settled` (fn) — opens a path with its names settled first.
- `Settlement` (struct) — the move's outcome, kept for the store's life.
- `words` (fn) — the status line for what the move could not finish; `None` on a clean move.
## repos
The repository kept open for the run, lent to jobs one at a time: a handle is not `Send` and `AppState` must stay pure.
- `RepoStore` (struct) — the shared open handle, its common dir, workdir and app folder, and `refresh`.
- `StoreError` (enum) — why the store could not open or refresh.
## repository
What Graft reads from a repository at startup and keeps.
- `RepositoryFacts` (struct) — path, bare, `HEAD`, mailmap, commit-graph.
- `Head` (enum) — where `HEAD` points.
- `CommitGraphFact` (enum) — present, absent, shallow or unreadable.
## search
Search over history (`/`): the typed query, the pure test of one commit against it, and how a search job stands.
- `Query` (struct) — a case-insensitive literal needle; sha prefix only when all hex.
- `Search` (struct) — a running or finished search's state in `AppState`.
- `CommitHit` (struct) — one palette match: id and subject.
- `first_hits` (fn) — the palette's commit match over the same walk, stopping at a limit.
## session
What Graft settled before the first frame and does not change while it runs.
- `Session` (struct) — the settled values the loop and `update` read.
- `Startup` (struct) — what `main` works out with the terminal still the user's.
- `Clock` (enum) — `Wall` or `Fixed`: the one wall-clock read for anything a capture can show.
- `TerminalChange` (enum) — a resize or a redetected capability.
## terminal
The terminal session: every mode switched on at startup and off in reverse order however Graft stops.
- `TerminalSession` (struct) — the RAII guard; `suspend`/`resume` for `$EDITOR` and `SIGTSTP`.
- `TerminalRestorer` (struct) — restore handle usable from the panic hook.
## trace
The frame trace the binary writes when `MWR_PERF_TRACE` is set: one line per drawn frame saying where its time went.
- `Tracer` (struct) — the open trace file and its per-frame record.
- `TerminalOut` (enum) — the terminal's writer: plain `Stdout`, or metered when tracing.
## update
The pure `update(state, event) -> (state, [Command])`: the only place `AppState` changes; no clock, file, thread or drawing. Re-exports the state and event types.
## update::amend_into
`a` on an older commit and absorb from the `.` menu: put the staged changes into the commit they belong to, through the Ghost Preview and `y`.
## update::answer
`answer`: one arm per job answer, each handed straight to its module.
## update::answers
What `answer` hands a started job, a mergetool, the journal, a stash, an undo, a rewrite, the commit editor or an operation.
## update::banner
The operation banner: which operation git is part-way through, and the conflict interim's keys.
## update::clash
A stash put back that clashed: whether one is live, what landing on it does, and its words. Not a `Stop`: it has no git state.
## update::commit_editor
The commit editor's state and keys (`c`, `a`): message, template and amend, all pure.
## update::commit_graph
The commit-graph offer: whether to offer after a slow topological sort, and the `B`/`X` state machine.
## update::confirm
The confirmation box opened by `d` in the changes pane; it answers its own keys first.
## update::context_menu
The `.`/right-click popup: opening it, answering its keys, carrying out what it dispatches.
## update::credential
The credential prompt: git asks through the askpass helper, the user answers in one modal field.
## update::detail
The detail pane's state: what it shows, the jobs it waits on, and the caches that make stepping back free.
## update::dispatch
A key's way through the dispatchers, and the core events: resize, focus, job cancelled/failed/ended, rows the viewport asks for.
## update::drag
Dragging a branch badge onto a row: the three-event state machine and the menu the drop opens.
## update::edge
Where a pressed key meets the user's `[keys]` remaps, ahead of every dispatcher.
## update::events
What flows through `update`: events in, commands out, and the job-runner vocabulary between them.
## update::facts
Facts beside the graph arriving (system git, refs, working-tree counts), their read failures, and the name prompt's key claim.
## update::finish
The finish summary: `c` on a fully resolved merge shows what will land; `y` commits it.
## update::fold_count
A folded merge's `+N`: counted once by a job, kept for the session, never recounted.
## update::geometry
Pure screen geometry a mouse event needs: where the detail pane, its regions, the context menu and a ref badge land.
## update::history
The history walk's share of `update`: progress landing from the walk and the topological-sort promotion.
## update::in_view
Keeping a shown cursor shown when the graph pane loses history rows without a key.
## update::journal
The undo journal screen's state (`J`): cursor, marks, and the read that keeps it current.
## update::keys
The graph's own keys: movement, view changes and toggles other than search and the commit-graph offer.
## update::map
The Map view's state and keys (`M`): built fresh on open, thrown away on close.
## update::mouse
What a mouse event does once `geometry` says where it landed: clicks, wheel, divider drag, menu clicks.
## update::name_prompt
The one-field modal naming something new: a branch (`b`), a tag (`t`), or the branch a stash becomes.
## update::op_keys
graft-ops' key words spelled from the user's live keymap: the one `KeyNames` answer.
## update::operate
Working-tree writes from the changes pane: stage, unstage, stage all, and opening the discard box; one write at a time.
## update::palette
The command palette and help (`?`, `⌃P`, `:`): one state, one listing from the live key table, one filter.
## update::picker
The branch picker (`C`): type part of a name, `⏎` switches through `switch` (on the branch HEAD is on it closes, as `Tab` does), `Tab` shows it on the graph; `hint_context` picks the bar (`Picker` / `PickerOnCurrent`).
## update::place
Places: the blame and history screens opened over the graph, and the `^` trail `Esc` walks back.
## update::prelude
The harness's key prelude: keys a gallery stop presses before its first frame, each once the state is settled.
## update::preview
Ghost Preview: every history-changing key opens this first; the answer is drawn as ghost rows, `y` runs it.
## update::quit
`q`: quit, unless a write is running; then a box says what quitting does to it.
## update::radar
The Mistake Radar's state: the default-branch context its rules need, and the resolved `[radar]` policy.
## update::recover
A put-back from the journal once it has run: the line it leaves and the tags it puts on the graph.
## update::relaunch
Landing on an operation git is part-way through: the banner takes the keyboard and the changes pane opens where the work is.
## update::remote
Fetch, pull and push (`f`, `p`, `P`): the push box, `Esc` cancelling a running write, progress and failure lines.
## update::resolver
The conflict resolver: a modal over one conflicted file; `o`/`t`/`b` take a side per block, `⏎` marks it resolved.
## update::rewrite_editor
The rewrite editor (`R`): the branch as rows, the plan and squashed message shown before `y` runs the rewrite.
## update::row_tags
Tags a graph row carries after its subject until the next write starts, e.g. `✎ changed`.
## update::search
`/`: the prompt, the running search and stepping between hits.
## update::selection
The uncommitted-changes row's one-bit selection, kept beside `History`'s cursor.
## update::stash
The stash overlay's state (`S`) and its keys: save, apply, pop, drop.
## update::state
Everything Graft knows as one value (`AppState`), plus the small types it is built from.
## update::status_read
The working-tree status read the app is waiting on after a write, and the line that waits for it.
## update::strip
`O`: the overview strip's only key.
## update::switch
The switch rule every switch goes through: `.` Checkout, the name prompt's `⇥`, the picker's `⏎`.
## update::tour
The practice tour inside the repository Graft built for it: ticks read from that repository's journal.
## update::tree_answers
What `answer` hands the repository opening, the history walk starting, the working-tree read and the stash entries.
## update::undo
`u`, `Ctrl+Z` and `U`: when a key is a no-op, the drift guard, and the `Command::Undo` it becomes.
## view
The pure `view(state)`: reads and draws, never changes anything; everything from a repository crosses into `ui` as `SanitizedText`.
## view::commit_editor
The commit editor's view model for `graft_ui::commit_editor`, every repository string sanitised.
## view::credential
The credential prompt's view model; `secret` decides how the text is drawn, no character counter.
## view::detail
The detail pane's view model, reshaped from what `update` keeps.
## view::entries
One frame's `GraphEntry` rows: the selection's thread, the uncommitted row, each row's cells and badges.
## view::explain
A radar `Explanation` turned into the sanitised `ExplainView` the Explain panel draws.
## view::finish
The finish summary's view model; keys, mouse and draw build the same view.
## view::frame
The top of the draw: picks the tier and screen, then hands off to `rows` and `status`.
## view::hit
Which drawn thing a screen cell belongs to: the hit-test half that needs the view's layout.
## view::journal
The undo journal screen's view model (`J`): rows, with marks answered by `graft_ops`; a lost commit's `Fate` picks `RowKind::Lost`/`Replaced`/`Deleted` and its words (`· undo brings it back` for `⇢`).
## view::map
The Map screen's widget input, built from `state.map`.
## view::name_prompt
The name prompt's view model: the title says what is being named and where.
## view::picker
The branch picker's view model for `graft_ui::picker`; row order is `graft_ops`' answer.
## view::preview
Drawing an open Ghost Preview: the slice around the cursor with ghosts laid in and refs where they would land.
## view::radar
`AppState` facts turned into radar `Finding`s, then into the graph marker and the detail pane's radar lines.
## view::resolver
The conflict resolver's `DocView`: the one sanitisation site for a conflicted file.
## view::rewrite_editor
The rewrite editor's view, its area and its bar's context; every non-refusal word is chosen here.
## view::rows
Building `GraphView`: the rows the walk last sent, folded and painted for this frame.
## view::stash
The stash overlay's view model (`S`): rows from the last read; decides nothing.
## view::status
Building `StatusView`: what was settled at startup and what has happened since.
## view::strip
Building `StripMarks`: the overview strip's marks, read off the rows `History` holds now.
## view::tour
The tour card's words: one line per tour task, every key spelled from the live keymap.
## view::strips
The pixel tier's rows: the viewport's history cut into whole aligned strips, painted like the Unicode tier.
## working_tree
The working tree's full status, one entry per changed path, for the uncommitted row and the changes pane. Read by a job.
- `WorkingTree` (struct) — staged, unstaged and untracked entries as one read gives them.
