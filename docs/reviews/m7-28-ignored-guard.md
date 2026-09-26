# Review — m7-power-tools group 28: the ignored-file guard (ADR-0049), tickets 28a + 28b

Reviewer, 2026-09-26. Diff `a7611fb8..73dd034f` (28a merge `76127bd9`, 28b merge `73dd034f`), graft-ops only for
this group. All runs were made in a detached worktree at `73dd034f` (outside the repo), with
`CARGO_TARGET_DIR=$TMPDIR/graft-target-m7-rev28`. The worktree has been removed.

**Counts: Critical 0 · Major 3 · Minor 6 · Questions 2.**

## The promise

**An operation that would overwrite a file git ignores is refused before it starts, and the refusal names the
file.** It holds for every kind ADR-0049 §2 covers. Checked three ways:

- The 24 tests in `crates/graft-ops/tests/it/in_the_way.rs` pass. The 18 refusal tests each assert the preview
  words, the apply words, the bytes still `MINE`, and 0 new journal entries, all read from disk and the journal.
  The other 6 are no-false-refusal tests or git's own refusal (the pull whose fetch brings the file).
- The guard runs before the set-aside and before `begin`: `apply.rs:224`, before `set_aside::create` at `:236` and
  `open_entry` at `:251`.
- The refusal names the path first (`operation.rs:981-998`). If the status line truncates the message, it cuts
  the next step, not the file name.

**Nothing else regressed:**
- **Undo:** the full fast tier is green twice. `recover.rs` has its two moved tests, and there is no new journal
  line on a refusal.
- **Set-aside:** `set_aside.rs::a_refused_rebase_puts_the_edits_back_on_the_same_tip` now uses a `pre-rebase`
  hook. The premise changed, so this is the right cause.
- **Pull:** the plan args at `operation/remote/tests.rs:151` are right. `pull_whose_fetch_brings_an_ignored_path_is_refused_by_git`
  passes. The only graft-side pull line is `docs/capabilities/m4-remotes.md:56`, and it is updated.

The guard does refuse things git would not have written. There are three such false refusals, all on the safe side
(Major 1, Major 2, Minor 1). I found no path by which his bytes are lost.

**Feature pipeline:** not applicable. This is a defect fix to M3's promise ("you cannot lose work"), with no new
screen or key. It is decided by ADR-0049, which the user accepted on 2026-09-26. There is no journey to count. The
one new "Time I left" (`move it aside first`) is recorded in ADR-0049's Consequences.

## Ticks spot-checked: 13 checked, 13 held

- **28a:**
  - switch, detached, new-branch, branch-from-stash, ff and no-ff merge, hard reset, put-back, untracked, dir/file-above
    (all ten test names exist and pass, and each asserts the four checks);
  - the pull plan-args (`remote/tests.rs:151`);
  - `in_the_way_words`: all four word rows, a mixed case, and one `<doing>` per kind (`operation/tests.rs:1395-1593`);
  - the one read: `grep symlink_metadata crates/graft-ops/src` finds only `in_the_way.rs:607,644` and
    `merge_state.rs:110` (the git dir);
  - `blocking` is called only from `preview.rs:438` and `apply.rs:471`;
  - MAP §in_the_way;
  - the gates (re-run, green).
- **28b:**
  - the middle-commit rebase, the two-commit pick, and the no-false-refusal replay (tests exist, assert what they
    say, and pass);
  - MAP's replay rows and its claim about the engine's `switch <home>`;
  - the carried `m4-remotes.md:56` line.

I did not re-run the "red before" tuples; the NOTES record them. Each refusal test asserts both the preview's and
apply's words, so removing either guard site should turn it red. That is reasoned from the assertions; I did not
mutate the code to check it.

---

## Critical

None.

## Major

### M1 — Pull falsely refuses when his branch is ahead and one of his commits stopped tracking a now-ignored file
**Where:** `crates/graft-ops/src/in_the_way.rs:210-211`. Pull reads `from_head(tracking_tip)`, which is
`diff-tree HEAD <tracking_tip>`. That is what a *switch* writes, but a fast-forward writes only the incoming side,
which is `<merge-base>`→`<tip>`. Merge already does this right with `--merge-base` (`:202-207`). The fault is in
ADR-0049 §4's table, which files Pull under the same row as Checkout. The builder followed the spec.

**Observed** with a reviewer probe (a throwaway test, not committed):
1. `LocalRemote`: the tracking tip tracks `remote-0.txt`.
2. He commits `git rm --cached remote-0.txt` and does not push. The file is ignored and holds `MINE`.
3. `p` gives, in both the preview and apply: `remote-0.txt (ignored) is in the way of pulling origin/main · move it aside first`.

Plain `git merge --ff-only --no-overwrite-ignore origin/main` gives `Already up to date.`

**Why it matters:** this is the ADR's own example (`.env`, once committed, now ignored). Committing "stop tracking
.env" and then pulling before pushing is a normal team sequence (memory: works in a team). `p` is a daily key, and
it would tell him to move a file that nothing would touch.

**Fix:** Pull's row becomes `diff-tree --merge-base HEAD <tracking_tip>`, the same as merge. Amend ADR-0049 §4's
table. Add a test: ahead by one commit that untracks an ignored file, and the pull is not refused.

### M2 — A case-only rename is refused on macOS's default case-insensitive disk, and the words call his tracked file "(ignored)"
**Where:** `in_the_way.rs:87-91` filters tracked paths byte-exact, then `:607` runs `symlink_metadata`, which
matches regardless of case.

**Observed** with a reviewer probe on this machine (APFS, `core.ignorecase=true`):
1. `main` tracks `Readme.md`; `other` renames it to `README.md`.
2. The write set is `{README.md, Readme.md}`. The index holds `Readme.md`, so `README.md` is a candidate.
3. The stat of `README.md` finds `Readme.md`.
4. Both the preview and apply give: `README.md (ignored) is in the way of switching to other · move it aside first`.

Plain `git switch other` succeeds.

**Why it matters:**
- The user is on macOS.
- Case renames happen in real repos (`button.tsx` → `Button.tsx`).
- While the rename sits on one branch, it blocks switch, merge, pull, rebase and pick between the two branches.
- The advice is wrong: moving his tracked file aside turns it into a deletion.
- The same byte-exact compare is in `:619` (the tracked ancestor) and `:678` (`holds_untracked`).
- A decomposed vs precomposed Unicode name (`core.precomposeunicode`) is probably the same class. I have not
  verified that.

**Fix:** when the stat finds something, get the on-disk name of the entry (`read_dir` of the parent, exact
match). If that name is tracked, it is not in the way. Or, when `core.ignorecase` is true, compare case-folded.
Add a test with the probe's fixture. It is naturally skipped on a case-sensitive disk, so add a note saying so.

### M3 — Readability (PROMPT.md 6.2): a new 718-line file, two functions over ~60 lines, and a runtime string template
- **The file:** `crates/graft-ops/src/in_the_way.rs` is 718 lines against a limit of about 400. It holds three
  concerns:
  - the public read and its words (`blocking`, `refusal`, `refused`, `doing`);
  - the paths per kind (`written_paths`, `replayed_paths`, and about eight git-read helpers);
  - the disk walk (`on_disk`, `file_above`, `holds_untracked`, `sort_out`).
- **Function size:** `written_paths` is 68 lines (`:186-253`) and `replayed_paths` is 64 (`:260-323`).
- **The template:** `operation.rs:983` and `:996` build the next step by `"move {} aside first".replace("{}", them)`.
  That is a template filled at run time where two plain `format!` arms would do, and it is the "clever over plain"
  pattern §6.2 names.

**Fix:**
- Split the file into `in_the_way/written.rs` (the table and the git reads) and `in_the_way/disk.rs` (the stat walk
  and the untracked/ignored split), keeping the public items in `in_the_way.rs`.
- Pull the stash and replay arms of `written_paths` into their own functions.
- Write the next step as `match (ignored, several)` with four literal strings.

## Minor

### m1 — KNOWN_GAPS M7/28skip: verified, and wider than the row says. My call: a real false refusal, cheap to fix, worth fixing before the M7 stop
**Where:** `in_the_way.rs:317-319` counts `HEAD`→`onto`, and `:271-277` counts every row not marked `drop`. git's
sequencer skips leading `pick` rows whose parent is the running `onto` (it advances `onto` and never writes the
skipped trees). Graft runs `rebase -i` without `--no-ff` (scout 28's table).

**Observed:** a fixture like `env_dropped` (X runs `rm --cached .env`, Y follows; `.env` is ignored and holds `MINE`):
- **Raw git:**
  - a fixup into X (`rebase -i --autosquash X^`) leaves `.env` as `MINE`;
  - `rebase -i main` with Y reworded leaves `.env` as `MINE`.
- **Graft:** `R` with Y reworded is refused with `.env (ignored) is in the way of rewriting feature · move it aside
  first`.

**Wider than the row:**
- `R` always starts at the base (`rewrite/plan.rs:109-110`). So on a branch that has untracked a file which is now
  ignored, **every** `R` that leaves X's row unchanged is refused. Only `R` that edits X itself is a real refusal,
  because then git does write `onto`'s tree.
- `a` is hit only when the target is X itself (`onto` = the target's parent, `plan.rs:176`).
- Absorb is hit when the first absorbed target is X.

**Verdict:**
- Safe side, and no loss.
- Hitting it is correlated with the `.env` case the ADR exists for.
- git's skip rule is small and deterministic: while the next row is `pick` and its first parent is the running
  `onto`, advance `onto` to it and drop it from the replayed set. That is about 15 lines.

**Fix:** model the skip in `replayed_paths`. Pin it with a test that runs the same rewrite through Graft and through
raw `git rebase -i`. Change the row's status from **unverified** to **broken (false refusal, verified)**.

### m2 — KNOWN_GAPS row M7/28 still reads **broken** after 28a and 28b closed it
**Where:** `KNOWN_GAPS.md:875` at `73dd034f`.

**Fix:** move it to `docs/gaps/closed.md` and cite this review. This is the conductor's.

### m3 — `c` continuing a stopped replay is not re-checked, and no gap row says so
**Where:** the ticket's scope-out (decision 2).

**Why it matters:** between a stop and `c`, he can create or ignore a file the rest of the replay writes. It is
narrow, but it is an out-of-scope hole in the promise, and CLAUDE.md rule 4 wants it written down.

**Fix:** add a KNOWN_GAPS row marked **unverified**.

### m4 — The same situation gets two sets of words, and keymap text is stale
- **The words:** a stash put-back over an ignored file says `c.txt is in the way` (`operation.rs:1004-1021`). It has
  no `(ignored)` and no next step. Every other kind says `c.txt (ignored) is in the way of … · move it aside first`.
  That breaks ux-principles consistency and next-step visibility. The ticket's scope-out ("changing any stash words")
  is why the builder left it.
- **The keymap:** `docs/design/02-keymap.md:425` still says the stash's in-the-way files are refused "(git refuses
  those)". Graft refuses them now, ignored files included.

**Fix:** give `StashInTheWay` the `(ignored)` tag and the next step when any of its paths are ignored, and edit the
keymap line. Both need the conductor.

### m5 — The cost of the read is unmeasured on a big repository
**Where:** `in_the_way.rs:84`. Every covered operation now runs a full `ls-files` into a `BTreeSet<PathBuf>`: once
at the preview and again at `y`. The instant switch (ADR-0048) runs it once.
- `preview.rs:383` runs the read even for kinds that then refuse `NotPreviewable` (pull, rewrite).
- The release perf run exercises none of these kinds, so the budgets being green says nothing about it.

**Fix:** time `blocking` for a switch on the 100k synthetic fixture (script to `docs/perf/m7-28.txt`). If it
matters, only list the index when `on_disk` found something, since the untracked candidates are usually empty.

### m6 — The tests: size, and absorb is untested
- **Size:** `crates/graft-ops/tests/it/in_the_way.rs` is 848 lines (test-only).
- **Absorb:** it goes through `Rewrite` with `made` commits (`rewrite/plan.rs:220-278`). It shares the path that
  `amend_into_over_an_ignored_file_is_refused` covers, but it has no test of its own, although decision 2 names it.

**Fix:** split the tests into `in_the_way/{one_shot,stash,pull,replay}.rs` and add one absorb case.

## Questions (no line, rule 7)

- **Q1: switching over a staged `rm --cached` drops the staged deletion.** Observed with a reviewer probe:
  1. `main` tracks `.env`; `git rm --cached .env`; `.env` is ignored and holds `MINE`.
  2. Status is `D  .env`.
  3. Graft switches to `other` (which also tracks `.env`). The preview and apply both run.
  4. `.env` is still `MINE`, but status afterwards is ` M .env`. The index tracks `.env` again, so his staged
     "stop tracking" is gone.

  This belongs to the set-aside (ADR-0045), not to 28, and I have not traced which step drops it. It is the
  builder's 28a question 1, observed. Needs a scout or a debugger before it becomes a ticket.
- **Q2: do the refusal words fit the status line at 80 columns?** A long branch name plus a deep path runs past 80.
  The path comes first, so the file name survives, but the next step may be cut. There is no capture. The ticket
  hands this to the acceptance ticket; it should be one of the scenarios there.

## Gates and runs (fixed tree `73dd034f`)

The machine was shared: load average 5 to 12, with other builders running. The wall times below are inflated
(the builder measured about 70 s for the fast suite).

| Check | Result |
|---|---|
| `cargo fmt --all --check` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | ok (29 s, peak RSS 800 MB) |
| `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run --workspace`, run 1 | 3897/3897 passed, 1 skipped · 352.5 s · peak RSS 273 MB |
| `cargo nextest run --workspace`, run 2 | 3897/3897 passed, 1 skipped · 348.7 s. The flaky capture in 28b's NOTES did not recur. |
| `cargo test --workspace --doc` | ok |
| Heavy tier: `cargo test --features heavy-tests -- --test-threads=2` | graft `it` 531/536, 574 s, peak RSS 1.05 GB. The 5 failures are all known and outside 28's crate: `heavy.rs:377` and `many_refs.rs:41,68,114` (M7/borrow4), and `heavy.rs:217` (M7/heavy217). graft lib 1349/1349. |
| Release perf: `frames::` + `heavy::budgets_hold`, `--release` | **14/14 passed**. First paint: 32 ms (100k with commit-graph), 74 ms (with 50k refs), 112 ms (realistic), 135 to 136 ms (no commit-graph). Bottom: 93 to 101 ms with a commit-graph, 965 to 1080 ms without. The 50k-ref badges take 10.3 to 13.2 ms. Peak RSS in-test is 103 MB. |
| Fuzzing / benchmarks | Not run. The 7 fuzz targets are parsers (blame, conflict, log, file list, porcelain v2, unified diff), and the benches are graft-graph and graft-ui. 28 touches none of them. |

The five slowest tests (run 2):
1. `m5_walk::every_scenario::merge_resolved_with_the_keys`: 16.1 s
2. `undo_cancel::cancel_at_every_step_boundary_is_clean`: 14.2 s
3. `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`: 13.8 s
4. `diff_fixture::parse_unified_matches_numstat_for_every_shape`: 13.3 s
5. `conflict_ops::whole_file_shapes_resolve_like_git`: 13.0 s

`target/` sizes: the review target is 7.5 GB, and the shared `target/` is 16 GB.

**Laptop safety:** the new tests have no sleep, no network, and no timeout-sensitive wait (read, not measured per
test). The pull tests use `LocalRemote` (file transport).

**Other checks:**
- No `unwrap()` or `expect()` outside tests in the diff (`in_the_way.rs` has none).
- Git work on the UI thread: no new call sites. The read runs only inside `preview::plan` and `apply`; I did not
  re-verify which thread graft calls those on.
- No terminal output was added, so there is no sanitising concern.
- Crate rules hold: graft-ops uses only graft-core and graft-git.
- Errors are not swallowed: `NotFound` is skipped on purpose and commented (`:593`), and every other I/O error
  becomes `FactsError::OnDisk`.
