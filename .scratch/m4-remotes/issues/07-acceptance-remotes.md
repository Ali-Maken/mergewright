# acceptance-remotes: one command that walks "you never leave Graft to fetch, pull or push", what right looks like, and the captures

Status: done
Blocked by: 01–06
Build with: Fable — the milestone's acceptance ticket (CLAUDE.md standing rule); last in order, before review. Does not count against the Fable build expectation.
Crates: graft (examples, tests/it, docs) — plus `TESTING.md`, `KNOWN_GAPS.md`, `README.md`, `CHANGELOG.md`, `docs/shots/`. No library code in any crate.
Needs: every other M4 group — `remote-fixtures`, `git-remote-and-askpass`, `ops-remote-operations`, `ui-credential-prompt-and-hints`, `remote-keys`, `credential-prompt-wiring`.

## Read first

- `docs/design-notes/m4-remotes.md` — whole (two pages)
- `docs/adr/0033-times-i-left.md` (whole); ADR-0035 §4–5 (mockups approved before code; ticks name their test); ADR-0036 §1 (worktree isolation — this group runs alone, after the merge)
- `PROMPT.md` §3 "Long operations", "Feedback" (lines 254–263); §4 Remotes (343–349); §9 "push" bullet (616)
- `TESTING.md` "Times I left" (lines 27–46) and the "Working tree, undo and stash" section (grep the heading) — the M3 walk's structure to mirror: stops, "what the walk proves and what it does not", per-breakpoint, per-terminal rows
- `.scratch/m3-working-tree-undo-and-stash/issues/07-acceptance-working-tree-undo-and-stash.md` — Scope pieces 1–5 and the NOTES (the pyte pitfalls, `--capture`, `--matrix`); `crates/graft/MAP.md` `examples/m3_walk` line (22) and §update `Harness`/`prelude` (116)
- `crates/graft-fixtures/MAP.md` §remote; `crates/graft/MAP.md` §update::remote, §update::credential, §jobs::remote (after those groups land)
- `KNOWN_GAPS.md` rows with expiry M4 (grep `M4`)

## Scope

In:

1. **The walk** — `cargo run -p graft --example m4_walk -- [--capture <name>] [--only <name>] [--list] [--stems] [--matrix]`, `examples/m4_walk.rs` with its pure half `tests/it/m4_walk.rs` (the `m3_walk` shape: `Scenario`, `Step`, `run_scenario`, `git_truth`, one `cargo test` per scenario). Each scenario builds a `LocalRemote` (or `HttpRemote`/`HungRemote` over its bare), drives the real app headless through `Harness::prelude` over the real job pool with `askpass_helper = CARGO_BIN_EXE_graft`, then **asserts repository truth by running git on both the work clone and the bare** — never from `AppState`. Prints `ok <name>` / `FAIL <name>: <≤ 20 lines>`, exit 1 on any failure. Scenarios, in order:
   - `fetch-brings-new-commits` (tracking ref == bare tip; `Fetch` entry; `u` afterwards says nothing to undo and moves no ref) · `fetch-prunes-deleted-branch`
   - `pull-fast-forward` (`main` == tracking tip, file bytes updated; `u` → `main` back + bytes back + tracking ref unchanged; `U` → forward) · `pull-diverged-refused` (main unchanged, tracking ref advanced, status names merge/rebase and the shell command) · `pull-dirty-file-refused` (bytes byte-identical) · `pull-no-upstream-refused`
   - `push-plain` (bare's `main` == local; `Push` entry with tracking ref before/after; `u` undoes the commit locally, status ends `origin/main still has it`, bare unchanged; `U` restores) · `push-sets-upstream` (`branch.x.remote/merge` written) · `push-detached-refused`
   - `push-rejected-then-lease` (remote moved → rejected sticky → local amend → `P` lease box → `Enter` → bare's `main` == local) · `push-lease-refused-stale` (bare unchanged, status names `stale lease`)
   - `wrong-password-then-right` (`HttpRemote`: prompt appears masked; `wrong` → sticky `authentication failed for 127.0.0.1:<port>`; retry with the right one → pushed; `requests()` = (≥1, ≥1); the journal file, the walk's recorded status lines, and every capture file contain neither password) · `password-prompt-esc-cancels` (entry `Aborted` `cancelled`, bare unchanged, `requests().0 == 0`) · `username-then-password`
   - `hung-remote-esc-cancels` (`operating` clears within 5 s, `Aborted("cancelled")`, no `.git/**/*.lock`, `connections() == 1`)
   - `progress-reaches-the-footer` (some recorded status line contains `objects`)
   - `u-never-touches-the-remote` (after fetch, pull, push, undo everything undoable: the bare's `for-each-ref` equals its state right after the push)
   - `journal-rows` (`J`: the push row contains `not undoable — origin has it`, the fetch row `not undoable`, the pull row `← u`)
   - `no-index-write-on-fetch` (`.git/index` bytes unchanged across `f`)
   Every scenario ends with `git fsck --no-dangling` clean in the work clone.
2. **`TESTING.md` "Remotes"** section: the command; one numbered entry per scenario saying **concretely what right looks like** (`after P ⏎, git -C bare rev-parse main == git -C work rev-parse main == a1b2c3d…; the journal's last entry is kind push with refs_before {refs/remotes/origin/main: 9f8e…}` — never "looks correct"); "What the walk proves and what it does not" (loopback HTTP, no TLS, no SSH, no real credential helper, pyte grids without colour, no real terminal focus); per-breakpoint rows for the push box, lease box, credential prompt, progress footer, rejected sticky, journal at 80×24 / 120×40 / 160×48; per-terminal rows (Ghostty, Zed, VS Code) each `not run` for the user's by-eye pass, with what to look for (`the password field shows • only`); the "Times I left" reminder: every remote-related reach for the shell during M4 daily use goes there.
3. **Captures** under `docs/shots/`: `.scratch/m4-remotes/pty-walk-m4.py` (from `pty-walk-m3.py`; run only with `--capture`; raw streams under `target/`) producing `m4-<nn>-<what>-{80x24,160x48}.txt` for: push box plain, push box lease, credential prompt masked (must show `•`, must not show the password — assert in the script), credential prompt username (unmasked), progress footer mid-fetch (may need a `LocalRemote` with a 2 MiB file so progress is on screen long enough; if it cannot be caught, say so and capture the `cancelling…` state instead), rejected sticky, journal with push/fetch/pull rows, `u`-after-push status. Every file under 8 KB, hard cap 200 KB. Captures go straight to files; the report names paths only.
4. **`--matrix`**: the coverage matrix gains the remotes table; fix KNOWN_GAPS' `gallery --matrix overwrites the working-tree table` row by teaching `coverage_matrix` all three tables (it is this group's own tool; a one-function change in `tests/it/gallery.rs` — the only library-adjacent edit allowed here, and it is test code).
5. **KNOWN_GAPS.md**: one row per M4 deferral the note names, marked **unverified-by-design, not broken**, with expiry — clone (M8), add/rename/remove remotes (M7), pull with merge/rebase (M5), per-remote fetch (M7), stall notice/timeout (M7), `q` confirming during an operation (M8), SSH askpass env set but unverified (first field note), `notify` watcher (M7, moved from M4); close the rows M4 closed (`gallery --matrix`, `RepoFacts.remotes` `&[]`). `README.md` and `CHANGELOG.md` M4 sections: what works, the keys, what a rejected push offers, that credentials are asked in-app and never stored.
6. **Context guard**: one line per scenario; failing scenario ≤ 20 lines; the builder reads summaries, never a capture.

Out: fixing what the walk finds (a failing scenario is a finding for the conductor, filed as a ticket); the heavy tier; running in a real terminal (the user does); any change to library crates (report `blocked: needs <crate>` instead).

## Signatures

```rust
// tests/it/m4_walk.rs (pure half, included by #[path] from examples/m4_walk.rs)
pub enum RemoteKind { Local, Http { user: &'static str, password: &'static str }, Hung }
pub struct Scenario { pub name: &'static str, pub shape: Shape, pub remote: RemoteKind, pub setup: fn(&LocalRemote) -> io::Result<()>, pub steps: Vec<Step>, pub expect: fn(&Truth) -> Result<(), String>, pub capture: Option<&'static str> }
pub enum Step { Keys(Vec<KeyPress>), WaitFor(Wait), TypeSecret(&'static str) }   // TypeSecret types into the credential prompt and is never echoed by the walk
pub enum Wait { Idle, CredentialPrompt, StatusContains(&'static str), Operating(bool) }
pub struct Truth { pub work: RepoTruth, pub bare: BTreeMap<String, String> /* for-each-ref */, pub journal: Vec<JournalEntry>, pub statuses: Vec<String>, pub http: Option<(usize, usize)>, pub hung_connections: Option<usize> }
pub fn scenarios() -> Vec<Scenario>;
pub fn run_scenario(s: &Scenario, capture: Option<&Path>) -> Result<(), String>;
pub fn truth(remote: &LocalRemote, statuses: Vec<String>, …) -> Truth;
pub fn diff_truth(before: &Truth, after: &Truth) -> Vec<String>;   // ≤ 20 lines
```

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035).

- [x] `cargo run -p graft --example m4_walk` prints `ok <name>` for every scenario (≥ 19) and exits 0; `--only <name>` runs one; a deliberately broken expectation prints `FAIL <name>: …` and exits 1 (revert it; name the run in NOTES) — **verified by** `./target-g07/debug/examples/m4_walk` (19 `ok` lines, exit 0, `target-g07/walk/run2.txt`), `--only journal-rows` (`target-g07/walk/journal-rows2.txt`), and the broken run in NOTES (`target-g07/walk/broken.txt`, exit 1); the list itself by `m4_walk::every_scenario::every_scenario_has_a_test`
- [x] `tests/it/m4_walk.rs` runs the same list under `cargo nextest run` as one test per scenario plus `every_scenario_has_a_test`; slowest scenario < 3 s; the fast tier's wall time recorded in TESTING.md's top table — **verified by** `cargo nextest run` (`m4_walk::every_scenario::*`, 19 scenario tests + 4 others, all pass; slowest `u_never_touches_the_remote` 1.408 s, from `--status-level pass` in `target-g07/walk/nextest-walks.txt`); TESTING.md top table says 34.3 s test time / 1822 tests and that the warm wall figure was not measured
- [x] `TESTING.md` "Remotes": one numbered entry per scenario naming exact refs/bytes/status text; the proves/does-not-prove list; per-breakpoint and per-terminal tables — **verified by** reading: 19 entries, each quoting the walk's assertion strings (`sticky_containing`/`done_containing`/`prompt_is` texts and the git relations from each `expect`); breakpoint table quotes lines from the 16 capture files; 120x40 and the three terminals `not run`
- [x] Captures: the eight listed, at both sizes, each under 8 KB, `pty-walk-m4.py` asserting no password bytes in any capture; `docs/shots/35-coverage-matrix.txt` gains the remotes table without losing the other two (`m4_walk --matrix`, checked by `captures_are_named_keyed_and_listed_in_the_matrix`) — **verified by** `GRAFT_TARGET=target-g07 python3 .scratch/m4-remotes/pty-walk-m4.py 80x24` / `160x48` (exit 0 both, 16 files 498–7992 bytes, `target-g07/walk/pty-*.txt`), the script's own secret/`•` checks (lines 200–208), `m4_walk::every_scenario::captures_are_named_keyed_and_listed_in_the_matrix` (8 captures, every stem in `remotes_matrix`, no secret in `--stems` output) and `gallery::the_matrix_marks_only_what_exists_by_name`; `--matrix` run once, the file holds four tables (graph, details, working tree, remotes)
- [x] `grep -rn "<the walk's passwords>" docs/shots TESTING.md target/nextest crates/*/target 2>/dev/null` finds nothing — run once, recorded in NOTES — **verified by** the grep in NOTES (exit 1, no match); plus `no_secret_leaked` after every scenario (`m4_walk::every_scenario::*`)
- [x] KNOWN_GAPS.md rows added/closed as listed — filed by the conductor at 5385da5 and this commit (3 rows from NOTES: Ctrl+R race, auth URL, walk coverage limits) — **not done: the conductor asked this builder not to touch KNOWN_GAPS.md, PROGRESS.md or docs/log/**; the rows to file are listed in NOTES. README and CHANGELOG M4 sections present — **verified by** reading `README.md` "What M4 actually does" and `CHANGELOG.md` "M4 — Remotes"
- [x] Every tick above names its test or the command that verified it — by inspection of this list

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` · `cargo nextest run` · `cargo test --doc`. The release perf run and the heavy tier are the reviewer's (ADR-0021); record the commands in TESTING.md, leave the figures `not run`.

## Size

One context. The M3 walk cost one Fable run; this one has fewer scenarios with more setup each — copy `m3_walk`'s harness, add remotes. Scout `tests/it/m3_walk.rs`'s helper names first (haiku, ≤ 1 page); say so. Stop seam: after the `Local` scenarios land (pieces 1–2 partial) with HTTP/hung scenarios and captures in a handoff note.

## NOTES

**Compacted once**, after the captures and gates were green; what remained
(TESTING.md section, ticket update, commit) was finished in the same
session. Nothing is left over.

**Scout:** one Sonnet `Explore` over `tests/it/m3_walk.rs`, `app_harness.rs`,
`examples/m3_walk.rs`, `pty-walk-m3.py` and `graft-fixtures::remote` (helper
names, prelude/settle semantics, the fixture API). Used its summary; read
source only where the walk needed exact types.

**What was built:** `crates/graft/tests/it/m4_walk.rs` (pure half, 19
scenarios, 23 tests), `crates/graft/examples/m4_walk.rs` (the command; also
its own askpass helper — `CARGO_BIN_EXE_graft` exists only in the test
binary, so the example answers `GRAFT_ASKPASS_SOCKET` itself via
`cli::askpass` + `respond`, and passes `current_exe` as the helper),
`.scratch/m4-remotes/pty-walk-m4.py`, 16 captures, `35-coverage-matrix.txt`
with four tables (`coverage_matrix` now appends the M3 and M4 tables —
`gallery --matrix`, `m3_walk --matrix`, `m4_walk --matrix` all write the
same file), TESTING.md "Remotes", README/CHANGELOG M4 sections, MAP.md line.
`m3_walk::{neutralise_global_config, fsck_clean}` made `pub` for reuse.

**Deviations from the Signatures block** (shape follows `m3_walk`, as the
ticket's "the `m3_walk` shape" asks):
- `RemoteKind::Http { anonymous: bool }` — the user/password are constants
  (`USER`, `PASSWORD`, `WRONG`); the flag is whether the URL carries the user
  (`username-then-password` needs it absent).
- `Scenario.setup: fn(&Fixture) -> Result<(), String>` and `expect:
  fn(&Fixture, &Marks) -> Result<(), String>` — `Fixture` wraps the
  `LocalRemote` plus the optional `HttpRemote`/`HungRemote`; `Marks` holds
  the truth `before` and at every `Step::Mark`, so an expectation compares
  states (`same_bare(before, after)`) rather than one `Truth`.
- `Step` is the M3 set plus `TypeSecret`, `Cancel`, `Shell`, `Mark`, `See`;
  `Wait` is `Ready | CredentialPrompt | Connected | JournalHas(kind)` —
  `Idle`/`StatusContains`/`Operating` were not needed (`See` checks status
  text against the settled state).
- `Truth` as specified, plus `work.index_bytes` for `no-index-write-on-fetch`.

**Findings (for the conductor; none fixed here):**
1. **Refresh race in the app (not the walk):** `awaiting` (the resident
   history job's id) is set only on the `Started` event. A `Ctrl+R` issued
   before the previous refresh's `Started` lands cannot cancel that older
   history job; two resident history jobs then fill a two-worker pool and
   `ReadRefs`/`ReadRemotes`/`ReadWorkingTree` never run. Seen in
   `journal-rows` as a 120 s timeout. The walk now waits for `Ready` before
   `Ctrl+R` (as a person's screen would settle); a `Ctrl+R` right after `p`
   in the real app with a small pool could still do this. Worth a ticket.
2. **Auth-failed line shows the full URL**, `authentication failed for
   http://127.0.0.1:<port>/repo.git/ · P to try again`, where the design
   note pictured `host:port`. Asserted as built; cosmetic, the conductor
   decides.
3. **The push box draws no frame** at 80x24 or 160x48 (`m4-01-*`): title,
   message, command and buttons over the graph, the graph's `Comm` visible
   left of the box. Whether that reads as a box is the user's call.
4. **80 columns truncates the rejection hint** to `… or P to fo…`
   (`m4-06-rejected-sticky-80x24.txt`) and the journal's push row to `not
   undoa…`. The key the person needs (`P`) is the cut part.
5. `last_rejected_push` is in memory only: a fresh run cannot show the lease
   box unless it is rejected in that run, so `m4-02` is captured with keys
   `P → ⏎ P` (rejected inside the run) rather than a headless prefix.
6. Remote footer hints (`f fetch  p pull  P push`) need ≥ 156 columns; none
   at 80x24 — by design.

**FAIL path, named run:** `fetch-brings-new-commits`'s `nothing to undo`
check was changed to look for `nothing to undo (deliberately broken)`;
`./target-g07/debug/examples/m4_walk --only fetch-brings-new-commits`
printed `FAIL fetch-brings-new-commits: u says nothing to undo: the line
lacks \`nothing to undo (deliberately broken)\`: nothing to undo` and exited
1 (`target-g07/walk/broken.txt`); reverted (grep count 0), rebuilt, rerun
green.

**Secrets grep:** `grep -rln "correct-horse-battery-staple\|not-the-password"
docs/shots TESTING.md README.md CHANGELOG.md` → no match, exit 1. The
nextest output under `target-g07/walk/*.txt` searched the same way: no
match. (`target/nextest` and `crates/*/target` do not exist in this
worktree; the target directory is `target-g07`.)

**KNOWN_GAPS rows for the conductor to file** (this builder was told not to
touch the file). Add, marked *unverified by design, not broken*, with
expiry: clone (M8); add/rename/remove remotes (M7); pull with merge or
rebase (M5); per-remote fetch (M7); stall notice / timeout (M7); `q`
confirming during an operation (M8); SSH askpass env set but unverified
(first field note); `notify` watcher (M7, moved from M4); the password sits
in a plain `String` in the prompt's `TextField`, not zeroed (M7 or later);
remote footer hints only at ≥ 156 columns (design). Plus finding 1 above
as *broken (app-level race)*. Close: `gallery --matrix overwrites the
working-tree table` (fixed here, `coverage_matrix` writes four tables);
`RepoFacts.remotes` `&[]` (M4 groups 01–06 filled it; the walk's captures
show `origin/` refs).

**Not verified here:** anything by eye; 120x40; Ghostty/Zed/VS Code;
colour; SSH; a real credential helper; the heavy tier and the release perf
run (commands recorded in TESTING.md, figures `not run`).

**Gates (workspace-wide, `CARGO_TARGET_DIR=target-g07`):** `cargo fmt
--check` ok; `cargo clippy --all-targets --message-format=short -- -D
warnings` ok; the same with `--features heavy-tests` ok; `cargo nextest
run` 1822 passed, 22 skipped, 34.3 s; `cargo test --doc` ok. Outputs under
`target-g07/walk/{fmt,clippy,clippy-heavy,nextest,doctest}.txt`.
