# m1-the-graph/77 — `.unwrap_or(())` swallows stderr write errors in event_loop startup

Status: ready
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
- [ ] Both sites handled or justified in a comment; test covers the failing writer.
- [ ] Report: cause or symptom; what it costs the user; other sites found.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens.
