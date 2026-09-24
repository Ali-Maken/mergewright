# Scout 25: stable file order, and opening a done file read-only (main c56b64e)

## What exists
- **ops open.** `open_conflict(repo, &WorkingTreeStatus, path) -> Result<Opened, OpenError>` is at `graft-ops/src/conflict.rs:79`. `OpenError` is `Refused(Refusal) | Read(io) | TooLarge | Markers` (`:49-66`). It has no status variant. `unmerged_how` refuses with `Refusal::NotUnmerged` (`:223-230`; `operation.rs:446`). `Refusal::Unmerged { paths }` already exists (`operation.rs:294`), and its words say "resolve and `git add` them in the shell first" (`:508`).
- **Set order.** `ConflictSet.paths` lists the unmerged paths first (`conflict/set.rs:28-31`).
- **Job.** The runner's `open_conflict` (`jobs/runner.rs:549-560`) calls `graft::jobs::conflict::open` (`jobs/conflict.rs:109`). That function **already reads the status itself** (`:112`), calls ops, maps the refusal to `OpenFailure { words, not_unmerged }` (`:113-124`), and adds `on_disk` (a glimpse of the file on disk) for cards. The product is `Product::ConflictOpened(Result<Arc<OpenedFile>, OpenFailure>)` (`update/events.rs:326`). `OpenedFile` is graft's own type (`jobs/conflict.rs:29`), not ops' `Opened`.
- **conflict_opened** is at `answers.rs:17`. Any `Err` goes to `open_failed` (`:97`), which **sets `state.resolver = None`** in every case: "resolved outside" when it already had a `file` and `not_unmerged`, otherwise a sticky message offering `e edit  M mergetool`.
- **List.** `listed_files` (`files.rs:68-99`) builds its order from the set's paths, then the unmerged paths, then the open path, each appended once (`:77-81`). No sort. A row is `Done` when the status has arrived and the path is not unmerged (`:89`). `select_file` refuses a `Done` row at `:168-178`. `switch_refusal` is at `:191`.
- **Moving to another file.** `reopen` (`open.rs:88`) builds a fresh `ResolverState::opening` and pushes `OpenConflict`, carrying over only `files`, `set_job` and `seen`. `restart_landed` goes through `reopen` (`restart.rs:73-94`). `handed_back` pushes `OpenConflict` plus a status read (`answers.rs:201-226`).
- **Keys.** `act` (`resolver.rs:260-282`) handles only plain keys: `o/t/b B n N ⏎ e d M Esc`, then scroll keys. `j/k` scroll the document (`scroll.rs:130-131`). `restart_box` refuses when `whole_file` is `Some` (`restart.rs:32`).
- **Readers of `whole_file`** (scout question 4): `files.rs:115` (`open_state`), `walk.rs:26`, `restart.rs:32`, `whole_file.rs:72,104`, `take.rs:23-33` via `whole_file_take`, `view/resolver.rs:33`, `view/rows.rs:487-495` (`resolver_context`). Writers: `answers.rs:47,69`.
- **Hints.** `HintContext::Resolver { base, restart, whole_file, roles }` is at `hints/context.rs:103`. The lists are at `hints/lists.rs:22-31,249,281`. `every_context` forms are at `hints/catalogue.rs:115-140`. `coverage_slot` is at `graph_screen/tests.rs:1945` (Resolver uses slots 20-24).
- **View.** `resolver_title` / `Identity { Blocks, Whole }` are at `view/resolver.rs:196-222`. `RegionView::WholeFile` also carries `ours_absent` and `theirs_absent` (`view/resolver.rs:77-89`). The three-pane card rows are at `graft-ui/conflict_resolver/panes.rs:132,226`.
- **status_read.rs** holds only `ask`, `owed`, `answered`, `settle` and `abandon`. No resolver routing.

## Answers to the ticket's scout questions
1. The job passes a status it read itself (`jobs/conflict.rs:112`), never the app's copy.
2. `Product::ConflictOpened`. `Refused(NotUnmerged)` becomes `not_unmerged: true`, and `open_failed` then closes the resolver (`answers.rs:104-106`).
3. Scout question 3, on a zero-block document: `n` crosses via `next_conflicted_file` (`walk.rs:55`, `:90-95`). That function walks the **unmerged** order with `skip_while(path != after)`. On a path that is not unmerged it returns `None`, so the line reads "no more conflicts". `N` never crosses (`walk.rs:52-54`). Same on a card.
4. The readers are listed above.

## Contradictions
- **C1.** Scope 2 says the product "carries `Opening` instead of `Opened`". The product carries `OpenedFile` (`events.rs:326`), not `Opened`, and adds `on_disk` in graft (`jobs/conflict.rs:125-134`). The seam type and `OpenFailure.not_unmerged` need deciding.
- **C2.** The D2 route says "the app never picks the route from its own status copy". It already doesn't (`jobs/conflict.rs:100-112`). The new ops status read would duplicate the job's.
- **C3.** "`n`/`N` cross files as on a card" is false today: on a card, `N` stays put and `n` crosses only in unmerged order. On a done path `n` finds nothing (`walk.rs:90-95`). `n` needs a list-order change, not a pin.
- **C4.** The refused-key list names `Ctrl+O`, `Ctrl+T` and `M`. `Ctrl+O`/`Ctrl+T` are unbound in the code (`resolver.rs:266`, `_ => {}`). They are claimed only in the keymap (`02-keymap.md:263`, P3). `O`/`T` (keymap `:264`, P6) are claimed there but missing from 25's list.
- **C5.** Hints plan `j/k` "only if P7 bound them". `j/k` are bound today as scroll keys (`scroll.rs:130`), not as P7's file moves.
- **C6.** The Scope 7 `RegionView::WholeFile` literal omits `ours_absent` and `theirs_absent` (`view/resolver.rs:87-88`).
- **C7.** "The resolver never closes on a listed path" also needs `open_failed`, which closes the resolver on **every** error (`answers.rs:106`), TooLarge and Markers included. Scope doesn't mention it.
- **Could not confirm:** that "no other reader relies on" `ConflictSet::paths` order. I did not trace every reader.
