# review-fix-amend-13: a file-list click under `d`'s box closes the resolver; `c` refused on a stale status; two over-long files

Status: done
Blocked by: none (13 merged). Runs **before** 14 and 25 (same crates, ADR-0036).
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-13.md` Major 1, Major 2, Minor 1. Minor 1 promoted: the app says `every file done ✓ · c finishes the merge` and then refuses `c` — the words contradict the key.
Build with: **Opus** (medium).
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-13.md`: "The builder's question", Major 1, Major 2, Minor 1 — each names file and line.
- `crates/graft/MAP.md` (`update::resolver`, `banner`, `jobs`), `crates/graft-ui/MAP.md` (`conflict_resolver::files`).
- Ticket 12's `ReadRefs(JobId)` guard (`.scratch/m6-conflicts/issues/12-landing-after-a-stop.md` NOTES) — the model the review suggests.
- `ARCHITECTURE.md`; `codebase-design` for the splits.

## Scope
In:
1. **Major 1.** A click on a file-list row while `d`'s confirm box is up closes the resolver and leaves the box (`update/resolver/files.rs:190-202` has no box check; `update/resolver.rs:447-453` removes the resolver before the refusal). Failing test first. Fix at the cause: a click on the list under a modal box does what every other key under that box does.
2. **Minor 1 — stale `c`.** After the last Stage, `answers.rs:281-298` says `every file done ✓ · c finishes the merge`, but `c`'s refusal counts the cached status (`banner.rs:200-206`, via `:77-86`) until the status refresh the Stage queued (`operate.rs:966-974`) lands. Failing test first (the review's: Stage last file, press `c` before the read lands). Fix at the cause, no sleep/retry/special case (rule 2). The review offers: give `ReadWorkingTree` a `JobId` (as 12 did for refs) and hold the `every file done` words until that read lands; or `continue_refusal` says `checking the files…` while the read is out. Choose, say why in NOTES. The words and the key must never disagree.
3. **Major 2.** Split, pure moves, ~400 lines per file where a seam exists: graft-ui `conflict_resolver/files.rs` (612); graft `update/resolver.rs` (476). Update MAP.md. No seam → say so with the count.

Out: Minor 2 (split computed twice) — ticket 14 moves the width source to `resolver_split`; every other Minor → KNOWN_GAPS.

## Acceptance (each tick names its test, at tick time)
- [x] Major 1: failing test first, red on unchanged code — test: `crates/graft/src/update/resolver/tests/file_list.rs` `a_list_click_under_the_box_leaves_both_as_they_were` (red on `ee8b2ad`: `the resolver is as it was — left: None`, the review's probe)
- [x] Major 1: a list click under `d`'s box leaves the resolver and the box as they were — test: same test, green: the click sends nothing, `resolver`, `confirm` and `status` unchanged; its second half calls `select_file` under the box → `finish the box first`, resolver unchanged (the `reopen` path)
- [x] Minor 1: failing test first, red on unchanged code — test: `crates/graft/src/update/resolver/tests/words_and_c.rs` `c_before_the_status_read_lands_agrees_with_the_line` (red on unchanged code: `the line says Done("every file done ✓ · c finishes the merge") but c answered Sticky { reason: "1 file still conflicted", … }`)
- [x] Minor 1: after the last Stage, the words and `c` agree at every moment before and after the status read lands — test: `words_and_c.rs` `the_line_and_c_agree_before_and_after_the_read_lands` (read out; an older read lands; the owed read lands; `c` pressed meanwhile; an outside conflict after the read) and `a_read_that_never_answers_stops_the_checking_line`. End to end with the real runner, pumping until `status_read` is `None`: `tests/it/resolver/last_file.rs` `last_file_closes_to_where_it_opened`, `tests/it/resolver/file_list.rs` `a_rebase_lists_only_this_steps_files`, both `tests/it/resolver/stash_pop.rs` tests
- [x] Major 2: splits done, line counts in NOTES, MAP.md updated — gates green; files: `crates/graft/src/update/resolver/open.rs`, `crates/graft-ui/src/conflict_resolver/files/strip.rs`, `crates/graft-ui/src/conflict_resolver/files/column.rs`, `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`
- [x] Gates workspace-wide: `cargo fmt --check` pass; `cargo clippy --all-targets -- -D warnings` pass; `cargo nextest run --workspace` 2796 passed, 1 skipped; `cargo test --doc --workspace` 3 passed

## NOTES

No scout: the review names every line (the conductor's call). I read the named lines directly.

**Major 1: fixed at the cause.** The mouse reached the resolver while a box owned the keys. `resolver_mouse` had no box check; keys go to `confirm_key` first (`update/keys.rs:34`).
- Fix: `resolver_mouse` does nothing while `box_open` (a confirm or the commit editor). The box owns the mouse as it owns the keys.
- Defence at the same cause: `switch_refusal` refuses under a box (`finish the box first`, before `remember` writes `seen`). `reopen` now decides `open_refusal` **before** taking the resolver, so any refused switch leaves it as it was.
- `resolved_advance` and `restart_landed` clear their landed id before `reopen`. A refused reopen then cannot strand a stale `staging`/`restarting`. That path is unreachable today (nothing opens a box during those writes) — by reading, not tested.
- Cost to user: none. A click under the box is silent, like a stray key.

**Minor 1: fixed at the cause. I chose (a) and (b) together.**
- `ReadWorkingTree` now carries a `JobId`: `Command::ReadWorkingTree(JobId)`, `queue_as` in the runner — ticket 12's `ReadRefs` model.
- Every read is built by `update::status_read::ask`, which records the newest in `AppState::status_read`.
- While it is out, both the last file's close and `c` say `checking the files…` (`Status::Busy`, never an error).
- When that read lands, `status_read::settle` turns the line into the verdict from the fresh status: `every file done ✓ · c finishes …`, or `c`'s own refusal if an outside edit left a file conflicted.
- Why both: (a) alone leaves `c` counting stale entries under a neutral line. (b) alone leaves `c finishes the merge` on screen while `c` says "checking". Together they show the same words at every moment.
- Cost to user: after every write, a `c` inside the read window answers `checking the files…` and must be pressed again once the line invites it. That includes the window after a focus-gained read. The window is milliseconds on a small repo; not timed on a large one.

**Split counts** (total lines; tests live in their own files):
- graft `update/resolver.rs` 476 → 393. New `resolver/open.rs` 109: `open_resolver`, `start`, `open_refusal`, `box_open`, `reopen`.
- graft-ui `conflict_resolver/files.rs` 612 → 359. New `files/strip.rs` 180: `strip_plan`, `grid_plan`, `strip_label`, `one_row_cell`, `grid_cell_text`, the strip's constants. New `files/column.rs` 96: `column_plan`, `column_width`, `column_header`, the column's constants.
- Deviation from the review: `window` stays in `files.rs`. The column uses it too, so it is shared plan, not strip.
- Pure moves; no public path changed. New `update/status_read.rs` is 78 lines.

**Gaps for the conductor to file** (builders don't read KNOWN_GAPS):
- **Broken**, by reading, pre-existing, not tested: an **older** working-tree read that lands after a newer one still overwrites `AppState::working_tree` (`update/mod.rs` `working_tree_read` has no id guard). It no longer affects `c` or the done line. The list and pane can show the older status until the newer read lands. The id now exists to drop it.
- **Unverified:** how long the read window is on a large tree (not timed). It now only decides how long `checking the files…` shows.
- **Unverified:** after a cancelled owed read (shutdown), the line goes to `Ready` and `c` counts the cached status again. `a_read_that_never_answers_stops_the_checking_line` covers only the line.
- **Note:** `M` queues two reads (`mergetool_returned` and `handed_back`); pre-existing, harmless, the newest wins.
- **Untouched:** every other Minor in `docs/reviews/m6-amend-13.md` (3–13) → KNOWN_GAPS, per the ticket. Minor 2 → ticket 14.

No compaction.
