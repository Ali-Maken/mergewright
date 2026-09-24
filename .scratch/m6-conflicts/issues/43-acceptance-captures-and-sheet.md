# acceptance-captures-and-sheet: ticket 21's remainder — captures, contact sheet, TESTING.md

Status: done (2026-09-24; the session compacted once — see NOTES)
Blocked by: 42 (its fixes change frames 43 captures).
Was: split by the conductor 2026-09-24 from ticket 21, which compacted once after part 1 (merged `36e89da1`). Scope, model and rules unchanged from 21; the seam is 21's own ("parser and journeys first; TESTING.md and captures second").
Build with: **Fable**. It is the acceptance ticket's second half.
Needs: 42.
Crates: graft (tests), scripts, docs.

## Read first
- Ticket 21 in full: its Scope 6, 7, 8 and its NOTES, **"Remaining (exact next steps)" 1, 2, 4** and "Gotchas". Those are this ticket's steps; follow them as written there.
- ADR-0037 (contact sheet). `.scratch/m6-conflicts/captions-fix-draft.tsv`.

## Scope
In: 21's Remaining steps:
1. The 10 capture stems.
2. The contact sheet, split into parts of 200 KB or less, with every `fix-*` capture in ticket order under its caption, including the captures from tickets 37–42.
3. TESTING.md "Conflict journeys": the Zed `Ctrl+T` line is marked **unverified**. Confirm "Times I left" :108 is already closed (`shell_begun::c_enter_finishes_a_shell_begun_merge`).

Out: perf (ticket 44). The M6/32 bar-words facts (conductor → researcher). Any behaviour fix: file it with a line, or as a question.

## Acceptance (each tick names its test, at tick time)
- [x] `every_scenario::contact_sheet_captions_cover_every_m6_capture`, green with the new frames. (21's box.) Widened to `fix-*` (the sheet itself excluded); red before `docs/shots/m6/m6-contact-sheet-captions.tsv` gained the 47 `fix-*` + 50 journey rows, green after (nextest, 2026-09-24).
- [x] TESTING.md "Conflict journeys" written, every line concrete (no "looks right"), with the Zed `Ctrl+T` line marked **unverified** (TESTING.md "Press `Ctrl+T` once in Zed's terminal — unverified"). (21's box.) Its journey rows restate the note's counts, which `conflict_journeys::tests::journey_counts_match_the_note` holds to the routes; its capture table's stems are held to the code and matrix by `every_scenario::captures_are_named_keyed_and_listed_in_the_matrix`. "Times I left" :108 was already closed — `resolver/shell_begun.rs:270` `c_enter_finishes_a_shell_begun_merge`, green in this run.
- [x] The captures are listed by path below. The user judges them by eye (rule 6); colour unverified (rule 5). Every landing's `on_screen` words were found in its file by the pty run (`.scratch/m6-conflicts/pty-walk-m6.py`, second run without `GRAFT_UPDATE_SHOTS`: 50/50 `settled`) — that checks words, not looks. Stems ↔ files ↔ matrix: `every_scenario::captures_are_named_keyed_and_listed_in_the_matrix`, `coverage::tests::write_matrix_writes_all_six_tables_where_the_ticket_wants_them`.
- [x] Contact sheet part paths below; each ≤ 200 KB (200 000 bytes, the strict reading): `scripts/contact-sheet.sh` refuses a section over the cap and splits between captures; sizes from its own output.
- [x] Gates workspace-wide: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` (3047 passed, 1 skipped), `cargo test --doc --workspace` — all exit 0.

## NOTES (builder, 2026-09-24) — **this session compacted once**

Scout: none new; used `.scratch/m6-conflicts/scout-21-harness.md` (21's).

**Captures** (`docs/shots/m6/`, 50 files, each 530–3817 bytes): `m6-10-stop-lands-in-resolver`, `m6-11-card-on-notes`, `m6-12-after-t`, `m6-13-after-ctrl-t`, `m6-14-last-file-done`, `m6-15-finish-summary`, `m6-16-summary-expanded`, `m6-17-after-commit`, `m6-18-launch-shell-begun`, `m6-19-after-esc-esc`, each `-{120x40,80x24,160x48,187x50,200x50}.txt`. Keys and landing words: `crates/graft/tests/it/m5_walk/conflict_journeys.rs::captured`; TESTING.md "Conflict journeys → Captures".

**Contact sheet** (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`): `docs/shots/m6/m6-contact-sheet-1.txt` (199 886 B), `docs/shots/m6/m6-contact-sheet-2.txt` (91 069 B); 120 captures, 0 uncaptioned. The single `m6-contact-sheet.txt` is deleted (the script removes it). Script: `--captions-order`, parts ≤200 000 B, the "earlier milestone" label only on `../` names. Also ran it for m5 to check it still works (split into 2 parts), then restored m5's committed sheet — **the committed `docs/shots/m5-contact-sheet.txt` is 281 412 B, over the 200 KB cap** (textual; not this ticket's to re-cut).

**Outside the crate set — a harness fix in graft src** (`crates/graft/src/update/prelude.rs::settled`, 2 tests in `update/tests.rs`, both red first). Cause, not symptom: the capture prelude fed keys before reads landed. (a) `⏎`/`Space` fed while the finish summary read (`finish.job`/`diff_job`) — `⏎` is refused while reading (`finish/tests.rs::enter_while_reading_says_wait`), so m6-16/17 captured the summary loading. (b) `n` fed before the resolver's conflict set (`set_job`) and the working-tree status (`status_read`) landed — `listed_files` (`resolver/files.rs:69-87`) saw a list of one, so m6-12 at 160x48 took `gone.txt`. The prelude is empty in the shipped binary, so the user sees no change. Tests: `the_prelude_holds_a_key_while_the_file_list_is_reading`, `…_while_the_finish_summary_is_reading`.

**Pty script:** `.scratch/m6-conflicts/pty-walk-m6.py` pins `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` (fixture epoch + 10 min) so m6-17's merge commit id is stable between runs.

**Stale earlier frames (not re-taken; out of scope):** a re-run of ticket 07's captures without `GRAFT_UPDATE_SHOTS` drifted on `m6-01`, `-02`, `-03`, `-04`, `-05`, `-07`, `-09` at 120x40 and 80x24; `m6-08` held; `m6-06`'s relaunch never reaches `took ours` before the kill. Their captions are prefixed **STALE FRAME** and TESTING.md "Conflicts walk" says so. Re-taking them is a follow-up ticket's call.

**Questions (no line, rule 7):**
1. Extends 21's question 3: `begun outside Graft` is drawn at none of the five sizes; at 160x48 the launch footer also drops the file count (`merge in progress · ⏎ resolve  c continue`; 120/187/200 show `2 files conflicted`). Intended?
2. Unread files show `…` for their state (`docs/shots/m6/m6-10-stop-lands-in-resolver-120x40.txt:4`, `notes.txt · …`; the three-panes column shows `gone.txt   …`). Reads as "not read yet" — intended wording?

**Not verified:** colour anywhere (rule 5); any look (rule 6); the Zed `Ctrl+T` line; capcheck (a scratch script matching each caption's backticked phrases to its file) misses only phrases that are absences, key names spread across the bar, wrapped box text, or in STALE frames — listed in this session's scratch output, not re-checked by eye.
