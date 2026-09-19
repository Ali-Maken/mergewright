# Review: M3 group 06 — `u`/`U`, the journal browser, the drift guard, the stash overlay

Range `0305c88..4fcbe4c` (group commits `596cd57..4fcbe4c`; conductor's doc
commits and group 05's review-fix commits inside `3e99778..HEAD` skipped).
Reviewer: Fable. Read order per CLAUDE.md: diff-stat → MAP.md ×3 → diff
crate by crate; whole files opened only to confirm a finding
(`update/keys.rs` 1–70, `update/mod.rs` 378–400, `graft-ops` `apply.rs`
340–420, `restore.rs` 116–138 and 225–262, `journal/file.rs` 28 and 155–170,
`tests/it/frames.rs` 505–518).

**Counts: 2 Critical · 8 Major · 13 Minor.**

## Gates (run one at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | **fails to compile**: `crates/graft/tests/it/frames.rs:512:17: error[E0061]: this function takes 5 arguments but 4 arguments were supplied` — see Critical 2 |
| `cargo test` | **1601 passed, 0 failed, 19 ignored**; 35.05 s wall (incremental), ~33 s summed suite time; peak RSS **339 MB** |
| `cargo test --features heavy-tests -- --test-threads=2` | **did not run** — same compile error; 51 s to fail |
| release perf (`frames:: heavy::budgets_hold`, `--test-threads=1`) | **did not run** — same compile error. No frame or first-paint numbers exist for this group |
| `cargo bench -p graft-graph --bench hot_paths` (1 s warm-up, 3 s measure) | `StreamingLayout::advance/one chunk` 838 µs (801 µs at m3-05; criterion flags "regressed", +4.6 %) · `rasterise/one strip` 622 µs · `StripCache::get/hit` 1.73 µs · `StripCache::get/miss` 621 µs. `graft-graph` untouched by this group — noise, recorded only |
| fuzzing (`fuzz/fuzz_targets/parse_{unified,file_list,porcelain_v2}`) | not run: needs nightly `cargo-fuzz`; this group touched no fuzzed parser |
| `target/` | **23 GB** |

Five slowest tests (fast tier): `diff_fixture::parse_unified_matches_numstat_for_every_shape` 7.54 s ·
`layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.02 s ·
`corpus::two_builds_produce_the_same_object_ids` 3.02 s ·
`diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 1.73 s ·
`corpus::a_second_build_in_the_same_place_is_a_no_op` 1.14 s. The new
key-to-repo tests are all under 0.9 s each (`ops_execution_hard::stash_pop_round_trips`
0.83 s is the slowest M3 one). Laptop-safe.

## The `u` evidence

**Real.** `crates/graft/tests/it/undo_by_key.rs` and `stash_by_key.rs` drive
`update()` with keypresses, dispatch its `Command`s to a real `JobRunner`
(2 workers) over a `copy_into_temp` fixture, pump the real `Product`s back, and
then read the repository with `std::fs::read`, `git rev-parse HEAD`,
`git diff --cached --name-only` and `git stash list` — never out of
`AppState`. Checked line by line:

- discard → `u`: `after == before && after == b"\x00\xff\n"` on the bytes from disk (undo_by_key.rs:76–80). Byte-for-byte as claimed.
- commit → `u` → `U`: `rev-parse HEAD` equals the pre-commit hash after `u`, `diff --cached` lists the file, and equals the post-commit hash after `U` (undo_by_key.rs:135–160). Goes through the real editor and `Ctrl+Enter`, which is stronger than the spec's injected `Operate(Commit)`.
- stash `s`/`Enter`/`p`/`d` each with `u` (and `U` for the push): `git stash list` length and message asserted after every step; the drop test moves to `stash@{1}` first so the wrong entry cannot pass (stash_by_key.rs:180–222).
- badges on a real repo through the real refs job: one `RefLabel::Stash(n)` per entry (stash_by_key.rs:229–244).

The soft spot is the journal claim — Major 7. The **id-rewind fix** is at the
cause for the two callees it names and the regression test would catch either
of them re-minting from `state.local_jobs`; it does not guard the caller-side
pattern that made the rewind possible — Major 5.

**Does `u` undo the right entry?** Within one process, yes. The browser's
marks and `Enter` both come from `graft_ops::journal::{next_undo,next_redo,
in_force_all}` (view/journal.rs:52–70, update/journal.rs `enter`), the job
re-derives `next_undo` from the freshly opened journal, and every write puts
`journal.entries` into `Pending`, under which the browser has no rows and
`Enter` is a no-op — so no stale row can be acted on. Across processes the
forced re-run is not pinned to the entry the box named — Major 4.

**The drift guard** compares refs, `HEAD`, the index and the working tree
against the entry's recorded after-state, and a touched field that was never
recorded is `Drift.unknown`, which is *not clean* — the box fires and its text
says "was never recorded; undo restores all of it" verbatim from
`Drift::explain()` (the capture `m3-undo-drift.txt` shows exactly that). The
wrapping fix keeps the whole sentence on screen. Sound.

## Are the ticks honest?

Mostly. Every test named in the NOTES exists (`grep -rn "fn <name>"`: 14 in
`update::undo`, 8 in `update::journal`, 5 in `view::journal`, 6 in
`journal_screen`, 20 in `update::stash`, 7 in `stash_screen`, 5 + 3 e2e, 3
focus). Four ticks are not fully earned, each named below: the interrupted
row is proved on a status the real path can never hand the view (Major 2);
"journal has … entries with `reverses` set" is asserted by a heuristic that
checks neither (Major 7); "`← u` / `← U` marks … visible in the grid" — no
capture shows `← U` (Minor 12); and piece 7's `S stash` hint is declared
already present on a false premise (Major 8). None of the four is a
fabricated test — they are claims a hair wider than the test under them.

## Seam 1 / handoff check

Nothing fell through. Every item in the seam-1 handoff is in the tree:
`Command::ReadStashes`/`Product::Stashes`/`Task::ReadStashes` rewritten,
`focus_gained` calls `stash::refresh`, `ConfirmThen` grew `Operate`, the
overlay copies the journal shape. What the handoff did **not** carry was the
seam-1 decision that `u`/`U` are global and run before every pane — and seam 4
then added a text field below that line. That is Critical 1.

---

## Critical

### C1. Typing `u`, `U` or `Ctrl+Z` in the stash save prompt runs an undo instead of inserting the letter

`crates/graft/src/update/keys.rs:75` — `is_undo_key(key)` returns before
`pane_and_overlay_key` (keys.rs:106), and that is the only route to
`update::stash::prompt_key` (stash.rs:319) → `edit` (stash.rs:343). The only
things ahead of it in `core_event`/`key_event` are the search prompt, the
confirm box, the context menu and the commit editor (mod.rs:383–390,
keys.rs:31–66). So with the prompt open: `s`, type `fix auth bug` — at the
`u` the prompt loses the letter and `Command::Undo(_, false)` is dispatched.
`operating` is set, the status reads `undo…`, and the user's last stage,
discard or commit is reverted — a journaled write they never asked for,
triggered by spelling. The prompt stays open with `fix a` in it.

Why it matters: this is the milestone's promise inverted — the key that
"keeps work" destroys it, from inside the one text field the group added.
Redo (`U`) is the same. No test types a `u` into the prompt: the unit test
feeds `wip` straight to `stash_key`, the e2e types `wip` too.

Cause: the comment at keys.rs:70–73 ("Not while a text field has focus — the
commit editor is modal and has already claimed every key above") was true in
seam 1 and false after seam 4, and the handoff did not carry the assumption.

Fix (cause): make the prompt modal the way the editor is — claim every key
while `next.stash.prompt.is_some()` **before** the undo check (a
`stash::prompt_open_key` arm beside the editor's at keys.rs:50–58), or gate
`is_undo_key` on "no text field has focus" as one predicate
(`AppState::text_field_open()` covering the search prompt, the editor and
the stash prompt) so the next text field cannot repeat this. Test through
`update()` not `stash_key`: `s`, `u`, `U` → `prompt.message.text() == "uU"`,
no `Command::Undo`/`Redo`, `operating` none.

### C2. The heavy tier does not compile, so no heavy or release-perf result exists for this group

`crates/graft/tests/it/frames.rs:512` still calls `refs::badges` with four
arguments; this group made it five (refs/mod.rs:220–226, `stashes`). `cargo
clippy --all-targets --features heavy-tests`, `cargo test --features
heavy-tests` and the release perf run all stop at `error[E0061]`. The plain
gates are green because `frames.rs` is heavy-only (KNOWN_GAPS "The heavy code
is not in the standard clippy gate" — this is the second group it has bitten).

Why it matters: ADR-0021 says frame budgets fail only under `--release`; the
M3 review has no frame or first-paint numbers after the largest UI group of
the milestone (two new overlays drawn in the graph area, a per-frame
`in_force_all` walk and `next_undo`/`next_redo` in the browser's view model).
Everything the group added to a frame is unmeasured.

Fix: `badges(&refs, &head, &upstreams, &[], |_, _| …)` at frames.rs:512, then
run the heavy tier and the release perf command and record the numbers in the
ticket. Process: add the heavy clippy line to CLAUDE.md's "Rust code is done
only when" list; a builder cannot see this break otherwise.

## Major

### M1. The interrupted-entry notice disappears on the first re-read, not on `J`

`crates/graft/src/jobs/undo.rs:239–248` derives `JournalRead::interrupted`
from `Recovered.closed` — the entries *this call* closed. The journal is
re-read after every write (`operate::refresh_scope`), on `Ctrl+R` and on `J`;
the second read closes nothing, `interrupted` is empty, and
`interrupted_notice` (update/undo.rs:182–189) returns `None`. Launch, read
"an operation was interrupted: commit "fix" · J to inspect", stage one file —
gone, `J` never pressed. It also blinks off for the duration of every
`Pending` read. Spec: "until `J` is pressed once". The three notice tests are
over a scripted `JournalRead` and never re-read.

Fix: derive the notice from the journal, not from the read that closed it —
`ops` exposes `JournalEntry::is_interrupted()` (status `Aborted` with the
`INTERRUPTED` reason, journal/file.rs:28) and the notice lists in-force
interrupted entries while `!notice_seen`; keep `unreadable` on `JournalState`
across reads (merge, don't replace). Test: two reads, notice survives the
second.

### M2. `RowMark::Interrupted` and the `interrupted` row text are unreachable in the real path

`recover_interrupted` runs before `ctx.journal.entries()` is copied
(jobs/undo.rs:239–250) and `mark_interrupted` rewrites the entry to
`Aborted { reason: "interrupted" }` (graft-ops `journal/file.rs:169`). The
view keys the mark and the summary on `EntryStatus::Pending`
(view/journal.rs:64–65, 117–118, 130), which the browser can therefore never
receive. What the user sees for a recovered entry is `failed: interrupted ·
… · backup saved` — "failed" is the wrong word for a process that died, and
`RowMark::Interrupted`'s `Warning` colour never draws. The acceptance tick
"an interrupted one reads `interrupted`" was earned by
`an_interrupted_entry_says_so_in_the_row_and_in_full_when_expanded`, which
scripts a `Pending` entry by hand.

Fix: the same `is_interrupted()` predicate as M1; `summary` leads with
`interrupted` (no `failed:`) for those; keep the `Pending` arms as a
defensive fallback with a comment saying they should be dead. Test the view
through `read_journal` over a fixture whose journal has a torn pending line.

### M3. A failed restore answers `refresh: None` where the journal (and possibly a ref) did change

`crates/graft/src/jobs/undo.rs:87–101`: `RestoreError::RefMoved | Facts |
Snapshot` → `None`, on the comment "the compare-and-swap refused before any
file was touched". But `restore::execute` (graft-ops restore.rs:116–138)
writes the `Pending` line in `begin`, runs steps in order, breaks at the
first failure and journals `Aborted` — so (a) the journal has a new entry the
browser will not show until some unrelated read, and (b) with two `UpdateRef`
steps the second's `RefMoved` leaves the first ref moved and the graph
showing a tip that is no longer there.

Fix: on every `Undone::Failed` re-read the journal (it is cheap and the entry
is there); for `RefMoved` use `Some(Everything)` unless ops can say it was
the first step — the honest shape is `RestoreError` carrying how many steps
ran, or `Failed { refresh }` computed by ops, which knows.

### M4. The forced undo is not pinned to the entry the drift box named

`ConfirmThen::Undo { redo }` (update/confirm.rs) re-runs `undo::forced`, and
the job re-derives `next_undo`/`next_redo` when it starts
(jobs/undo.rs:296–301). The box asks `Undo "commit X"?`; what runs is
whatever is next at run time. Same for `Enter` in the browser
(update/journal.rs `enter`). Within one process the two agree (the box is
modal; any write puts the journal `Pending`). Across processes — the
KNOWN_GAPS "two Graft processes" row — they need not, and the drift guard is
precisely the rail for "the repository moved underneath you".

Fix (ops API, small): `undo::undo(ctx, force, aim: Option<&EntryId>)`
refusing with `UndoError::Aimed { wanted, found }` when
`next_undo().id != aim`; `Command::Undo(JobId, bool, Option<EntryId>)`; the
box and the browser pass the id they showed. Then the box's question is
literally what runs.

### M5. The id-rewind fix removes two symptoms; the pattern that causes it is now in 13 places

The bug: a caller copies `local_jobs` out, hands a closure over the copy, and
writes the copy back — so any callee touching `state.local_jobs` is rewound.
The fix threads `next_id` into `refresh_key` and `write_job_ended` (good),
but the copy-out/write-back dance is now at keys.rs:37, 52, 120, 129, 137,
148, 166, 444 and mod.rs:87, 192, 260, 488, 506, 533. The regression test
`a_refresh_never_mints_from_the_states_own_counter` asserts only that those
two callees leave `state.local_jobs` untouched; the next helper written the
old way is not caught.

Fix (cause): one copy-out per pass. Either take `local_jobs` out of
`AppState` at the top of `update()` and put it back in `settle`, passing
`&mut impl FnMut() -> JobId` everywhere (no state field to mint from), or one
helper `with_ids(next, |next, next_id| …)` and a lint-style test that
`grep`s `next.local_jobs` outside it. Guard test: over a scripted key/event
sequence, no `LOCAL_JOB_BIT` id appears twice in the union of all
`Command`s emitted.

### M6. ARCHITECTURE.md has no entry for the milestone's headline feature

"Where to start reading" (ARCHITECTURE.md:30–100) names the detail pane, the
Map view, the radar and the hand-over, but nothing for `u`/`U`, the journal
browser or the stash overlay — nine new modules (`app::update::{undo,journal,
stash}`, `app::jobs::{undo,stash}`, `app::view::{journal,stash}`,
`ui::{journal_screen,stash_screen}`). `jobs/undo.rs` is also a second module
in `app` that opens the journal (through ops), while
`crates/graft/MAP.md` `jobs::operate` still reads "the one place `app`
performs a write". PROMPT.md 6.1.

Fix: one bullet — "Changing `u`/`U`, the journal browser or the stash
overlay → `app::update::undo` for the keys and the drift box,
`app::update::journal`/`stash` for the overlays' state, `app::jobs::undo` for
the journal read and the call into `ops::undo`, `ui::journal_screen`/
`stash_screen` for the widgets" — and amend the `jobs::operate` MAP line to
"one of two".

### M7. The e2e journal assertion is a heuristic that checks neither count nor `reverses`

`crates/graft/tests/it/undo_by_key.rs:37–44` counts lines containing
`"pending"` or `"Pending"`, else `lines / 2` rounded up, and line 82 asserts
`>= 2`. The acceptance line reads "journal has 3 entries (discard, undo) with
the undo's `reverses` set". The bytes proof beside it is real; this claim is
not earned, and the helper is exactly the "clever code where plain code would
do" 6.2 names — `Journal::open(&path)?.entries()` is one line.

Fix: `let entries = graft_ops::journal::Journal::open(&journal_path)…
.entries()`; assert `len() == 2`, `entries[1].kind == OperationKind::Undo`,
`entries[1].reverses == Some(entries[0].id.clone())`. Correct the spec line's
"3".

### M8. No `S stash` footer hint; the NOTES' reason for skipping it is wrong

`crates/graft-ui/src/graph_screen/footer.rs:183–190` has `u undo`, `H stash`,
`J journal`. The NOTES say the `S` hint "is already in the footer from M1's
`H stash`" — but `H` hides stash commits (keys.rs `ViewChange::Stashes`); `S`
opens the overlay. Sitting beside `J journal`, `H stash` now reads as the
overlay key, and the user has no footer route to the feature. Spec piece 7:
`u undo  S stash  J journal`.

Fix: `S stash` added; `H stash` → `H hide stash` (or dropped: it is already a
KNOWN_GAPS "not in the footer" row). Regenerate the footer snapshots.

## Minor

1. **Stash preview omits untracked files.** `update/stash.rs:250–256`
   `DiffSpec::Commit { commit, against: None }` is the first parent only; an
   `-u`/`-a` stash keeps untracked files in the third parent, which
   `StashEntry::paths` counts (row says `3 files`) and the preview does not
   list. A second `ListFiles` for `stash^3` when the entry has one, or count
   the preview's rows in the header.
2. **`Esc` mid-load cancels the journal read and immediately re-requests it.**
   `update/journal.rs:176–183` sets `entries = Idle` on close;
   `journal::request` (journal.rs:65) fires at `settle` whenever `Idle`. Also
   drops the marks and the notice until the re-read lands. Keep the last
   `Ready` on close; cancel only the job.
3. **A stash-reflog error kills every badge.** `refs/mod.rs:205` `?`s
   `stash::commits`. Compliant with "never swallow", but a corrupt
   `logs/refs/stash` now removes branch and tag badges too. Consider a
   `RefBadges::stash_error` surfaced on the status line with the refs kept.
4. **`request_drop` returns a `bool` its only caller discards**
   (`update/stash.rs:407`, call at :208). Return `()`.
5. **`let _ = state;` ×4** in `tests/it/stash_by_key.rs:114, 143, 167, 211`.
   Not an error swallow, but the rule bans the spelling; `undo_by_key.rs`
   uses `drop(state)`. Pick one.
6. **Two overlays can be `shown` at once.** `journal_key` opens on `J` while
   `stash.shown` (it checks only its own flag and focus); `S` while the
   browser is open is swallowed. `view::frame` then draws by fixed precedence
   (map > journal > stash, frame.rs:216–218) and the keys go to whichever
   handler runs first. Have each open close the others.
7. **Focus-gained does not re-read the journal or the refs**
   (`update/mod.rs:471–491`), though `refresh_key`'s own comment says another
   Graft appends to the journal, and a stash in another window moves the
   badges. Spec-compliant; a note for M4's watcher.
8. **"backup saved" and "in force" use different predicates.** `view/journal.rs`
   `summary` reads `backup_ref.is_some()`; `ops::in_force` counts an Aborted
   entry by `worktree_before.is_some()`. An entry with one and not the other
   reads "backup saved" without `← u`. One predicate from ops.
9. **MAP/NOTES say `Tab` is never claimed by an overlay**; `prompt_key`
   (`update/stash.rs:319–330`) claims `Tab`/`BackTab` to flip scope — right
   per spec, wrong in the doc. Fix the `update::stash` MAP line.
10. **Pre-epoch clock is silently `0`** in `with_context`
    (`jobs/undo.rs`, `now_unix_ms` … `map_or(0, …)`): a journal entry dated
    1970 rather than an error. Same in `jobs::operate`. Say why it's safe, or
    fail.
11. **The drift box overdraws the footer hints** (`m3-undo-drift.txt` row 20:
    `q quit  F2 status  F3 rende[ Stop ]     Undo  … ve  Space fold`). The
    box has no border, so at 120 columns the buttons sit inside the hint
    text. Judged by the user's eye; noted because the user was told to look
    at the buttons.
12. **No capture shows `← U`, `undone` or `interrupted`.** The acceptance
    line says "`← u` / `← U` marks … visible in the grid"; only `← u` is in
    any of the three journal captures. Add one row to `journal_entries()` in
    `view/tests.rs` (an `Undo` entry) so the 160×48 capture shows all three
    marks.
13. **`test result` totals only** — the fast tier is 1601 tests in ~33 s,
    up from 1503/53.7 s at m3-05 (the m3-05 figure included a slower
    `diff_fixture` run: 14.6 s then, 7.5 s now). No breach; the KNOWN_GAPS
    "watch the trend" row can be closed at the M3 checkpoint if the heavy
    tier compiles again.

## For KNOWN_GAPS

- **BROKEN:** `u`/`U`/`Ctrl+Z` typed in the stash save prompt run an undo/redo (C1). Expiry: before the acceptance walk.
- **BROKEN:** heavy tier and release perf do not compile after this group (C2); every M3 frame budget is therefore **unverified** until fixed. Expiry: before the M3 checkpoint.
- **BROKEN:** the interrupted-entry notice is lost on the first write or `Ctrl+R`, not on `J` (M1); the `interrupted` row mark never draws (M2).
- **UNVERIFIED:** a forced undo across two Graft processes may run against a different entry than the box named (M4; rides on the existing "two processes" row).

## What was verified and how

- Gates as tabled; the heavy-tier failure reproduced twice (heavy test run, release perf run).
- C1 by reading `core_event` → `key_event` → `is_undo_key` → `pane_and_overlay_key` → `stash_key` → `prompt_key`; no test exercises it either way.
- M1/M2 by reading `recover_interrupted`, `Recovered`, `mark_interrupted` and the `INTERRUPTED` constant in `graft-ops`.
- M3 by reading `restore::execute`'s step loop and `begin`.
- Captures: 7 files, 2.9–8.7 KB, all under the 200 KB cap; each shows what its header says except Minor 12; none carries colour (already filed).
- Not verified: anything visual (the user's eye); the release frame budgets (C2).
