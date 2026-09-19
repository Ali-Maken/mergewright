# acceptance-remotes: one command that walks "you never leave Graft to fetch, pull or push", what right looks like, and the captures

Status: draft
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

- [ ] `cargo run -p graft --example m4_walk` prints `ok <name>` for every scenario (≥ 19) and exits 0; `--only <name>` runs one; a deliberately broken expectation prints `FAIL <name>: …` and exits 1 (revert it; name the run in NOTES)
- [ ] `tests/it/m4_walk.rs` runs the same list under `cargo nextest run` as one test per scenario plus `every_scenario_has_a_test`; slowest scenario < 3 s; the fast tier's wall time recorded in TESTING.md's top table
- [ ] `TESTING.md` "Remotes": one numbered entry per scenario naming exact refs/bytes/status text; the proves/does-not-prove list; per-breakpoint and per-terminal tables
- [ ] Captures: the eight listed, at both sizes, each under 8 KB, `pty-walk-m4.py` asserting no password bytes in any capture; `docs/shots/35-coverage-matrix.txt` gains the remotes table without losing the other two (`m4_walk --matrix`, checked by `captures_are_named_keyed_and_listed_in_the_matrix`)
- [ ] `grep -rn "<the walk's passwords>" docs/shots TESTING.md target/nextest crates/*/target 2>/dev/null` finds nothing — run once, recorded in NOTES
- [ ] KNOWN_GAPS.md rows added/closed as listed; README and CHANGELOG M4 sections present
- [ ] Every tick above names its test or the command that verified it

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` · `cargo nextest run` · `cargo test --doc`. The release perf run and the heavy tier are the reviewer's (ADR-0021); record the commands in TESTING.md, leave the figures `not run`.

## Size

One context. The M3 walk cost one Fable run; this one has fewer scenarios with more setup each — copy `m3_walk`'s harness, add remotes. Scout `tests/it/m3_walk.rs`'s helper names first (haiku, ≤ 1 page); say so. Stop seam: after the `Local` scenarios land (pieces 1–2 partial) with HTTP/hung scenarios and captures in a handoff note.
