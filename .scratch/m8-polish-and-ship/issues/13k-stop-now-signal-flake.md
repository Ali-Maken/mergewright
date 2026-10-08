# Stop-now flake: a slow `ps` used up git's grace in a hook-first stop

Status: done
Build with: debugger (Opus).
**Crate set: graft-git only.** Files: `crates/graft-git/src/run/stop.rs`, `crates/graft-git/src/run.rs` (doc),
`crates/graft-git/MAP.md`.
Source: KNOWN_GAPS M8/stop-now-signal-flaky. `quit_waits::signal::stop_now_after_the_real_offer_restores_then_says_where_git_stopped`
failed once at 20.6 s in a loaded full run and passed alone in 12 s. No failure output was kept.

## NOTES (debugger, 2026-10-08)

No compaction. No scout: one test and the stop code under it, read directly.

**Not the 13g cause.** 13g's test sent its second signal at a fixed time. This test sends no signal, sleeps for
no fixed time, and waits only on screens and the exit. Its siblings are not a factor either.

### The loop

- **Natural.** Ran the test 36 times, 4 lanes under `taskpolicy -b`, beside the other builder (load 10–21). **0
  failed.** A loaded pass took 17–19 s here, so the 20.6 s does not point to any one step.
- **Probe** (`[DEBUG-13k]`, removed): timestamps at `force_stop`, the stop's start and end, each read of git's
  process group, the after-stop reads, and `wait_for_forced`.
  - In one round (4 of 24 runs), the first `group::members` call took **1.89–2.32 s**. On macOS that call is one
    `/bin/ps`. 2 s is the whole `HOOKS_GRACE`.
  - Those runs passed only because git exited inside the next 2 ms poll.
- **Forced red.** Two changes, both temporary:
  1. `members` sleeps 2.1 s, standing in for the slow `ps`.
  2. The hook traps `TERM` and exits 0.3 s later, standing in for git being slow to write its pause under load.
  - Result: the real test failed **3 of 3** at `signal.rs:201` (`text.contains(PAUSED)`). The exit line was
    `graft: stopped merge main into feature after 10 s, before git could pause it · u in graft puts your files
    back`. The probe showed the grace was spent, then `group_stop`, and git ended by `SIGTERM`
    (`git_signalled: true`).
  - Control: the trapping hook alone, with no delay, passed 2 of 2.

### Cause: the product, `crates/graft-git/src/run/stop.rs` (as at HEAD 7fd043b0)

- `hooks_first` started git's grace before it had read the group (`let start = Instant::now();`, :211). It then
  shot git once `now >= start + graces.hooks` (:220).
- But git's time to pause starts only when its hook is ended. The hook is ended in `Helpers::pass`, **after** the
  `ps`. So a slow `ps` used up git's grace before the hook had even been signalled.
- git then got `SIGTERM` while it was still writing `MERGE_HEAD`. The merge he asked to leave paused was not
  paused. It ended as `BeforePause`: the entry `Aborted`, `u` to put the files back.
- The doc already said "leaves git alone after ending its helpers". The code did not do that.
- Second, smaller mis-timing: `pass` stamped each `SIGTERM` with the time from **before** the read (:307), so
  `HELPER_TERM_GRACE` was also shortened by a slow `ps`.

### Fix (cause, not symptom)

- Each signal `pass` sends is stamped after the read returns.
- `hooks_first` times git's grace from the first helper's `SIGTERM` (`Helpers::first_asked`). The cleanup bound
  after git ends (`until`) is measured from the same point.
- No timeout was made longer, and no retry or sleep was added.
- **Cost to the user:** a stop on a loaded machine can take one `ps` longer before git is shot. On an idle machine
  that `ps` takes milliseconds. In return, the merge is left paused as promised.
- `STOP_WITHIN` and the MAP rows now say a hook-first stop adds that first read, and that Graft does not bound
  it. The constants are unchanged.

### Tests

- **Regression:** `graft-git run::stop::tests::a_slow_read_of_the_group_does_not_spend_gits_grace`. A stand-in
  git waits on a helper and then takes 100 ms to exit 1. The group read takes `HOOKS_GRACE` + 100 ms, and the
  test uses the real graces so git keeps the app's 1.9 s margin under load. It asserts that git was not
  signalled and that it exited 1.
  - **Red before the fix:** 3 of 3 against HEAD's `stop.rs`, `git_signalled: true`, status 15.
  - **Green after:** 3 of 3. It takes about 2.3 s.
- **Original scenario, forced:** the same two injections run against the real pty test. Red 3 of 3 before the
  fix, green 3 of 3 after (`git_signalled: false`, exit 1, `Paused(Merge)`).
- **Natural flake:** never reproduced (0 of 36), so the link to the 20.6 s failure is inferred. What supports it:
  - a `ps` reaching the grace was measured under load;
  - when the race is forced, it gives the test's only load-sensitive failure in the stop phase.
  - **Unverified:** that the original failure was at the `PAUSED` assertion. No output was kept.

### Hypotheses ruled out

- **The forced answer takes longer than `FORCED_WAIT` (7.55 s).** Measured from stop start to answer: 0.1–0.6 s
  under load. Not it.
- **A 20 s `PATIENCE` wait times out.** That needs a step taking 20 s or more. Under load the setup took 2–7 s
  and the offer comes at 10 s. Not seen.

**Touches.**
- `hooks_first` is used by every hook-first stop: local writes and resumes (`graft-ops` `plan_git`,
  `resume_invocation`). Behaviour changes only when a group read is slow. Checked by:
  - `graft-git run::tests::stop::*` (the real-git pause, a hook that ignores `TERM`, a stopped hook, the fallback);
  - `graft-ops` `stop_hooks` tests;
  - `graft it::quit_waits::*`.
  All of these pass in the workspace run.
- `after_git_ended` shares `pass`, with the same stamping change. Covered by the same tests.
- `SHUTDOWN_GRACE` and `FORCED_WAIT` (graft `jobs/mod.rs`) are derived from `STOP_WITHIN`, which is unchanged.
  A `ps` slower than the 3.5 s slack could leave the answer unheard. The exit line then says "graft here shows
  where git left it", and the next launch recovers. Not changed.
- No keymap, capture or journal change.

**Gates.** All workspace-wide, with `--test-threads 4`: `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` clean.
- `cargo nextest run`: 5902 passed, 10 skipped (555 s, beside the other builder). That run used the test's earlier
  300 ms graces; afterwards only the test's constants changed. The new test plus `run::tests::stop::*` were then
  re-run green 3 of 3.
- `cargo test --doc`: ok.
