# m1-the-graph/77 — `.unwrap_or(())` swallows stderr write errors in event_loop startup

Status: done
Build with: **Sonnet** (medium) — two call sites and a decision recorded in a comment.
Blocked by: 76b

Found by 71 (moved verbatim, pure-move ticket): `crates/graft/src/event_loop/startup.rs` `finish()` and `report_stalled_jobs()` drop stderr write failures with `.unwrap_or(())`. CLAUDE.md: never swallow an error.

## Read first
- CLAUDE.md "Never swallow an error".
- `crates/graft/MAP.md`; `crates/graft/src/event_loop/startup.rs` those two functions only.

## Scope
In: for each site decide: propagate (preferably — exit status reflects a failed report), or, if the only failure is stderr itself being closed (nowhere to report), keep ignoring it with an explicit match and a comment saying why it's safe. A test where a writer fails shows the chosen behaviour. Grep the workspace for other `unwrap_or(())` / `let _ =` on writes and list them in the report (don't fix beyond these two).
Out: anything else.

## Acceptance
- [x] Both sites handled or justified in a comment; test covers the failing writer.
- [x] Report: cause or symptom; what it costs the user; other sites found.

## Notes
- `report_stalled_jobs` now returns `io::Result<()>`; its write failure is
  folded into `run`'s return so it turns into `ExitCode::FAILURE` (propagate).
- `finish`'s restore-failure write stays best-effort: `loop_error` is
  returned regardless, so a failed write only loses an extra detail, not the
  failure itself (justified, explicit `match`, comment explains why).
- Both split into `_to(writer, ...)` variants for testability; tests in
  `startup.rs` use a `FailingWriter`.
- Other swallow sites found, not fixed (out of scope): `crash.rs:122`,
  `main.rs:82`, `event_loop/redetect.rs:85`, `jobs/runner.rs:91,227`
  (`.unwrap_or(())`); `terminal.rs:326`, `crash.rs:156` (`let _ = write...`).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens.
