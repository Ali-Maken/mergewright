# finish-summary-app: the app half of 19 — `c` opens the summary, it commits, `Esc` goes back, `space` shows a git-combined file

Status: done (16 of 17 boxes; `:161` left unticked, see NOTES)
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
- [x] 19's boxes above ticked with tests — 16 of 17 ticked in 19's list, each naming its test; `:161` deliberately not ticked (below).
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (2940 passed, 1 skipped), doctests — all green, run with `CARGO_TARGET_DIR=/private/tmp/claude-501/graft-target-34`.

## NOTES
- **Context compacted twice** during this ticket. Nothing remains of 34's scope except the `:161` question below; the work resumed from the summary each time and the gates were run after the last compaction.
- Scout: `.scratch/m6-conflicts/scout-19.md` (existing; no new scout).
- **`:161` not ticked.** `it::finish::a_hash_line_from_the_editor_lands` passes, but the harness job pool cannot hand the terminal over (19's Gotchas), so the test takes the `EditCommitMessage` command and answers it with what a fake editor would write. The body line and `%B` are verified against real git; no `$EDITOR` process runs. The handover itself is the commit editor's existing `event_loop::running` route, unchanged. A real round trip would need the m5_walk `HandOver` step or a pty — ticket 35's captures, or the conductor's call.
- **`:173` observed passing** (`it::finish::done_lands_on_the_merge_commit`): cursor on `git rev-parse HEAD`, two parents, `merged feature into main ✓`. No finish-only landing: it reuses `banner::finished_lands_on_head`.
- **Hook refusal shape, confirmed by trace:** a failing `commit-msg` hook arrives as `Operated::Conflicted { paths: [], stalled: Some("`git commit --cleanup=whitespace -F -` failed (1): commit-msg: …") }`, not `Failed` — graft_ops' Continue commits through `git commit --cleanup=whitespace -F -` even when unedited. `finish::resume_answered` treats that shape (and `Failed`) as a refusal.
- **Deviations from 19's Signatures:**
  - `resume_answered` returns `Option<FinishAnswer>` (`Merged(words)` / `Kept` / `Closed`), not `bool` — `resumed` needs to know which of three things happened.
  - `FinishState` gains `from_draft: bool` so a late summary does not overwrite a restored draft.
  - `expandable` keeps graft-ui's meaning (the cursor is on a by-git row); `space` also expands a by-you row, unadvertised.
- **Bar gap (known, not fixed):** `j/k move` is drawn but does nothing when there is one row, or while the summary is loading or failed. The sweep states always have ≥2 rows, so it does not catch this.
- **graft-ui flip is outside 34's crate set** (`graft`), in its own commit so the conductor can drop it: `finish_list` entries `hint(..)`, `planned` kept test-only with `cfg_attr(not(test), expect(dead_code))`, Finish back in `all_contexts`. graft's sweeps were only run with it; the first commit alone is not verified to pass (planned Finish hints are filtered from the drawn bar, so the sweeps likely hold, but `every_bound_action_is_hinted_live_or_is_named_here` likely fails on the new `BOUND_ACTIONS` rows — not run).
- **Clippy `too_many_lines` refactors** (behaviour-neutral) in files outside the finish code: `jobs/runner.rs` (`start_search` pulled out of `dispatch`), `update/banner.rs` (`resumed_done`), `update/mod.rs` (`resized`), `update/tests.rs` (`bar_hints`; `classified_form`'s identity arms merged), `tests/it/m5_walk/run.rs` (`Resolved`, `FinishReady` moved to `wait_for_resolver`), `tests/it/m5_walk/vocabulary.rs` (`finish_the_merge`), `tests/it/conflict_interim.rs` (`committed_from_the_summary`).
- MAP: graft `update::finish`, `view::finish`, `jobs::finish`, the commands/products, `GraphOverlay::Finish`; line 115's "comments already stripped" corrected. graft-ui MAP: Finish hints now `Live`.
- Colour of the summary ships **unverified**; no captures here (ticket 35).
