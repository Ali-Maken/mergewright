# Review: M8 stop/quit 13e–13k and look polish 17–17c

Fixed hash: `f800e8ce` (`git rev-parse main` at start). Each merge diffed against its first parent:
13e `32d13544`, 13f `04eb1eab`, 13g `55aa28f9`, 13h `b19b74d7`, 13j `93699bd8`, 13k `07f7f819`,
17 `e2781b5a`, 17b `7f840f07`, 17c `fd0b66c8`. Tickets: `.scratch/m8-polish-and-ship/issues/`. ADR-0060 for 17c.
Method: `code-review` skill, both axes (Standards / Spec) done inline by one reader, with no sub-agents. Each finding is
tagged with its axis.

## What was and was not run

- **No cargo** (conductor: a builder runs beside this review). None of the gates, the heavy tier, the release perf
  run, fuzzing or benchmarks was run by me. Each is **unverified** here. These are the builders' claims, not mine:
  - 13k: workspace nextest 5902 passed / 10 skipped.
  - 13j: 5888/5888.
  - 13f: heavy clippy and `nextest --features heavy-tests --no-run` rc 0.
  - 13g: heavy test run alone, 60.08 s.
- The fast-suite time, the 5 slowest tests, peak memory and the size of `target/` were **not measured**. `du` on
  `target/` gave no answer within 60 s.
- No suspected false tick needed the one allowed targeted run, so it was not used.

## Tick spot-check (CLAUDE.md gate)

| Ticket | Ticked | Checked | Held | Which |
|---|---|---|---|---|
| 13e | 7 | 6 | 6 | splits (`git show --stat -M` of 21f2db0c/30e75ce2), SIGSTOP, Minor 2, Minor 4, Minor 8, R2 comment (`apply/close.rs:348-353`) |
| 13f | 5 | 4 | 4 | `pool.rs` 325 lines; Minor 7 (`jobs/tests.rs` + `quit/tests.rs`); R4 grep (`left_at(` non-test only `operate.rs:353`); Minor 5 grep (no old string left) |
| 13g | 8 | 6 | 6 | never-101, quick hook, second hang-up (`hangup.rs:430-556`), EOF/EIO (`event_loop/tests.rs`), Minor 6 pair (`exit_wait.rs` tests), capture (`check_shot`, banner asserted `hangup.rs:593`) |
| 13j | 8 | 4 | 4 | drop-waits (`remote/tests.rs:419`), chunked (:457, asserts 200 + `refs/heads/main` + counters), 100-continue (:484), idle client (:520); Cargo.lock has no tiny_http/ascii/chunked_transfer/httpdate |
| 17 | 5 | 3 | 3 | item 3 (`graph_column/tests.rs:206`), 5 (`startup.rs` + `config/tests.rs`), 7 (`detail_pane/tests.rs:1698`) |
| 17b | 7 | 4 | 4 | refresh (`fold_count/tests.rs:51`), failure once (:71), blame tail (`blame/tests.rs:641`), blame shots: `git diff 428091d9^ 7f840f07 -- docs/shots/m7/…` empty. They differ at HEAD only because of 11d (`f9cbb1a2`) |
| 17c | 6 | 3 | 3 | `graph_tier` (`view/tests/pixel_preview.rs:65`), hit-test (:117), gfx (:207) |

**30 checked, 30 held. No false tick.** Open boxes are honestly open: 13j load loop, 17/17b/17c contact sheet,
17's items 2 and 6 (moved to 17b/17c).

## Rule 2 (retry / sleep / longer timeout presented as a cause fix)

None found.
- 13h removed a real-time sleep from the test clock (`event_loop/tests.rs`, `ScriptedWakes::now`).
- 13g's debugger replaced a fixed 2 s signal timing with a release file and added no timeout.
- 13j took no timeout up.
- 13k changes **where** the 2 s grace starts, not its length. The cost is that `STOP_WITHIN` stops being a bound
  under a slow `ps`. It is stated (`run.rs` doc, MAP) and filed (`M8/stop-group-read-unbounded`). The link to the
  original flake is inferred and filed unverified (`M8/stop-now-signal-flaky`).

---

## Critical

None.

## Major

### M1. Look polish 17/17b/17c shipped visible features with no prior-art study (pipeline step 2) [Spec]
- **Where:** `docs/design-notes/m8-polish-and-ship.md` "Prior art" (:35-99) and "Journeys" (:100-) have no row for the
  folded-merge `+N`, the off-thread fade by hue, the whitespace-mode title tail, `ui.overview`, or Pixel ghosts.
  The 17 ticket cites only the Need row "Readable in his theme". No file under `docs/research/` covers any of them
  (`grep -il "fold\|whitespace\|fade" docs/research/*.md` finds none of these topics). ADR-0060 lists rejected
  options but cites no tool.
- **Why it matters:** CLAUDE.md "The feature pipeline": a feature with no prior-art study is Major. Each of these
  has a well-known precedent we never looked at:
  - how GitKraken, Sublime Merge, JetBrains and lazygit mark a collapsed merge;
  - how delta, VS Code and JetBrains show "ignore whitespace" is on;
  - how other graph UIs fade off-path lanes.
  The `+N` and the tail are now the user's daily view.
- **Fix:** a `researcher` pass `docs/research/look-polish.md` (fold mark, whitespace-mode indicator, lane fade).
  Then a Prior art row (take/reject/invent) and a one-line journey for `Space`-to-`+N` in the design note. Any
  "reject" in it becomes a follow-up ticket.

### M2. New `unsafe` in graft's integration tests without an ADR (13g) [Standards]
- **Where:** `crates/graft/tests/it/quit_waits/hangup.rs:143-163`, `session_leader`:
  `#[allow(unsafe_code)]` + `unsafe { command.pre_exec(... close / setsid / ioctl(TIOCSCTTY) ...) }`.
- **Why it matters:** PROMPT.md 6.2 says "No `unsafe` … Any exception needs an ADR." `Cargo.toml:41-43` names
  exactly two exceptions, `graft-term` (ADR-0013) and `graft-fixtures` (ADR-0015). This is a third, approved by
  nobody. The builder disclosed it in NOTES, and the merge took it anyway.
- **Fix:** an ADR, or an amendment to ADR-0015, that allows exactly this `pre_exec` with its SAFETY reasoning. Move
  the helper into `graft-fixtures`, which already holds the one sanctioned test-side `unsafe` site, and add a line
  to the `Cargo.toml` comment. Do not just keep the `allow`.

### M3. Files grow past PROMPT 6.2's ~400 lines with no size row [Standards]
- `crates/graft-fixtures/src/remote/http.rs` went from **514 to 942 production lines** in 13j (`#[cfg(test)]` at
  :943). `Drop` alone is :401-468, 67 lines, over the ~60-line function rule. There is no KNOWN_GAPS row.
- `crates/graft-ui/src/blame.rs` went from 550 to 601 (17, 17b), with no row.
- `crates/graft-ui/src/graph_column/mod.rs` went from 420 to 443 (17), with no row.
- `crates/graft/src/event_loop/startup.rs` went from 741 to 801 (13g, then 17). Row `M8/look-file-sizes` still says
  704.
- `crates/graft/tests/it/quit_waits/hangup.rs` is 674 lines and new (13g).
- **Why it matters:** PROMPT 6.2 makes size a hard rule. 13e/13f just spent a ticket splitting files that crossed
  400, and these merges add more of the same.
- **Fix:**
  - Split `http.rs` by concern into `remote/http/{listener,request,cgi}.rs`: accept loop and `Drop`, framing
    (`read_request`/`read_chunked`/`write_response`), and the `http-backend` bridge.
  - Split `hangup.rs`'s `Session`/pty harness (:1-300) into `quit_waits/pty_session.rs`.
  - File rows for `blame.rs` and `graph_column/mod.rs`, and refresh `M8/look-file-sizes`.

## Minor

### m1. 13k: under a slow `ps`, the group is read back-to-back [Standards]
- **Where:** `crates/graft-git/src/run/stop.rs:226,234`. `next_read = now + LIST_EVERY` uses the `now` taken
  **before** `helpers.pass` reads the group, so a read that takes ≥ 250 ms makes the next read start at once. The
  same holds at :262/:266 in `after_git_ended`.
- **Effect:** on a loaded Mac (1.9–2.3 s per `ps`, 13k NOTES) the stop runs `ps` continuously, adding load exactly
  when the machine is already slow.
- **The regression test is slower than claimed:** `a_slow_read_of_the_group_does_not_spend_gits_grace` (:507-532)
  sleeps 2.1 s on **every** read (:520-523). By reading the loop it does three reads, about 6.3 s. 13k NOTES says
  "about 2.3 s". Not run (no cargo), so the figure is unverified.
- **Fix:** take `next_read` after the read returns. Check the test's time against the 7.1 fast-tier budget.

### m2. 13k's new test leaves `sleep 600` unguarded [Standards]
- **Where:** `stop.rs:510`, `sleep 600 & …`, with no `Reap` guard.
- **Why it matters:** 13e's Minor 8 rule is "every stop test that runs `sleep 600` holds a guard". If
  `wait_until_ready` or `hooks_first` panics before the stop, the sleep outlives the run. 13e's grep check is now
  out of date.
- **Fix:** write the pid to a file and hold a `Reap::hook`, or kill the group in a `Drop`.

### m3. 13g's hang-up exemptions match the exit, not the error [Standards]
- **Where:** `crates/graft/src/event_loop/startup.rs:531` turns **any** restore error into `Ok` when the exit is
  `Signal(SIGHUP)`. `:482` drops **any** stalled-report error on the same condition.
- **Why it matters:** a `kill -HUP` from outside, with the terminal alive and a restore failing for another reason,
  now exits with no word and possibly a raw terminal. Rule 3: the comment's "nothing left to restore" is true only
  for `EIO`.
- **Fix:** add `if input::hung_up(&error)`, or `raw_os_error() == Some(EIO)`, to both guards.

### m4. 13g: the waiting line says "again" on exits that sent no signal [Spec, textual]
- **Where:** Minor 6 now prints `waiting_line` on any exit with a write running (`exit_wait.rs:96-97`). Its words
  are still "send the signal **again** to stop it half-way" (`exit_wait.rs:186`), and the test asserts them for
  `Ending::Exited` (`a_fatal_exit_with_a_write_running_prints_the_waiting_line`).
- **Why it matters:** on a fatal exit no signal was sent.
- **Fix:** give the line per `Ending`. For `Exited`: "… · Ctrl+C stops it half-way".

### m5. 13j: the idle-client test cannot tell `Drop`'s shutdown from the 1 s read timeout [Standards]
- **Where:** `crates/graft-fixtures/src/remote/tests.rs:534-537` allows `drop` 2 s. The handler's 1 s
  `READ_TIMEOUT` (`http.rs:42`, `:383`) alone ends an idle client inside that, so the test passes even if step 3's
  `shut` (`http.rs:419-421`) did nothing.
- **Fix:** bound it under `READ_TIMEOUT`, for example 500 ms, so the test pins the shutdown path.

### m6. 17b: blame's whitespace default is written down twice [Standards: Duplicated Code]
- **Where:** `crates/graft-ui/src/blame.rs:232` passes a literal `true` as blame's default. The truth is
  `crates/graft-git/src/blame.rs:77` (`BlameOptions::default`).
- **Why it matters:** if either one changes, the title tail names the wrong state.
- **Fix:** carry `default_ignored` on `BlameView`, filled by graft from `BlameOptions::default()`.

### m7. 17b: `whitespace_tail` lives in `blame.rs` but is used by detail and history [Standards: Feature Envy]
- **Where:** `detail_pane/header.rs` (`detail_whitespace_tail`, :350-351) and `history.rs:299-300` call
  `crate::blame::whitespace_tail` and `crate::blame::put_whitespace_tail`.
- **Why it matters:** a reader looking for the detail pane's title rule will not look in blame.
- **Fix:** move both functions and the two constants to `graft-ui/src/whitespace_tail.rs`.

### m8. M8's captures are in two places, and the contact sheet will miss one of them [Spec]
- **Where:**
  - `scripts/contact-sheet.sh` reads `<shots-dir>/<milestone>-*.txt`.
  - 55 captures follow that pattern as `docs/shots/m8-*.txt`.
  - 17 sit under `docs/shots/m8/` with no `m8-` prefix, including all four of 17/17b/17c's captures
    (`m8/polish-{whitespace,fold,blame-whitespace-counted,ghost-pixel}-…`).
- **Why it matters:** no single invocation gathers both sets (ADR-0037: one sheet). 17's ticket also asked for
  `docs/shots/m8-polish-whitespace-120x36.txt`, and the tick names the `m8/` path.
- **Fix:** pick one convention before the acceptance group assembles the sheet.

### m9. KNOWN_GAPS rows these merges fixed are still open [textual]
- `KNOWN_GAPS.md:156` "A folded merge looks like a plain commit": fixed by 17b.
- `:983` `M7/21c-ws`: fixed by 17/17b.
- `:1149` `M8/hangup-exit-101-orphans-git`: fixed by 13g.
- `:1219` `M8/stop-files-over-400`: the splits are done. `exec.rs` 242, `apply.rs` 342 (children ≤ 362),
  `resume.rs` 306, `pool.rs` 325.
- **Fix:** move each to `docs/gaps/closed.md`, naming the fixing ticket.

### m10. 17b's refresh test does not show that a refresh happened [Standards]
- **Where:** `crates/graft/src/update/fold_count/tests.rs:56-60` sends `Ctrl+R` and asserts no `CountFolded`.
- **Why it matters:** it would pass just as well if `Ctrl+R` did nothing in that state.
- **Fix:** also assert the refresh's own command, for example a `ShowRows` or a re-read command, in `commands`.

### m11. 17c: the docs say less than, or other than, what was built [textual]
- **"Ghost rows":** the ticket, ADR-0060 decision 1 and §1.7 say Unicode "while a preview **with ghost rows** is
  open". The built condition is any landed preview: `rows::preview_drawn` (`view/rows.rs:177-182`) only checks
  `!rows.is_empty()`. That is right, since the picture replaces the live rows either way, but the docs say less.
- **`gfx` field:** ADR-0060 decision 2 says the header's `gfx` field keeps reading `tier()`. It reads
  `tier_in_force` through `pixel_tier_on_graph` (`view/frame.rs:110`, `:224`).
- **Fix:** one wording pass on ADR-0060 and §1.7.

## Questions (no defect claimed)

- **Q1. 13g: can one window close skip the 60 s wait?** This gives `M8/13g-double-sighup` the lines it lacked.
  - When the dead-terminal read (`EIO`) ends the loop before any `SIGHUP` is counted, `again_after_one` sets
    `at = max(seen, 1) = 1` (`event_loop/signals.rs:98-99`, called from `exit_wait.rs:92`).
  - A kernel `SIGHUP` plus a shell-forwarded `SIGHUP` then make `seen = 2 > 1` (`signals.rs:106`). The write is
    stopped at once instead of being given 60 s.
  - Whether his shells forward `SIGHUP` to a foreground job on window close is unverified.
- **Q2. 13j: does the 1 s per-read timeout add a new way for load to fail the tests?** `http.rs:383` sets it. A git
  client that pauses more than 1 s mid-body on a loaded machine gets no answer (`handle`, `http.rs:732-737`). The
  builder flagged this, unverified. `Drop` already shuts every stream (`:419-421`). If shutdown is shown to wake a
  blocked read on macOS, the timeout buys nothing for teardown and could be raised. That is a spec change, through
  the conductor.
- **Q3. 17b (builder's open question, unfiled):** should `crates/graft/tests/it/blame_place.rs:160` go back from
  `contains` to `ends_with`? 17 loosened it only because of the blame tail, and that tail is now gone by default.

## Checked and fine (short)

- **13e:** the moves are moves. `group_stop` sends `SIGCONT` (`stop.rs:147-148`). One helper path,
  `after_git_ended`. `kill_reaped_group` matches `ESRCH` and comments `EPERM` (:375-385).
- **13f:** `Forced::started` is set from `jobs.started` at force time (`forced.rs:61`, `:84`). The exit line
  branches on it in both places (`exit_wait.rs`).
- **13g:**
  - The cause is fixed at its line: `put_away` (`startup.rs:374-381`) plus `RowClear::cursor_given_up`. No panic
    hook, no `catch_unwind`.
  - EOF/`EIO` maps to `Exit::Signal(SIGHUP)` in both loops (`running/turn.rs`, `places.rs`). Headless is
    untouched.
- **13h:** both fixes are causes. They are the test clock and the places screen's frames now wrapped in DEC 2026
  `draw_whole` (`places.rs`), with the harness `ended` matching. No sleep was added.
- **17b:**
  - `Command`/`Product` routes are exhaustive.
  - The failure text is sanitised on draw (`view/status.rs:47`).
  - No u16 underflow in `draw_subject`/`subject_truncated`: `drawn_fold` guarantees `fold_cells < width`.
  - The count runs on the pool, never on the UI thread.
- **17c:** chrome stays on `tier_in_force`. Hide versus free stays on the tier in force (`event_loop/graphics.rs:153`).
  Prefetch and raster are gated by `draws_pixels`.
- **Architecture:** no new crate edge. graft's `nix` dev-dependency gains only the `poll` feature. `tiny_http` is
  gone from `Cargo.lock` and every doc.
