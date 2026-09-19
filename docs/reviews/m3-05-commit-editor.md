# Review: M3 group 05 — commit editor, amend, `$EDITOR` suspend/resume

Range `f17e15f..96c8d30` (conductor's doc commits skipped). Reviewer: Fable.
Read order per CLAUDE.md: diff-stat → MAP.md ×3 → diff crate by crate; whole
files opened only to confirm a finding (`terminal.rs` 240–360, `crash.rs`
20–60).

**Counts: 0 Critical · 8 Major · 14 Minor.**

## Gates (run one at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` | **1503 passed, 0 failed, 15 ignored**; 62.96 s wall incl. compile, 53.7 s test time; peak RSS **360 MB** |
| `cargo test --features heavy-tests -- --test-threads=2` | **1534 passed, 0 failed, 15 ignored**; 341 s wall; peak RSS **802 MB** |
| release perf (`frames:: heavy::budgets_hold`, `--test-threads=1`) | **12/13 first run, 13/13 on rerun** — see Minor 13 |
| `cargo bench -p graft-graph --bench hot_paths` (1 s warm-up, 3 s measure) | `StreamingLayout::advance/one chunk` 801 µs · `rasterize/one strip` 625 µs · `StripCache::get/hit` 1.70 µs · `StripCache::get/miss` 627 µs. No regress/improve flags |
| fuzzing | no targets exist |
| `target/` | **22 GB** |

Five slowest tests (fast tier): `diff_fixture::parse_unified_matches_numstat_for_every_shape` 14.56 s ·
`layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 11.58 s ·
`corpus::two_builds_produce_the_same_object_ids` 5.36 s ·
`diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 3.05 s ·
`corpus::a_second_build_in_the_same_place_is_a_no_op` 1.95 s.

Perf run numbers (recorded, not asserted): Unicode tier idle_prefetch p99 ≤ 1 µs, max ≤ 19 µs
across 5 configurations; Pixel tier p99 6.1–6.2 ms, max 7.5–8.2 ms; idle-wake first key 1.3–11.6 ms.

## Are the ticks honest?

Yes, with two literal slips and one false claim in the NOTES. Every one of the 39 test names in
"Evidence for each tick" exists (`grep -rl "fn <name>"`, none missing); `update::commit_editor::tests`
has 28 tests as stated; the real-runner test asserts `git log -1`, `rev-list --count`, the
journal's `refs_before != refs_after`, and that the editor and draft close on success. Markedly
better than group 04.

- Tick 2 says "`Tab` ×4 reaches `CommitButton`"; the test walks Subject→Body→Amend→Button in
  **3** Tabs (the spec's wording, not the builder's — fine).
- Tick 7 is `[x]` while its own evidence says the 8 KB line "was not met and cannot be". The box
  should be `[ ]` with the note, or the line amended. **I agree with the arithmetic:** 48 rows ×
  (160 chars + 2 quotes + newline) = 7,824 B before the preamble; the file is 9,289 B. The spec's
  8 KB cap is impossible at 160×48 and CLAUDE.md's 200 KB cap is the one that binds. **Spec
  defect, worth recording** so the acceptance group does not inherit it.
- NOTES: "No footer hint for `e` … The key works and is tested." **No test references
  `OpenExternalEditor` or `edit_target`** anywhere under `crates/graft/src` or `tests/`. See Major 2.

## Suspend/resume verdict

The evidence is real and covers the right properties; the real-terminal walk stays unverified
(already filed).

- **`Gate` really stops the reader.** `gate.rs`: the flag is checked *before* the read
  (`mux.rs`: `Ok(Ready::Input) if gate.is_paused() => continue`), the reader parks on a condvar,
  and `pause` waits for `parked`. `a_paused_mux_leaves_the_input_for_whoever_has_the_terminal`
  reads the typed bytes back from a **second handle on the same socket** while paused — the
  property that matters, not "pause was called".
- **Every mode is restored.** `suspend_undoes_every_mode_and_resume_puts_the_same_ones_back`
  pins the byte order: kitty pop, cursor show, focus off, paste off, five mouse modes off, alt
  screen off, raw off — alt screen last so the editor draws over the shell. `resume` re-applies
  exactly the recorded set (kitty only if it was on: `resume_does_not_turn_on_a_mode_suspend_did_not_find`).
- **Non-zero / unstartable editor still restores.** `Handover::open` (outside.rs:113–130) inspects
  the exit status only *after* `run_outside_terminal` returns, so re-entry does not depend on the
  editor's exit. The old text is kept and the reason is worded.
- **Panic path ends clean — but not for the reason the code says.** On a panic inside `f` the hook
  restores (idempotent no-op, already suspended), announces to stderr, then `Reenter::drop`
  re-applies raw mode + alt screen, then unwinding reaches `TerminalSession::Drop`
  (terminal.rs:248) which restores again. The terminal is clean because of `Drop`; the guard's
  re-entry is redundant on that path. See Minor 1.
- **Gaps:** `pause` returns `Ok(())` after a 250 ms park timeout even when the reader is alive
  (Major 6); `reenter` resumes the mux before raw mode is back (Minor 1).

## Critical

None.

## Major

**1. A hook-rejected commit's reason is invisible while the editor stays open.**
`crates/graft/src/view/commit_editor.rs:18` builds `CommitEditorView` with no status field
(`graft-ui/src/commit_editor.rs:132`); `view/frame.rs::draw_commit_editor` renders only the editor
screen (and the pane). `Status::WriteFailed` is only drawn as the **graph screen's** `problem`
(`view/rows.rs`, `graph_screen/footer.rs`). So after `Operated::Failed` the editor returns to idle,
title back to `COMMIT ── …`, and nothing on screen says why. The message is not lost, but the
design note's "Hook stderr appears in the status line and stays till `Esc`" is not met while the
editor is up — the user has to `Esc` (closing the editor) to read it.
`a_commit_that_failed_keeps_the_editor_and_the_message` checks state only. **Fix:** draw
`state.status`'s text on the title row or beside the button, the way `busy` is; add a `view`
test that the refused text appears in the drawn grid.

**2. `e` opens the wrong file when Graft is not launched from the worktree root.**
`update/operate.rs:222` `edit_target` returns `selected_path` — the raw porcelain path, relative
to the worktree root — and `outside.rs:113–116` spawns `editor_command(...).status()` with **no
`current_dir`**. `graft ~/code/api` from `~` then opens `~/src/main.rs`: an empty new buffer in
vim, "file not found" in code/zed. Saving creates a stray file. No test covers `e` at all
(contrary to the NOTES). **Fix:** thread the worktree root into `Handover` and set
`command.current_dir(workdir)` (or join in `edit_target`); add an update test for
`e` → `OpenExternalEditor { path, line }` and an `editor_command` test that the path is absolute
or the cwd is the worktree.

**3. Bracketed paste is dropped — spec piece 1 stubbed.**
`update/mod.rs:410`: `Event::Paste(_) | … => {}` regardless of the editor. `TextField::insert_str`
and `TextArea::insert_str` have **no non-test callers**. Graft turns bracketed paste *on*, so
pasting a commit message into the editor does nothing at all. Not in KNOWN_GAPS. **Fix:** route
`Event::Paste(text)` to the focused field (`insert_str`), with the subject dropping newlines as
designed; test it. Until then: KNOWN_GAPS row marked **broken**.

**4. `Esc` during `committing…` SIGKILLs `git commit` — lock-file residue unverified.**
`update/commit_editor.rs:372` emits `CancelJob`; `jobs/runner.rs:606` cancels; `graft-git`'s
runner kills the child (`run/exec.rs:107`, `Child::kill` = SIGKILL). Git's lockfile cleanup runs
on SIGTERM/atexit, not SIGKILL, so a stale `.git/index.lock` after an `Esc` mid-commit is
plausible and would make every later stage fail until the user removes it by hand — a "time I
left". Nothing tests it. The spec asked for `Esc` → cancel; the cause is the runner's kill
policy (group 03). **Fix:** SIGTERM with a short grace before SIGKILL in the runner, or do not
offer cancel for a `Commit`/`Amend` and say so in the title. Needs the user's call;
KNOWN_GAPS **unverified** meanwhile.

**5. `editor_command` breaks on a quoted or shell-form `GIT_EDITOR`.**
`outside.rs:312` `split_whitespace`. Git runs an editor string containing quotes or shell
metacharacters through `sh -c`; a common macOS `core.editor` such as
`'/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl' -w` fails here with
"could not run '/Applications/Sublime". The user's own `zed --wait` / `code --wait` are
unaffected. **Fix:** when the string contains any of `'"$\`;&|<>()` run
`sh -c '<editor> "$@"' sh <args>` (git's `prepare_shell_cmd` rule); test with a quoted path.

**6. `Gate::pause` reports success after a timeout it cannot distinguish from a live reader.**
`gate.rs:95`: on the 250 ms park timeout it `break`s and returns `Ok(())`. The comment assumes the
reader has exited; a live reader on a loaded machine yields the exact race the gate exists to
prevent, and the caller cannot tell. A swallowed condition (CLAUDE.md "Never swallow an error").
**Fix:** have the reader set a `finished` flag when its loop ends; on timeout return `Ok` only if
`finished`, else `Err` so `run_outside_terminal` refuses the hand-over.

**7. `Status::WriteFailed` names a *write* but means *sticky until `Esc`*; MAP.md not updated.**
Three read failures use it: HEAD message for amend (`update/commit_editor.rs:271`), `$EDITOR`
failure (`:337`), `HandedOver(Err)` (`update/mod.rs:199`). The choice (survive the refresh walk) is
right; the name misleads the next reader into thinking a read is a write. `crates/graft/MAP.md:161`
still lists `Status` as ending at `Failed(message)` — a changed public item without its MAP line
(CLAUDE.md MAP rule). **Fix:** rename to something like `Status::Sticky(String)` / `NeedsDismiss`,
or document "any failure that must outlive a refresh"; update MAP.md.

**8. Architecture: ARCHITECTURE.md silent on the loop-carried handover; a bare `git` spawn is back.**
`event_loop::outside` and the rule "three commands are carried out by the frame loop, never a job"
appear in code and MAP.md but not in ARCHITECTURE.md (PROMPT.md 6.1). `outside.rs:147` spawns
`Command::new("git")` directly — the m3-01 closed gap moved every git spawn onto `run` (env,
`--literal-pathspecs`, cancel, output cap); this one is invisible to the seam guard (it only sees
`GitInvocation`) and runs on the UI thread. **Fix:** resolve `git var GIT_EDITOR` as a job at
editor open (beside the template read), store it in `AppState`, pass it in
`EditCommitMessage { editor, … }`; document the handover pattern in ARCHITECTURE.md.

## Minor

1. **Guard rationale inverted; resume order.** `outside.rs:216–219` says the guard exists so a
   panic does not leave the alt screen off — but the hook has *already* printed to the scrollback
   by then, and `TerminalSession::Drop` is what leaves the terminal clean. `reenter`
   (`outside.rs:272–276`) resumes the mux **before** `session.resume` re-enables raw mode; a key in
   that window is cooked and echoed. Swap the two lines; fix the comment.
2. `running.rs:233` `unreachable!` and `update/mod.rs:203` `debug_assert!(false)` in non-test
   code: narrow the type (a `Handover` enum carved out of `Command`) so the compiler proves it.
   `JobId::new(0)` as a sentinel (`running.rs:218–230`) — harmless since `HandedOver` ignores the
   id, but say so or use `Option<JobId>`.
3. Two phrasings for one state: `"working tree not read yet"` (`update/commit_editor.rs:173`) vs
   `"reading the working tree…"` (`view/commit_editor.rs:92`).
4. 80×24 capture preamble contradicts itself: `view/tests.rs:2248` "the detail pane is not drawn"
   and `:2260` "the detail pane is still there beside the editor" share one header for both sizes.
5. Tick 7 checked against evidence saying it was not met (see "Are the ticks honest?"). Untick or
   amend; record the 8 KB line as a spec defect.
6. `head_short` (`update/commit_editor.rs:278`) repeats the 7-char truncation done in `keys.rs`
   and `context_menu.rs`; one helper.
7. `Ctrl+Z` inside the *open* editor is swallowed by the modal (`control_key` `_ => {}`,
   `:446`) even with `suspend_on_ctrl_z` off; `ctrl_z_suspends_only_when_the_config_says_so` runs
   with the editor closed, so tick 6's "must not be swallowed" is untested for the open case.
   Probably intended (group 06's undo should not fire mid-message); say so in the doc comment.
8. `strip_comments` (`update/commit_editor.rs:348`) hardcodes `#`; git honours
   `core.commentChar`/`commentString`. A user with `commentChar = ;` loses real `#` lines from a
   `$EDITOR` round-trip.
9. `git var GIT_EDITOR` + `fs::write`/`read_to_string` on the UI thread (`outside.rs:92–96, 147`):
   acceptable because the editor blocks next anyway; folds into Major 8's fix.
10. In-progress refusal reads `state.in_progress` from the *last* status read; a merge begun in
    another terminal between reads passes the preview. Whether `Operation::check` (group 02) also
    refuses is not asserted here — **unverified**; the focus-gain re-read narrows the window.
11. `Space` on `[ Commit ]` commits (spec: `Enter`); fine, but the footer says `⌃⏎/Commit`.
12. Fast-tier trend: `parse_unified_matches_numstat_for_every_shape` 7.76 s (M2 row, "watch,
    expiry M3") → **14.56 s** now. Not this group's; the M3 row's expiry has arrived.
13. Release perf: `scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier`
    failed once — keys 292–295 at 70.8/54.6/38.3/22.0 ms, every "own frame" < 0.6 ms, i.e. one
    ~70 ms stall on the UI thread then the queue draining — and passed alone (13/13). Same family
    as the KNOWN_GAPS "unexplained 17–18 ms release key"; larger. Nothing in this diff runs on the
    graph path. **Unverified/intermittent**, not a regression claim.
14. External `SIGTSTP` (`kill -TSTP` from another shell) is still unhandled — ADR-0002's "SIGTSTP
    arriving from elsewhere" consequence: Graft would stop in raw mode + alt screen. Not in this
    spec's scope; pre-existing. KNOWN_GAPS row, **broken by omission**, low likelihood (raw mode
    means the user's own `Ctrl+Z` is a key, never a signal).

## Compaction seam

The builder compacted between `a5c6ca9` (handover) and the captures. Post-seam commits carried the
wiring, config, real-runner test, MAP.md and the `[keys]` schema fix. What is missing and would
have belonged to piece 5: paste routing (Major 3), an `e` test (Major 2), the status text on the
editor screen (Major 1), MAP.md's `Status` line (Major 7), ARCHITECTURE.md (Major 8). I cannot
prove which were lost to the compaction and which were never planned, but all five are things a
spec-following piece 5 would have done.

## Checked and fine

`Tab`→`[ Commit ]`→`Enter` works pure and over real git (`press_the_button`); `Ctrl+Enter` still
works. Refusal names `kind.continue_command()` and `in_progress` errors propagate
(`working_tree.rs`). `WriteFailed` survives `JobFinished`; a read `Failed` still clears as before;
`Esc` clears both before anything else (`update/keys.rs:76`). All repository text reaching the
widget goes through `sanitize_line`; typed control characters are refused at the field. No
`unwrap`/`expect` outside tests. Crate rules hold (`graft-ui` has no `graft` dependency; `update`
stays pure). Captures: 4 files, 3.4–9.3 KB, all under 200 KB, `┊` guides and `26/50` visible;
appearance not judged here.

## For KNOWN_GAPS (not already filed)

- **Broken:** bracketed paste into the commit editor does nothing (Major 3).
- **Broken:** `e` opens paths relative to Graft's cwd, not the worktree (Major 2).
- **Broken:** hook-rejection text not drawn while the editor is open (Major 1).
- **Unverified:** `Esc` mid-commit SIGKILLs git; `index.lock` residue untested (Major 4).
- **Unverified:** `Operation::check` refuses during merge/rebase independently of the preview (Minor 10).
- **Unverified/intermittent:** one 70 ms release-run stall (Minor 13).
- **Spec defect:** the 8 KB capture cap is impossible at 160×48 (7,824 B bare).
