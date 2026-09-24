# Scout 28 — main 4a6703d

## graft-ops
- `prepare_restart(ctx: &Context<'_>, op: &Operation, facts: &RepoFacts) -> Result<RestartPrep, ApplyError>`, `pub(crate)`, `apply/restart.rs:167`. Error type **is** `ApplyError`. Called once, `apply.rs:200`, after `op.check(&facts)` (`:196`).
- It holds only the **not-unmerged** half: `stage_source` → None ⇒ `NoStagesKept` (:178); Backup → `shape_of_stages` → `NotMarkerShape` (:194) / `NoStagesKept` (:201). `Index` ⇒ `RestartPrep::Nothing` (:183) with no shape check.
- The **unmerged** half (`has_markers` on the status) is `operation/conflict.rs:238 check_restart`, `pub(super)`, reached via `Operation::check` (`operation.rs:908`) — not inside `prepare_restart`.
- `stage_source(ctx: &Context<'_>, facts, path) -> Result<Option<StageSource>, FactsError>` (`restart.rs:67`). Uses `facts.status` + `facts.in_progress` only (:72–:80), **plus `ctx.journal.entries()`** (`chain_states` :101, `journal_states` :120) and `ctx.repo`, `ctx.stop`. `Context` (`apply.rs:96`) needs `&mut Journal`, `GitVersion`, clock, random.
- `shape_of_stages` `pub(crate)` (`operation/conflict.rs:267`).
- Refusal words: `operation.rs:631` (`… — no markers to take from`), `:636` (`… sides were not kept`).

## graft job
- `jobs/conflict.rs:123 open(store, path, stop)` — no git version, no journal. Own status read :126; done route :130–:138 sets `done: true`; `OpenFailure.done: bool` :89; `failed()` :225 `done: false`. `Opening::Done(Arc<DoneFile>)` :99.
- `conflict_set` (:287) reads `system_git()` + `read_facts` with `FactsScope::default()` — pattern exists. Journal open pattern: `jobs/operate.rs:277–289`, `jobs/undo.rs:295–307` (`Journal::open` + `Context::open`). Called from `runner.rs:556`.

## graft update / view
- `resolver.rs` = **442** lines now (ticket says 414). `act` :295–:331; done gate :302–:315 (keys `o t b B M n N ⏎`); `mergetool_here` :335; `edit_here` :393–:409 (no refusal of any kind); `close` :412. `write_refusal` :251 checks `job`/`operating` only, not `in_flight`/`queued`.
- `handed_back` `answers.rs:270`: `drop_takes()` (`resolver.rs:220`) + notice `<n> queued takes dropped` :292.
- `ResolverState.done: Option<DoneShape>` :171. Readers: `resolver.rs:66,210`; `resolver/done.rs:44,101`; `files.rs:118`; `answers.rs:37,44 (failure.done),54`; `view/rows.rs:494`; `view/resolver.rs:43,50,97`; `view/resolver/whole_file.rs:56`; tests `update/tests.rs:3092,4423,4470,4942`, `tests/advance.rs:404`, `tests/file_list.rs:685,722,730,759,860,930`.
- `restart_box` `restart.rs:24`: `write_refusal` → `whole_file` NotMarkerShape (:32) → box. `done_refusal` `done.rs:42`, always `… · d starts it over`.
- `resolver_context` `view/rows.rs:485`: `restart: !card`. Whole-file card → `resolver_card_list` (`graft-ui/.../hints/lists.rs:287`: `o t e n M esc`) — **no `d` today** (Q5).

## graft-ui
- `lists.rs:21` `done: true` → `resolver_done_list` (:300) ignores `restart`. `catalogue.rs:141` one done form; `tests.rs:1968` `coverage_slot` maps all `done: true` to slot 30.

## Keymap
- §2.6 `d` `02-keymap.md:266`; `e` :271, `M` :272. :111–:114 "a key displayed must work".

## Contradictions
1. Signature `restart_gate(repo: &RepoHandle, …)` cannot call `stage_source`, which needs `&Context` (journal) — `restart.rs:67,101,120`. Job would need to open the journal (`operate.rs:277` pattern); could not confirm a read-only journal open exists.
2. "prepare_restart … drops its own copy of the two checks": the unmerged check is `check_restart` in `Operation::check` (`operation.rs:908`), not in `prepare_restart`.
3. `resolver.rs` is 442, not 414 (ticket :15).
4. Pin is not `:196`: `start_over` helper `it/resolver/file_list.rs:194`, used by `:204` (shell merge, gone.txt) and `:245`. `done_open_capture` :314–:328 also pins `d starts it over` on shell-merge gone.txt — not named by the ticket.
5. `done_refusal` doc (`done.rs:38`) lists `M` among refused keys; ticket's R4 plus new `in_flight` rule — `M` is already refused on done files (`resolver.rs:307`).
