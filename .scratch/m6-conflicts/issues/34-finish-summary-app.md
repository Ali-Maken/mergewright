# finish-summary-app: the app half of 19 — `c` opens the summary, it commits, `Esc` goes back, `space` shows a git-combined file

Status: ready
Blocked by: none (19's graft-ui half merged, `04af62c1`).
Was: split by the conductor 2026-09-24 from `19-finish-summary-screen.md`, which compacted mid-ticket (builder NOTES "Next", steps 1–5, 7, 8). Scope, signatures and model are 19's, unchanged. Choices and captures are ticket 35.
Build with: **Opus** (19's).
Needs: none.
Crates: graft.

## Read first
- **`19-finish-summary-screen.md` is the spec** — Scope, Signatures, rulings S1–S9, and its builder NOTES ("Done", "Next", "Gotchas and questions").
- `.scratch/m6-conflicts/scout-19.md` (line numbers predate 16/17/32/33; re-find by name).
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md` (`finish_summary`).

## Scope
In — 19's builder NOTES "Next" steps:
1. Events (`ReadFinishSummary`, `ReadFinishDiff`, products, `took_fact`).
2. `jobs/finish.rs` (summary, staged diff), runner dispatch.
3. `update/finish.rs` per 19's Signatures: `finish`, `finish_draft` fields; overlay order; `text_field_open`, `modal_owns_keys`. (`choices` field: declare it, empty — 35 fills it.)
4. Keys and the banner: `finish_key`; `continue_pressed` opens the summary for a merge; `resumed` → `finish::resume_answered`.
5. Editor route (no `strip_comments` on the finish route, S3); fix graft MAP line on "already stripped".
7. View, mouse, paste (`view/finish.rs`, the frame arm, `overlay_context`, too-small `esc back`).
8. Bind and flip: `finish_list` to `hint(..)`, Finish back into the sweeps, `BOUND_ACTIONS`, `presses_for`, fixtures.

Conductor rulings on 19's builder questions: `⌃E` / `⌃⏎` (the screen's glyph, as `⌃O/⌃T`); the files-focus bar without an expandable row keeps `j/k move` and drops `space diff` — accepted.

Out: step 6 (choices) and the captures → ticket 35.

## Acceptance
Tick **in ticket 19's list** (19 `:155`–`:168`, `:173`, `:179`, `:180`), each naming its test at tick time. Boxes `:169`–`:172` and `:182` are ticket 35's; `:181` is ticket 21's. For `:173` (`done_lands_on_the_merge_commit`, S9): observe it; if it does not land where 19 says, report the line — don't adjust the test to the code.
- [ ] 19's boxes above ticked with tests
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
