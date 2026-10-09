# Review — M8 group: hygiene, docs, release, Linux

Reviewer, 2026-10-09. Read-only against ROOT `main` at **`174121a0bb0de226e7eaa881b2ce3555f4dd163f`**.

Merges reviewed: `96793472` (21b-3), `7e6bbb29` (21b-4 items 1–10), `9505ca4a` (21b-4 additions 5b–5h),
`7877f670` (23), `255a9e84` (24), `298d0a90` (24-b), `986adbe2` (21b-5 + 5j–5l), `8fbb5eb9` (21b-7a),
`c598fceb` (21b-7b), `be7b66b1` (30).

SKILLS: code-review (SKILL.md read directly; its two parallel sub-agents were not spawned, because the conductor's
brief was read-only and lean. I ran both axes, standards and spec, myself.)

## What I ran, and what I didn't

- **No cargo.** The conductor's instruction: another helper holds the cargo slots. Gates, heavy tier, the release perf
  run, fuzzing, benchmarks, the timings, peak memory and the size of `target/`: **none run, none reported.** The
  builders' own gate lines are quoted in their NOTES. I did not re-observe them.
- **Ticks** were checked against the tree at the hash. Each named test exists (`grep -rn "fn <name>"`, 30 names, 30
  found), and a sample was read to confirm it asserts what the tick says.
- **Moves** were checked with a line-set script over each merge's diff, run in bash with `LC_ALL=C`. It lists non-blank
  trimmed added lines that are absent from the removed lines, after dropping `use`/`mod`/`//!` lines, `pub(…)`
  widening and `super::super::` re-pathing.
- **Sizes** were checked with `wc -l` at the hash.

### Spot-checks per ticket (sampled / held)

| Ticket | Checked | Held |
|---|---|---|
| 21b-3 | items 1 (`next_read` taken after the read: `run/stop.rs:239`, `stop/helpers.rs:40`), 4 (500 ms bound, `remote/tests.rs:559`), 5 (`READ_TIMEOUT` 30 s, `http/mod.rs:49`), 6 (`CheckOut`, grep `its filter failed\|SnapshotError::Filter` empty), 8 (`graft-core/src/tests.rs:56`) | 5/5 |
| 21b-4 + 5b–5h | 1 (`excused_by_hangup` table; startup line-set shows only item-1 lines), 2/3 (startup, session, empty_state line-sets clean; `wc -l` 365/364/373), 4 (`empty_state/palette.rs` `first_runnable` once), 5 (`find.rs` 395), 6 (`view/tests.rs:5290` drives `stash_view`), 7 (`fold_count/tests.rs:65` asserts `ReadHistory`), 9 (J numbers match the note), 5f/5g/5h (assertions read) | 10/10 |
| 21b-5 + 5j–5l | undo, banner, update/tests, keymap_words/stops, graft-ui empty_state, places/outside: line-sets clean (only fmt re-wraps, re-export lists, two "only the tests reach these" comments); sizes | 5/5 |
| 21b-7a | 1a table = the spec's 9 rows; 1b guard; 2 (`update/tests.rs` 128, fixtures 212/205); 3 (`consequence.rs` 289, line-set clean); 4 (`rows.rs` 893, `context.rs` 222, line-set clean apart from `pub(super)`) | 5/5 |
| 21b-7b | header_tail, catalogue/forms, footer/tests: line-sets show only `pub(super)` widening and one import list; sizes 71/396/285 | 3/3 |
| 23 | `help_is_usage_of_flags`, `committed_man_page_is_the_generated_one`, `config_keys_and_the_loader_name_the_same_keys` (`docs_tests.rs:121,128`), `readme_backticked_names_exist` (`:237`) | 4/4 |
| 24 / 24-b | `plan.txt` (4 archives, `graft.1` ×4, installer, formula), `git-matrix.txt` per-version lines, `version_is_asked_for_in_both_spellings…` (`cli.rs`) | 3/3 |
| 30 | hang-up ENXIO tests (`startup/tests.rs:165`, table row), `racy_index.rs` (sets mtimes deterministically, asserts the edit is seen), `a_local_remote_never_grows_a_commit_graph`, `pty::a_walk_runs_in_the_test_terminal…` (the env pinned covers every var `graft-term/src/detect/` reads) | 4/4 |

**No false tick found:** 39 sampled, 39 held. Some boxes are *under*-ticked, meaning done but left open (Minor 1).

### The asked-for checks

- **Moves were move-only.** Every sampled line-set held. The only non-move lines are item 4 of 21b-4
  (`empty_state/palette.rs:100`), item 1 of 21b-4/21b-7a (new code, ticketed), and fmt re-wraps.
- **Purity scan (21b-7a) keeps its guarantee at this hash.** Every directory named `tests` under `update/` has a
  parent that declares `#[cfg(test)]` directly above `mod tests` (12 dirs, checked by grep:
  `mod.rs:1419–1420`, `undo.rs:272–273`, `banner.rs:383–384`, …). There is no `#[path]` under `update/`, and no
  one-line `#[cfg(..)] mod` (grep empty). `mod.rs`'s strip point is `:1419`, the last two lines. So no production
  file escapes today. The guard's blind spots are in Minor 5.
- **Rule 3 in 30's product fixes holds.**
  - `wind_down.rs:65–69` and `:107–109` each say why the excused error is safe to drop. The ENXIO excuse is gated on
    `Exit::Signal(SIGHUP)` (`:80–81`), and the const's doc (`:84–90`) gives the cause.
  - `plumbing.rs:42–62` propagates every error with `?`. The NotFound arm is commented (`:45–46`). The `TempIndex` is
    built before the copy, so its `Drop` (`:87–100`) removes a half-written copy on failure.
- **README, man page, `--help` and config keys.** Flags agree: `-h/--help`, `-V/--version`, the hidden
  `--generate-man` (in the man page's SYNOPSIS and OPTIONS, hidden from `--help` by design). Config keys are pinned
  both ways by test. Two textual drifts: Minor 2 and Minor 7.
- **The release scripts never push, tag or publish.** Grep of the four scripts for
  `push|tag|publish|gh |upload|login`: comments only. `publish-jobs = []` (`Cargo.toml:115`). The only "Graft" in
  them is the codename in comments and artefact names, which the G-rename list (24 NOTES) covers. The committed
  workflow is the one armed thing (Question 1).

---

## Critical

None.

## Major

### M1. A fast-tier test over the 10 s bound, observed and never filed

- **Where:** `.scratch/m8-polish-and-ship/issues/21b-hygiene-round-2.md:465–467` and `:499–502`.
- **What's wrong:** 21b-3 measured `m5_walk::every_scenario::merge_resolved_with_the_keys` at **15.2 s** (READ_TIMEOUT
  1 s) and **15.3 s** (30 s). That is over PROMPT 7.1's fast-tier bound, and it was over before the ticket. The
  builder proposed rows `M8/m5-walk-merge-keys-15s`, `M8/sleep-guard-after-run` (`tests/stop.rs:131`, guard taken
  after a fallible `.expect`) and `M8/checkout-words-carry-scratch-path`.
- **Evidence:** none of the three is in `KNOWN_GAPS.md` at the hash. I grepped for the ids and for the content
  (`15 s`, `15.2`, `sleep-guard`, `--prefix`).
- **Why it matters:** the fast tier is the builders' only gate, and it is meant to be laptop-safe. A known breach of
  its bound with no row is invisible to the acceptance ticket (28) and to 25's ranking. Rule 4 says unfinished work
  goes in `KNOWN_GAPS.md`.
- **Fix:** the conductor files the three rows, all **unverified** as cause, as the builder worded them.

## Minor

### m1. Boxes done but left open, and one status line wrong

- `21b-5-additions-18e.md:50`: box **5i** is unticked. 21b-4 did it in `843be1d8` (NOTES `21b-hygiene-round-2.md:537–538`;
  `wc -l` at the hash: `empty_state.rs` 373, `empty_state/palette.rs` 149), and the ruling (`:16–17`) says the box
  ticks by naming that commit.
- `21b-hygiene-round-2.md:571`: box **5+1** is unticked. 21b-7b did it (`21b-7-moves-remainder.md:231–244`).
- `21b-hygiene-round-2.md:344`: 21b-5's `[~] 1–4` is still "partial". 21b-7a closed item 3 (`update/tests.rs` 128).
- `30-linux-fast-tier-red.md:3` says "54 of 56 fixed", and the merge `be7b66b1` says "53 of 56". NOTES `:195` says
  `outside_repo::launch_and_open_land_alike` was one of the 56 (masked by the ASCII wait) and is still red. So 53 is
  right.

Why it matters: the acceptance ticket and the next reviewer read these boxes as the state of the work.
Fix: tick 5i and 5+1 naming their commits, set 21b-5 to `[x]` naming 21b-7a, and correct 30's status line.

### m2. The README's command-line line is stale since 24-b, and it has no flag list

- **Where:** `README.md:69`.
- **What's wrong:** it says "`graft --help` prints the command line, which is one optional path." Since `298d0a90`
  there is also `-V/--version` (`cli/flags.rs:32–36`). 23's decision says `FLAGS` feeds "the README's flag list", but
  the README has no flag list, so no test caught the drift (`docs_tests.rs` pins keys and config, not flags).
- **Fix:** list the flags from `FLAGS` in the README, with a test like `readme_daily_keys_are_the_tables_keys`, or
  drop the sentence and point at `--help`.

### m3. `cargo-install-check.sh` still checks `--help`, so re-running it cannot close its box

- **Where:** `scripts/cargo-install-check.sh:42–44`.
- **What's wrong:** the comment says "Graft has no `--version` flag", and the script prints `graft --help`. 24-b fixed
  `release-local.sh` the same way (`298d0a90`) but not this script. Ticket 24's open box (`24-release-dist-and-matrix.md:57`)
  wants "binary prints `--version`", and 24-b says the box closes "on a re-run of `scripts/cargo-install-check.sh`"
  (`:163–164`). A re-run would still print `--help`. Gap row `M8/24-no-version-flag` (`KNOWN_GAPS.md:1283`) waits
  on it.
- **Fix:** print `"$tmp/cargo-home/bin/graft" --version` and drop the stale comment.

### m4. Rows routed to 23 were not in 23's ticket and stay open

- **Where:** `KNOWN_GAPS.md:1212` `M8/mockup-stash-footer` ("→ 23 docs pass") and `:1194` `M8/adr-0060-wording`
  ("23 docs pass").
- **What's wrong:** `23-docs-readme-and-man.md` names neither. `docs/design/01-view-mockups.md:392` still draws
  `⏎ apply  p pop  d drop (undoable)  b branch from stash  space preview  ? keys`, while the code says
  `… S/esc close  ? keys` (21b-1 NOTES `:418`). That is textual drift.
- **Fix:** re-route both rows to a docs ticket, 25 or 28. They stay **broken (textual)** until then.

### m5. The purity guard's blind spots, beyond the documented `#[path]` one

- **Where:** `crates/graft/src/update/tests/purity.rs:170–184` (`declares_tests`) and `:120–129`.
- **What's wrong:** `declares_tests` matches only a line that *starts* with `mod tests` or `pub… mod tests`.
  - A one-line `#[cfg(feature = "x")] mod tests;` is never counted, so it is never checked, while
    `is_test_module` (`:13–19`) still skips its file.
  - The same skip rule now applies to the `empty_state/` scan (`:128`). `every_tests_module_is_cfg_test` (`:189–190`)
    walks only `src/update`, so `empty_state`'s skipped files are unguarded. They are `cfg(test)` today
    (`empty_state.rs:368–373`).
  - Not a defect at this hash: grep finds no one-line `#[cfg(..)] mod` under `update/`.
- **Fix:** match `mod tests` anywhere on a non-comment line and require `#[cfg(test)]` before it on the same line or
  directly above. Run the guard over `empty_state/` too. Say both in the doc at `:7–12`.

### m6. `--generate-man` panics on a closed pipe

- **Where:** `crates/graft/src/main.rs:37`.
- **What's wrong:** `print!` panics when stdout is a closed pipe (Rust ignores SIGPIPE, so the write returns `EPIPE`
  and `print!` panics). `graft --generate-man | head` would exit 101 with a panic message. The same pattern is at
  `:31` for `--help` (pre-existing).
- **Not observed:** reasoned from `print!`'s documented behaviour; no run (no cargo).
- **Fix:** `write!` to a locked stdout, and treat `BrokenPipe` as a clean exit, with a comment saying why that is safe.

### m7. "Print this and exit" in the man page

- **Where:** `docs/man/graft.1:30`, from `cli/flags.rs:28`.
- **What's wrong:** in the man page, "this" reads as the man page, but `--help` prints the usage. The text is shared
  by both outputs.
- **Fix:** "print the usage and exit".

### m8. Leftovers 21b-7a found are filed but not fixed

- `crates/graft/src/update/tests/copy_and_compare.rs:166`: an `eprintln!("DBG …")` in a test loop.
- Stale comments name the old purity path: `config/tests.rs:28`, `view/resolver/tests.rs:6`,
  `update/preview/rect.rs:34`, `update/resolver/tests.rs:667`.

Row `M8/21b7a-leftovers` (`KNOWN_GAPS.md:1287`) holds them. This is listed so 28 doesn't miss them.

## Questions

### Q1. The committed release workflow is armed under the codename

- **Where:** `.github/workflows/release.yml:41–45`: `on: pull_request` and `push: tags: '**[0-9]+.[0-9]+.[0-9]+*'`.
  The host job runs `gh release create` (24 NOTES; `:214–221` gate "publishing" on a non-PR event).
- **The risk:** nothing is published today. But if this repo is ever pushed to GitHub ("the repo may be published")
  and a version tag follows before G-rename, a public GitHub release of `graft-*.tar.xz` is created. That breaks the
  codename rule and ADR-0057 §1. Ticket 24 asked for the workflow to be committed, so this is spec-compliant.
- **Question:** keep it armed, or switch the trigger to `workflow_dispatch` (or delete the file) until the rename and
  the trademark search clear? G-rename regenerates it either way.

### Q2. The git floor: ticket rule versus builder's call

- **Where:** `24-release-dist-and-matrix.md:31–32` says "A failure on 2.38 → the floor rises to the oldest passing
  version". The builder did not raise it (NOTES `:112–118`), and filed it as `M8/24-git-floor` (`KNOWN_GAPS.md:1285`).
- **The facts:** at 2.38 the app behaves as designed ("not checked", `graft-ops/src/preview/chain.rs:31,78`), but the
  fast tier fails there: 10/26 on the matrix, and 146 failures on bookworm's 2.39.5 (`docs/release/linux-git-2.39.txt`).
  Meanwhile `README.md:46` says "git 2.38 or newer".
- **Question (user's call at the stop):** raise the floor to 2.40, or keep 2.38 and document that the fast tier needs
  2.40.

### Q3. `cherry_pick.rs:281–285` asserts the Mac's outcome

Ticket 30 (`:117–124`) found Apple git outlives `SIGTERM` (2.0 s, then `SIGKILL`) while git 2.47 dies at once. If every
Mac stop ends in `SIGKILL`, a stopped git can leave `index.lock`. This is unverified and not traced. It is in
`M8/24-linux-56-red`; it is raised here because it may be a Mac-side product cost, not a Linux test issue.

---

## KNOWN_GAPS rows these tickets should have closed (still open at the hash; not edited)

All checked at the hash (test or `wc -l`, above):

- **21b-3:**
  - `M8/stop-read-cadence` (:1185)
  - `M8/13k-sleep-unguarded` (:1186)
  - `M8/idle-client-bound` (:1189)
  - `M8/http-read-timeout` (:1198)
  - `M8/snapshot-filter-blamed` (:1211)
  - `M8/adr-0007-unguarded` (:1171)
  - `M8/aborted-refs-moved` (:1215) stays open, correctly. It was re-worded as a confirmed defect.
- **21b-4:**
  - `M8/hangup-exemption-too-wide` (:1187)
  - `M8/look-file-sizes (refresh)` (:1196)
  - `M8/12b-outside-paste-duplicates-first-runnable` (:1164)
  - `M8/12f-find-rs-407` (:1291), now `find.rs` 395 and `listing.rs` 267
  - `M8/stash-ago-test-blind` (:1210)
  - `M8/fold-refresh-test-blind` (:1193)
  - `M8/blame-place-ends-with` (:1199)
  - `M8/12-walk-journey-numbers` (:1166)
  - `M8/12-test-file-size` (:1168)
  - `M8/11-files-over-size` (:1159): `session.rs` 364, `empty_state.rs` 373; `palette.rs` by 21b-2
- **21b-4 5e + 21b-1:** `M8/stash-footer-close-key` (:1208)
- **21b-5 / 21b-7:**
  - `M8/doorway-files-over-400` (:1213)
  - `M8/keymap-file-sizes` (:1151), for its named files; the rest is folded into `M8/files-over-400-unrowed`
- **21b-5 + 18e:** the graft-ui `empty_state.rs` part of `M8/look-file-sizes` (:1125). That row stays open for
  `rows.rs` (893), and maybe `render.rs` (not re-measured).
- **From 21b-1/21b-2, outside this group but still open:**
  - `M8/11f-b2-ready-join` (:1223)
  - `M8/marked-row-no-ctrlp` (:1207)
  - `M8/whitespace-tail-home` (:1191)
  - `M8/blame-ws-default-twice` (:1190)
  - `M8/look-sizes-2` (:1195)
  - `M8/11-allow-no-reason` (:1160)

  Close them on those reviews' word; I did not re-check them.

Rows that stay open, correctly:

- `M8/24-brew-style` (:1282)
- `M8/24-no-version-flag` (:1283, see m3)
- `M8/24-linux-56-red` (:1284)
- `M8/24-git-floor` (:1285)
- `M8/23-readme-deferred-unchecked` (:1276)
- `M8/30-captures-changed` (:1288)

## Feature pipeline (23, 24)

- **Need and Prior art:** design note rows "Man page", "Binaries, tap, installer" (`docs/design-notes/m8-polish-and-ship.md:73,75`),
  citing `docs/research/install-docs.md` (present).
- **Journeys:**
  - J16 install = 2 (`:135`, homebrew-core 2), and J17 man = 1 (`:248`).
  - Both are a person's steps in `TESTING.md:3220–3265` with the same counts: 1 step and 2 steps.
  - Neither can be walked yet: J16 needs a release, and J17 needs his read.
