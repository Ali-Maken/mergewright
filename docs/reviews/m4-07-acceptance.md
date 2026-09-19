# Review: M4 group 07 — the acceptance walk for remotes

Reviewed: merge `96a188e` against its first parent (`git diff 96a188e^1 96a188e`),
plus the state of `KNOWN_GAPS.md` at HEAD `349cbd8`, because the ticket hands the
gap rows to the conductor and the acceptance artefacts point at them.
Ticket: `.scratch/m4-remotes/issues/07-acceptance-remotes.md`.
Design reference: `docs/design-notes/m4-remotes.md`, the Promise line.

**The question:** does the walk let the user judge *"you never leave Graft to
fetch, pull or push — not for a password, a rejection or a hang"*, or does it
only assert that code ran?

**Answer:** it lets the user judge two of the three failure modes end to end,
and the third in part. The password mode runs git → askpass helper (the real
`graft` binary under `cargo test`) → socket → in-app prompt → keys → git →
loopback server with Basic auth, and the verdict is `git -C remote.git
rev-parse main == HEAD` plus the server's own authorised/rejected counters.
The rejection mode runs a real non-fast-forward rejection, the lease box, and
a lease that the bare's move defeats, with the bare's `for-each-ref` before
and after. The hang mode is a socket that accepts and never answers, `Esc`,
the write gone in under 5 s, `Aborted("cancelled")` in the journal, no
`*.lock` — **for fetch only**. Every scenario reads truth from git on both
repositories, never from `AppState`; every one ends in `git fsck` and a
search for the two passwords. That is judgeable, not "code ran".

What the walk honestly cannot make the user judge: colour, the box's frame,
SSH, TLS, a real credential helper, 120x40, and any real terminal. TESTING.md
says all of that in "What the walk proves and what it does not".

## Counts

| Severity | Count |
|---|---|
| Critical | 1 |
| Major | 5 |
| Minor | 9 |

## Gate results (all in `CARGO_TARGET_DIR=target-review07`, since deleted)

| Check | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo run -p graft --example m4_walk` | `19 ok, 0 failed`, exit 0; **10.4 s wall** for the binary alone, 22 MB peak RSS; `--only journal-rows` → `1 ok`, exit 0; `--only no-such` → exit 1 |
| `cargo nextest run -p graft` | **760 passed, 19 skipped, 12.4 s test time, 17.3 s wall**, 715 MB peak RSS |
| Slowest 5 | `m4_walk::every_scenario::u_never_touches_the_remote` 1.62 s · `pull_fast_forward` 1.31 s · `push_plain` 1.06 s · `remote_by_key::pull_then_undo_redo` 1.01 s · `m4_walk::every_scenario::journal_rows` 0.99 s |
| Every M4 walk test | 19 scenarios + 4 others, all pass, none over 1.62 s (the ticket's 3 s line holds) |
| `target-review07` size | 989 MB (debug only); the shared `target/` is 6.0 GB |
| Heavy tier + release perf | **Not rerun here** — the conductor scoped this review to the walk and nextest. The conductor's own run at `349cbd8` (`docs/perf/m4.txt`) records `cargo test --release --features heavy-tests`: 1855 passed, 0 failed, exit 0, "frame budgets pass". **That record holds no numbers** (no p50/p99/max, no first-paint ms) — see Minor 8 |
| Secrets grep (`correct-horse-battery-staple`, `not-the-password`) over `docs/shots TESTING.md README.md CHANGELOG.md docs/reviews` | no match, exit 1 |
| Capture sizes | 16 files, 498–7992 bytes, all under the 8 KB soft cap |

## Ticks spot-checked

7 acceptance ticks. **Checked 7, held 6.**

| Tick | How I checked | Held |
|---|---|---|
| 1. walk prints `ok` ×19, exit 0; `--only` runs one; broken run prints `FAIL`, exit 1; `every_scenario_has_a_test` | ran the walk (19 ok, exit 0); ran `--only journal-rows`; `every_scenario_has_a_test` passes. The deliberately-broken `FAIL` run is described in NOTES with its exact output; **not reproduced** (needs a source edit) | yes |
| 2. one nextest test per scenario, slowest < 3 s | 19 + 4 tests pass; slowest 1.62 s (NOTES said 1.41 s; same order) | yes |
| 3. TESTING.md entries name refs/bytes/text; tables present | read all 19 entries against the `expect`/`See` code; counted the 28 `•` the breakpoint table claims (28 = the password's length); 120x40 and the three terminals `not run` | yes |
| 4. 16 captures, each < 8 KB; script asserts no password and a `•`; matrix has 4 tables | `stat` on 16 files; read `pty-walk-m4.py` lines 195–208; `captures_are_named_keyed_and_listed_in_the_matrix` and `gallery::tests::the_matrix_marks_only_what_exists_by_name` pass; read the 4 tables in `35-coverage-matrix.txt` | yes |
| 5. secrets grep finds nothing; `no_secret_leaked` after every scenario | reran the grep (exit 1); `run_scenario` calls `no_secret_leaked` on line 447 | yes |
| 6. KNOWN_GAPS rows added/closed; README and CHANGELOG sections | README and CHANGELOG sections are there. **The box is `[x]` while its own text says "not done"; at HEAD the rows are still unfiled** (Critical 1) | **no** |
| 7. every tick names its test | inspection | yes |

## Critical

### C1. The acceptance artefacts point the user at KNOWN_GAPS rows that do not exist, and tick 6 is ticked while saying "not done"

- `CHANGELOG.md` "M4 — Remotes · Known gaps": *"See `KNOWN_GAPS.md`'s M4 rows: clone (M8), add/rename/remove remotes (M7), pull with merge/rebase (M5), per-remote fetch (M7), a stall notice or timeout (M7), `q` confirming during an operation (M8) …"*.
- `README.md` "What M4 actually does", last bullet: *"Not in M4: clone, adding/renaming/removing remotes, pull with merge or rebase, fetching one remote, a stall notice, SSH … All in `KNOWN_GAPS.md`."*
- `KNOWN_GAPS.md` at HEAD `349cbd8`: `grep -n -i "clone\|per-remote\|stall\|merge/rebase\|confirming during" KNOWN_GAPS.md` finds **none** of them among the M4 rows. The conductor's commit `2b76de3` ("gaps from the walk filed") added three rows (474–476: the `Ctrl+R` race, the auth URL, the walk's coverage limits) and none of the deferrals the ticket lists. Also still open and now stale: row 205 (`notify` watcher, expiry **M4** — the design note moved it to M7) and row 208 (`gallery --matrix` overwrites the working-tree table — **fixed by this group**, `coverage_matrix` writes four tables, but the row is open and `docs/gaps/closed.md` has no entry). The `RepoFacts.remotes` `&[]` row the ticket says to close: not found either way.
- Three of the builder's six NOTES findings are unfiled too: the push box draws no frame (finding 3), 80 columns cuts the rejection hint to `or P to fo…` and the journal's push row to `not undoa…` (finding 4), `last_rejected_push` is memory-only so a restart loses the lease offer (finding 5).
- Ticket tick 6 reads `- [x] KNOWN_GAPS.md rows added/closed as listed … — **not done: the conductor asked this builder not to touch KNOWN_GAPS.md**`. The prose is honest; the box is not. CLAUDE.md: never mark half-done work done; a false tick is Critical whatever its size.

**Why it matters:** KNOWN_GAPS.md is the one file the user trusts to list what is not there. README and CHANGELOG both tell him it holds these rows. It does not, so seven deferrals have no expiry and will not surface when M5/M7/M8 are planned, and the `notify` row will expire silently at M4's close.

**Fix (conductor's, small):** file the ten rows from the ticket's NOTES ("KNOWN_GAPS rows for the conductor to file"), marked *unverified by design, not broken* with the expiries given; file findings 3–5 (finding 4 as *broken*, see Major 2); move row 205's expiry to M7; close row 208 into `docs/gaps/closed.md`. Then either untick tick 6 or leave it ticked with "filed at `<hash>`".

## Major

### M1. The hang mode is proven for fetch only; CHANGELOG and TESTING.md claim more

- `crates/graft/tests/it/m4_walk.rs:1695–1729` `hung-remote-esc-cancels`: `f` against `HungRemote`, `Esc`. The only hang scenario.
- `CHANGELOG.md`: *"`Esc` cancels any running write (SIGTERM, 2 s grace, entry `Aborted("cancelled")`), **fetch, pull and push included**"*.
- `TESTING.md` "Proves": *"every fetch, pull and push above ends inside Graft — … a hang ends on `Esc` with no lock file left"*.
- A push is cancelled by the walk only through the credential prompt (`password-prompt-esc-cancels`, line 1601), which is a cancel while git waits on the helper, not a hung transport. A **pull** is never cancelled at all. A pull is the one of the three that writes the index and the working tree, so it is where a leftover lock or a half-applied fast-forward would hurt. A stall *after* the connection (bytes stop mid-transfer) is not exercised either; the deferred "stall notice" row covers the notice, not `Esc` mid-transfer.

**Fix:** either add `hung-remote-pull-esc-cancels` (same `HungRemote`, `p` instead of `f`; assert `HEAD`, `.git/index` bytes and the working tree unchanged, no `*.lock`) — it is ~30 lines by copying the fetch scenario — or change the CHANGELOG line and the "Proves" bullet to say the walk proves it for fetch and the password prompt, and that pull/push cancel rest on `remote_by_key` unit tests. Prefer the scenario: it is the promise's third clause.

### M2. At 80 columns the rejection hides the key the user needs (product finding, unfiled)

- `docs/shots/m4-06-rejected-sticky-80x24.txt` last line: `push rejected: origin/main has commits you don't · p to pull first, or P to fo…`. The same in `m4-02-push-box-lease-80x24.txt`.
- `docs/shots/m4-07-journal-remote-rows-80x24.txt`: the push row ends `not undoa…`.
- The rejection mode's whole point is that the next key is on the line. At 80 columns the user sees `p to pull first` and a cut `P to fo…`. The design (`docs/design/02-keymap.md`) puts `P` after a rejection on the sticky line, not the footer, and the footer hints for `f p P` need 156 columns (KNOWN_GAPS row 459), so at 80 columns **no line on the screen names `P`**.
- TESTING.md's breakpoint table discloses the truncation; the builder's NOTES finding 4 names it; nothing files it as broken.

**Fix:** file it as *broken* with expiry M5; a fix ticket should truncate the reason, not the hint (`push rejected: origin/main has commits you don't… · p pull · P force`), or drop the reason first when the width is short. Not this group's to fix.

### M3. `coverage_matrix` now drags every walk into every example (architecture and readability, PROMPT.md 6.1/6.2)

- `crates/graft/tests/it/gallery.rs:875–876`: `coverage_matrix` calls `crate::m3_walk::working_tree_matrix` and `crate::m4_walk::remotes_matrix`.
- Consequence in the diff: `examples/gallery.rs` gains `#[path]` includes of `app_harness.rs`, `m3_walk.rs` and `m4_walk.rs` with `#[allow(dead_code)]`; `examples/m3_walk.rs` gains `m4_walk.rs`; `examples/m4_walk.rs` includes `gallery.rs`, `m3_walk.rs` and `app_harness.rs`. Every example now compiles all three walks (2.2k + 2.x k lines of scenario code) to print a coverage table, and the pure halves are mutually dependent: `gallery` → both walks → `gallery`. The next walk (M5) will add a fourth include to four files.
- It does fix the KNOWN_GAPS row (the three `--matrix` commands agree), so the outcome is right; the shape is not.

**Fix:** move the three table writers into a small `tests/it/coverage.rs` that holds only "which stems exist at which sizes" (the stems as string constants or a tiny `fn stems() -> &[&str]` each walk exposes without its scenarios), and have `coverage_matrix` take them from there. `codebase-design` applies when moving it.

### M4. `tests/it/m4_walk.rs` is one 2,182-line file

- Harness (`Fixture`, `Truth`, `Marks`, `Walk`, waits: lines 1–750), step vocabulary and assertion helpers (751–962), 19 scenarios (963–1959), the coverage table and CLI helpers (1961–2028), and the tests (2030–2182), all in one file. The M3 walk set the precedent; this one is larger and will be copied again for M5.
- PROMPT.md 6.2 makes file size a hard rule; the reader who wants to know what `hung-remote-esc-cancels` asserts scrolls past 1,700 lines of harness.

**Fix:** `tests/it/m4_walk/{mod.rs, harness.rs, scenarios.rs, coverage.rs}` — `#[path]` inclusion from examples still works on a directory module. Do it before M5 copies the shape.

### M5. "Quoted as the walk asserts them" is not quite true, and `no_secret_leaked` is described as doing more than it does

- `TESTING.md` "Reading the entries": *"Status lines are quoted as the walk asserts them"*. Entry 7 quotes the box's message `1 commit · can't be undone from here — origin keeps what you…` and `[ Cancel ]     Push`; entry 10 quotes `origin/main <old> → <amended> · refused if origin moved since…` and `rewrites history for everyone who has <old>`. The walk asserts only `box_title_starts(state, "Push ")` / `"Force-push "` (`m4_walk.rs:1321`, `1458`); the message lines come from the captures. A reader who takes the section at its word thinks the box text is under test. It is not — a wording regression in the box would pass the walk.
- `TESTING.md`: *"a search of the journal file, every status line the walk recorded **and every capture file** for the walk's two passwords (`no_secret_leaked`)"*. `no_secret_leaked` (`m4_walk.rs:480–502`) searches the journal bytes, the recorded events and the status lines. Capture files are checked only by `pty-walk-m4.py` at write time; **no test reads the committed `docs/shots/m4-*` files**. The claim is stronger than the code.

**Fix:** (a) assert the box's message lines in `See` for `push-plain` and `push-rejected-then-lease` (`view.message.iter().any(...)`, as `push-sets-upstream` already does for `sets upstream`), or change the sentence to "status lines and box titles are quoted as asserted; box bodies as captured". (b) add the sixteen capture files to `captures_are_named_keyed_and_listed_in_the_matrix` (read each, assert no `SECRETS` member), and say so in TESTING.md.

## Minor

1. **Progress percentage never demonstrated.** `README.md` shows `fetch all remotes · receiving objects 45% · esc cancels`; the walk asserts only `line.contains("objects")` (`m4_walk.rs:1747`) and both `m4-05` captures show `enumerating objects` with no count. 400 × 4 KiB over `file://` is too quick for a percent to land. Either grow the fixture (the ticket suggested one 2 MiB file) and assert a `%`, or mark the percent form as unit-tested only (`graft-git::parse_progress`) in TESTING.md.

2. **Captures a user cannot judge from the page.** Of 16: `m4-01-push-box-plain-{80x24,160x48}` — the box has no frame and its lines sit inside graph rows (`●   CommPush main → origin/main`, `●   Commit 182   Push main →`), so it reads as graph corruption, not a box; `m4-05-progress-footer-*` — only the footer line differs from an idle screen and the frame is unsettled; `m4-06-rejected-sticky-80x24` and `m4-07-journal-remote-rows-80x24` — the text the row claims is cut (M2). The other ten show what their scenario claims. TESTING.md discloses each; this is for the user's by-eye pass.

3. **README omits two limits CHANGELOG lists:** the footer hints need 156 columns and the typed password sits unzeroed in a `String`. README says *"Graft never stores what you type"* — true of disk, and KNOWN_GAPS row 470 covers memory, but the README sentence is the one a user reads. Add half a line.

4. **`Step::TypeSecret` is `Step::Type` under another name.** `m4_walk.rs:570` matches `Step::Type(text) | Step::TypeSecret(text)` to one arm; the doc says the secret step "is not printed", but `drive` prints no step. The distinction is documentation. Keep it (it reads well in a scenario) but say so in the doc comment, or have `drive` trace steps under `GRAFT_TEST_TRACE` and elide the secret there — then the variant earns its name.

5. **Redundant check.** `u_never_touches_the_remote_expect` (`m4_walk.rs:1775–1781`) asserts the tracking ref `!= theirs` and then `== head(pushed)`; the second implies the first. Drop the first.

6. **`Fixture::root()` uses `expect` in code compiled into three examples** (`m4_walk.rs:142–146`, under a file-level `#![allow(clippy::expect_used)]`). Its siblings return `Result<_, String>`; this one could too. Dev-only, so Minor.

7. **Matrix rows are in scenario order, not stem order** (`m4-01, m4-02, m4-06, m4-03, …` in `35-coverage-matrix.txt`). The M3 table has the same quirk. Sort by stem in `remotes_matrix` or renumber the stems.

8. **`docs/perf/m4.txt` records a verdict, not numbers.** Five lines: pass counts and "frame budgets pass under --release". The review protocol reports the frame p50/p99/max and first-paint figures; the `frames::` and `heavy::budgets_hold` tests print them under `--nocapture`. Not this group's diff, but the milestone's perf record should hold them. Rerun the release perf command into the file when the fix round runs.

9. **The masked field shows one `•` per character** (`m4-03`: 28 dots for a 28-character password). Standard, and the capture leaks only a length, but a walk whose point is "the password is never on screen" now commits its password's length. A fixed-width mask (say 8 dots regardless) would remove even that; user's call, since it changes what typing feels like.

## Answers to the conductor's questions

- **TESTING.md rows that amount to "looks correct":** none. All 19 entries name refs, bytes or exact status text. The breakpoint table's push-box row says *"judge whether that reads as a box"* — that is the user's judgement, honestly asked, not a tick.
- **Captures a user could not judge:** four of sixteen, listed in Minor 2. Twelve show their state.
- **Three failure modes end to end:** password — yes; rejection — yes; hang — fetch only (M1). The one honest hole in the promise is by design: a diverged pull sends the user to `git pull --rebase` (M5), and `pull-diverged-refused` proves the line says so.
- **Overclaim:** C1 (KNOWN_GAPS rows cited that do not exist; tick 6), M1 (hang "fetch, pull and push included"), M5 (box text "as asserted"; capture files "searched"). None of README/CHANGELOG/TESTING.md hides SSH, `Esc` on `u`/`U`, the lease-to-upstream-only rule, 156 columns or the unzeroed password: CHANGELOG and TESTING.md state all five; README states three and points to KNOWN_GAPS for the rest.
- **`coverage_matrix` with 4 tables:** honest — each table says "Unicode tier only; colour is never captured" and marks only files that exist (`the_matrix_marks_only_what_exists_by_name`). It has no 120x40 column and does not say 120x40 was skipped; TESTING.md does.
- **Ticks:** 7 checked, 6 held (table above).

## Not verified here

By eye, colour, any real terminal, 120x40, SSH, TLS, a real credential helper, the deliberately-broken `FAIL` run (needs a source edit), the heavy tier and the release perf numbers (not rerun; the conductor's record at `349cbd8` has the verdict and no figures).
