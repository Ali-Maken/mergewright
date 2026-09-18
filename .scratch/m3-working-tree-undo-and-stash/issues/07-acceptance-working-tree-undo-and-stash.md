# acceptance-working-tree-undo-and-stash: one command that walks "you cannot lose work", what right looks like, and the captures

Status: ready
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

- [ ] `cargo run -p graft --example m3-walk` prints `ok <name>` for every scenario listed and exits 0; `--only stash-drop` runs one; a deliberately broken expectation (temporarily) prints `FAIL` with the differing lines and exits 1 (then reverted).
- [ ] `tests/it/m3_walk.rs::every_scenario_passes` runs the same list under `cargo test` (fast tier; each scenario is a small fixture copy, laptop-safe; report the added seconds against KNOWN_GAPS line 463's watch).
- [ ] `TESTING.md` section present with every scenario's "what right looks like" line naming bytes, counts or exact git output; per-breakpoint and per-terminal tables with `not run`; the "Times I left" table left empty and pointed at; `docs/field-notes.md` exists.
- [ ] Captures exist at the paths the ticket lists, each under 8 KB; `docs/shots/35-coverage-matrix.txt` gains a working-tree table (`--matrix`).
- [ ] `docs/perf/acceptance-m3.txt` exists with the command and `not run`.
- [ ] KNOWN_GAPS/README/CHANGELOG updated as piece 6 says; nothing in README implies a deferred feature exists.
- [ ] The report says which scenarios were verified by running git and which by eye remain for the user, and lists any scenario that failed as a finding, not as a fix.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

~700 lines (walk + scenarios) + docs. Seam: after piece 1 (the walk runs), then docs and captures. Reads summaries only; a capture is never read into context.
