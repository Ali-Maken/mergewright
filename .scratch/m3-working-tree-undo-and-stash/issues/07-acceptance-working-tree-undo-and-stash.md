# acceptance-working-tree-undo-and-stash: one command that walks "you cannot lose work", what right looks like, and the captures

Status: done (2026-09-19, builder-fable; KNOWN_GAPS rows left to the conductor — see NOTES)
Blocked by: 01, 02, 03, 04, 05, 06 — every other M3 group
Build with: Fable — the milestone's acceptance ticket (CLAUDE.md standing rule); last in order, before review. Does not count against the Fable build expectation.
Needs: every other M3 group — `ops-model`, `git-reads-and-patches`, `ops-execution`, `changes-pane`, `commit-editor`, `undo-and-stash-ui`.

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — whole (two pages)
- `docs/adr/0029` §7 (daily use is a test tier), `docs/adr/0030` "What this binds", `docs/adr/0033` (whole)
- `PROMPT.md` §3 "Undo and redo" (the untracked test), §7.2 items 3–4, §8 M3 row
- `TESTING.md` "Times I left" (lines 26–45), "Details and diffs" (lines 875–1090: the M2 walk's structure — stops, "what the walk proves and what it does not", per-breakpoint, per-terminal), and the top table (update the measured figures)
- `.scratch/m2-details-and-diffs/issues/23-acceptance-details-and-radar.md` "Built" + its log lines in `docs/log/m2.md` (lines 78–79: the pyte pitfalls — alt-screen, APC; `run_headless` seeds no config)
- `crates/graft/MAP.md` §update (`Harness`, `prelude`, `Banner`), `examples/gallery`, `examples/budgets`; `crates/graft-fixtures/MAP.md` (`copy_into_temp`, `Scratch`)
- `KNOWN_GAPS.md` rows this milestone opened (grep `M3`), and the two "Unverified" colour rows (line 469) — the capture pipeline still carries no colour

## Scope

In:

1. **The walk** — `cargo run -p graft --example m3-walk -- [--capture] [--only <scenario>]`, `examples/m3_walk.rs` with its pure half in `tests/it/m3_walk.rs` (as the gallery does). Each scenario: copy a corpus shape into a `Scratch`, script the dirty state with plain `std::fs` + `graft_fixtures::git`, drive the real app through `Harness::prelude` keys headless (`run_headless`), then **assert repository truth by running git** (`for-each-ref`, `ls-files -s`, `status --porcelain=v2`, file bytes), never by reading `AppState`. Prints one line per scenario: `ok <name>` / `FAIL <name>: <what differs>`, exit 1 on any failure. Scenarios, in this order:
   - `stage-file`, `unstage-file`, `stage-hunk`, `stage-line` (the second `+` line of a two-line hunk only), `stage-all`
   - `discard-tracked-unstaged` → `u` → bytes back; `discard-untracked-binary` (`[0x00, 0xff, 0x0a, 0x0d, 0xfe]`) → `u` → bytes back; `discard-staged-and-unstaged` → `u` → index *and* worktree bytes back
   - `commit` → `u` → HEAD back, index re-staged → `U` → commit back; `commit-refused-nothing-staged`; `amend` → `u` → old message and old tree; `commit-on-unborn-head` → `u` → branch gone, HEAD unborn
   - `stash-save` → `u` → `U`; `stash-pop` → `u` → stash back, tree clean; `stash-apply` → `u`; `stash-drop` → `u` → entry back (with a `git gc --prune=now` between drop and undo); `stash-apply-refused-dirty-touched-file`
   - `undo-drift-guard`: edit after commit, `u` → the box, Stop → nothing changed; `u` → `→` `Enter` → undone and the edit is in the undo entry's backup (assert `git ls-tree <backup>^{tree}` has `worktree/` and the edited blob)
   - `interrupted-entry`: a hand-written Pending line → launch → footer notice text present → `u` restores
   - `linked-worktree`: an op in a `git worktree add` copy lands in that worktree's journal only
   - `refresh-on-focus`: edit a file out of band, scripted `FocusGained`, the pane's counts change without a history restart
   - `no-index-write-on-read`: `.git/index` bytes unchanged across a launch + pane open + `J` + `S` with no write
   Every scenario ends with `git fsck --no-dangling` clean and the backup ref count equal to the number of worktree/index-touching entries.
2. **`TESTING.md` "Working tree, undo and stash"** section: the command; one entry per scenario saying **concretely what right looks like** (`after u, src/a.rs is 7 bytes 00 ff 0a 0d fe … and git status shows ?? src/a.rs` — never "looks correct"); "What the walk proves and what it does not" (pty-driven, character grids, no colour, no real `$EDITOR`, no signing prompt, no real terminal focus event); per-breakpoint rows for the changes pane, editor, stash, journal at 80×24 / 120×40 / 160×48; per-terminal rows (Ghostty, Zed, VS Code) for the user's by-eye pass, each `not run`; the **daily-use start**: the sentence that says "Times I left" and `docs/field-notes.md` collect from today (ADR-0030/0033), with `docs/field-notes.md` created if absent (a heading and the one-line format).
3. **Captures** under `docs/shots/`: `.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py` (from M2's script: pyte with alt-screen and APC handling; **run only with `--capture`**, raw streams stay under `target/`), producing `m3-<nn>-<scenario>-<size>.txt` for the eight scenarios with a visual result (changes pane, hunk cursor, confirm box, editor, editor refused, stash list, stash prompt, journal with `← u`, drift box) at 80×24 and 160×48; every file under 8 KB, hard cap 200 KB. Captures go straight to files; the report names paths only.
4. **Perf line:** `docs/perf/acceptance-m3.txt` from `scripts/m3-snapshot-cost.sh` (ops-execution's script) — the builder does **not** run it; the ticket records the command for the reviewer and the line stays `not run` until the review. Fast-tier figure (`cargo test --quiet` wall time) recorded in the TESTING.md table.
5. **Context guard:** the walk prints one line per scenario; captures and git outputs go to files; a failing scenario prints at most 20 lines of the difference. The builder reads summaries, never a capture.
6. **KNOWN_GAPS.md**: one row per M3 deferral the note names (hunks in untracked files, `i`, message history, selected-files stash, branch-from-stash, `notify` watcher, clear-backups action, in-progress refusal, signing prompt unverified, `Ctrl+Enter` outside kitty keyboard) with expiry; the R2-3 snapshot-cost row re-worded to point at the perf line; the m2/06b row closed. `README.md` and `CHANGELOG.md` M3 sections (what works, the keys, the "Times I left" ask).

Out: fixing what the walk finds (report it: a failing scenario is a finding for the conductor, filed as a ticket, as m2/23 → m2/24 did); the heavy tier; running anything in a real terminal (the user does).

## Signatures

```rust
// tests/it/m3_walk.rs (pure half, included by #[path] from examples/m3_walk.rs)
pub struct Scenario { pub name: &'static str, pub shape: Shape, pub setup: fn(&Path) -> std::io::Result<()>, pub keys: Vec<KeyPress>, pub expect: fn(&Path) -> Result<(), String>, pub capture: Option<&'static str> }
pub fn scenarios() -> Vec<Scenario>;
pub fn run_scenario(s: &Scenario, capture: Option<&Path>) -> Result<(), String>;   // Scratch → setup → headless run with prelude keys → expect → fsck
pub fn git_truth(repo: &Path) -> RepoTruth;                                        // refs (sans refs/graft), index entries, every file's bytes incl. untracked
#[derive(PartialEq, Eq, Debug)] pub struct RepoTruth { pub refs: BTreeMap<String, String>, pub index: Vec<String>, pub files: BTreeMap<PathBuf, Vec<u8>> }
pub fn diff_truth(before: &RepoTruth, after: &RepoTruth) -> Vec<String>;           // ≤ 20 lines, what differs
```

## Acceptance

Every tick names what verified it, at tick time (ADR-0035).

- [x] `cargo run -p graft --example m3_walk` prints `ok <name>` for all 22 scenarios and exits 0 — run 2026-09-19, `22 ok, 0 failed`, 12.1 s wall. `--only stash-drop` runs one (`ok stash-drop`, exit 0). A deliberately broken expectation (`"shout three"` → `"shout four"` in the `commit` scenario, reverted) printed `FAIL commit: subject "shout three\n"` and exited 1.
- [x] `tests/it/m3_walk.rs` runs the same list under `cargo test` — as **one test per scenario**, `m3_walk::every_scenario::{stage_file, unstage_file, stage_hunk, stage_line, stage_all, discard_tracked_unstaged, discard_untracked_binary, discard_staged_and_unstaged, commit, commit_refused_nothing_staged, amend, commit_on_unborn_head, undo_drift_guard, stash_save, stash_pop, stash_apply, stash_drop, stash_apply_refused_dirty_touched_file, interrupted_entry, linked_worktree, refresh_on_focus, no_index_write_on_read}`, plus `every_scenario_has_a_test` (the walk's list equals the test list). Slowest scenario test 1.36 s (`commit`); the 22 together 4.6 s under nextest. Fast tier whole workspace: 40.3 s and 38.4 s warm on two runs, 36.2 s without the walk tests (`cargo nextest run -E 'not test(m3_walk)'`), 21.7 s recorded the morning before — the walk explains 2–4 s of the rise, not the rest (finding 2).
- [x] `TESTING.md` "Working tree, undo and stash": 22 numbered entries each naming bytes, blob ids, list lines or exact git output (`grep -c '^[0-9]*\. \*\*' TESTING.md` → 22 in that section); "What the walk proves and what it does not"; per-breakpoint table (80x24 / 120x40 `not run` / 160x48); per-terminal table (Ghostty, Zed, VS Code `not run`); the "From today" paragraph pointing at "Times I left" (left empty) and `docs/field-notes.md` (already existed; left as is).
- [x] Captures: 18 files `docs/shots/m3-<nn>-<what>-{80x24,160x48}.txt`, largest 2652 bytes (`ls -la docs/shots/m3-*`), written by `pty-walk-m3.py`; every one reports `settled` with its expected text on screen. `docs/shots/35-coverage-matrix.txt` gains the working-tree table via `m3_walk --matrix`; `m3_walk::every_scenario::captures_are_named_keyed_and_listed_in_the_matrix` checks nine captures, distinct stems, keys, expected text and their presence in the table.
- [x] `docs/perf/acceptance-m3.txt` exists with the command and `review: not run` (read back).
- [ ] KNOWN_GAPS.md rows — **not done by the builder**: the conductor's dispatch forbade editing `KNOWN_GAPS.md`; the rows are drafted under NOTES for the conductor to file. README.md "What M3 actually does" and CHANGELOG.md "M3" sections written; README's "Not in M3" list names every deferral so nothing implies a deferred feature exists (read back).
- [x] The report (and NOTES below) says which scenarios git verified (all 22) and which remain by eye (nine captures at two sizes, 120x40, three terminals, colour), and lists findings as findings, not fixes.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

~700 lines (walk + scenarios) + docs. Seam: after piece 1 (the walk runs), then docs and captures. Reads summaries only; a capture is never read into context.

## NOTES (builder-fable, 2026-09-19)

**Scouts:** three `Explore` scouts on Sonnet (harness mechanics; M3 key flows and exact strings; ops/journal/fixtures facts). The builder read source only for the prelude (42 lines), `Field::next`, `refusal`, `plan_discard`'s head, and the shared `app_harness.rs` it builds on.

**Where the build differs from this spec (the conductor's as-built file is theirs; recorded here):**

1. **`Harness::prelude` + `run_headless` cannot drive the walk.** `run_headless` takes no keys and prints one frame; the prelude is a flat key list with no way to wait for a write to land or edit a file between keys. The walk drives `update` directly over the real `JobRunner` (`tests/it/app_harness.rs::App`, as `undo_by_key`/`stash_by_key` do) through `Step`s: `Key`, `Type`, `Await(pred)`, `Write` (settle a queued write, demand `Done`), `Settle`, `CursorTo(name, nth)`, `Focus`, `Shell(fn)`, `Mark`, `See`, `CommitButton`. Signatures changed accordingly: `Scenario { name, shape, setup: fn(&Path) -> Result<(), String>, open: Option<&str>, steps: Vec<Step>, expect: fn(&Path, &Marks) -> Result<(), String>, capture: Option<Capture> }`; `git_truth -> Result<RepoTruth, String>`; `RepoTruth` gains `head`, `stash`, `index_bytes` (the last compared only by `no-index-write-on-read` — a write and its undo legitimately rewrite the file); `run_scenario(&Scenario) -> Result<(), String>`.
2. **The prelude is still the capture path**, and `update::prelude::settled` now also waits for `working_tree`, `operating`, `journal.reading`, a pending stash list, `head_message` and `commit_template` (public behaviour of `Harness::prelude`; MAP.md updated; test `the_prelude_holds_a_key_while_a_write_is_in_flight`; the M2 prelude test now delivers a working-tree answer first). `Capture { stem, headless: Vec<Step>, keys: Vec<KeyPress>, on_screen }` — `headless` runs before the interactive launch (the drift box needs a commit and an edit first).
3. **One test per scenario**, not a single `every_scenario_passes`: 22 serial scenarios would be a 20 s+ test, past PROMPT §7.1's 10 s line.
4. **Capture names** `m3-<nn>-<what>-<WxH>.txt` (no tier suffix: Unicode only). Nine captures, not eight — the spec's own list has nine items. `m3_walk --matrix` writes all three tables; **`gallery --matrix` still writes only its two and would drop the working-tree table** — run `m3_walk --matrix` after it.
5. **Commit goes through `Tab` to `[ Commit ]`** (3 Tabs: Subject → Body → options → button — `Field::next` treats the options row as one stop), found dynamically by `Step::CommitButton`.
6. **`stash-drop`'s gc** is `git gc --prune=now` alone. The spec's `reflog expire --all` empties the stash reflog the scenario then checks (the graft-ops test only checked the top entry).
7. **`interrupted-entry`** builds its torn entry from a real discard through the app with the outcome line deleted, not a hand-written line with no before-state (which `u` could not restore anything from).
8. **`discard-staged-and-unstaged`**: a path discard restores **both** the index and the file to HEAD from either row (`plan_discard`, staged class). The walk asserts that. See finding 1.

**Findings (not fixed; for the conductor):**

1. **`d` on the UNSTAGED row of a file that also has staged changes discards the staged version too.** Undoable (`u` brings both back, proven), so not loss — but the design note says `d` discards "the same unit" as `Space` per section membership, and a user aiming at the unstaged half loses their carefully staged half. Product preference: keep (simple, undoable) or split by section. Not fixed here.
2. **Fast tier wall time** rose from 21.7 s to 38–40 s over the day; the walk tests account for 2–4 s (36.2 s without them). The rest is unexplained — other M3 tests, or machine load; the 22 walk tests each spawn a job pool and dozens of git processes and contend with the rest of the suite under nextest's parallelism. KNOWN_GAPS line 463's watch applies.
3. `gallery --matrix` overwrites `35-coverage-matrix.txt` without the working-tree table (deviation 4). Small; fix = teach `coverage_matrix` the third table, which needs `tests/it/gallery.rs` to include the walk or vice versa.
4. Group 06's captures `docs/shots/m3-changes-pane-160x48.txt`, `m3-commit-editor-160x48.txt`, `m3-journal-160x48.txt` predate this naming and are not in the matrix. Leave or retire.
5. The pinned uncommitted row (KNOWN_GAPS m2/06b): `k` from the top row reaches it in every pane scenario (17 of 22), all with `HEAD` topmost. Not verified with `HEAD` below the top — the conductor decides whether that row closes.

**KNOWN_GAPS rows to file (conductor; from the design note's deferrals, expiry suggested):** hunks inside untracked files (`add -N`) → M7; `i` for .gitignore → M7; commit message history → M7; stash selected files → M7; branch from stash → M5; `notify` file watcher → M4; clear-backups action → M8; commit/amend refused during merge/rebase/cherry-pick → M5/M6; signing prompt unverified (no walk scenario, no real terminal) → unverified, M8; `Ctrl+Enter` outside the kitty keyboard protocol → unverified, standing; forced undo not pinned to the entry the drift box named → M5 (from group 06); `Esc` on an unlanded journal read re-requests → M8; **R2-3** reword: "snapshot cost measured by `scripts/m3-snapshot-cost.sh`; the acceptance line is `docs/perf/acceptance-m3.txt` (`review: not run`)"; **m2/06b**: close only if finding 5 satisfies.

**Verified how:** all 22 scenarios by running git after the app closed (`for-each-ref`, `ls-files -s`, `stash list`, `status --porcelain`, `rev-parse`, `rev-list --objects`, `hash-object --stdin`, file bytes), plus `fsck --no-dangling` and the backup-ref count per scenario. UI-only facts (`nothing staged` refusal, the interrupted notice, focus counts, the drift box opening and closing) by `AppState`, named as such in TESTING.md. **Not verified:** anything by eye (nine captures × two sizes are for the user), 120x40, Ghostty/Zed/VS Code, colour (captures carry none), `Ctrl+E` external editor, signing, hooks, a real terminal focus event, the snapshot cost.

**Context:** compacted once, after every gate had passed and the doc edits were in; what remained (the TESTING.md fast-tier figure, this ticket update, the commit) was finished in the resumed context. Nothing remains to split. No turn count reported (harness has it).
