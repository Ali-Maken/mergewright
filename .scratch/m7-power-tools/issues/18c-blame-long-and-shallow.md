# 18c blame-long-and-shallow — record blame's long-file wait (ADR, no code); a shallow boundary ends a trace truthfully

Status: done
Blocked by: none (18b merged at `79b801f5`)
Crates: graft-git only (`trace/log.rs`, `trace/follow.rs`, a new `trace/shallow.rs` if it helps the 400-line rule,
their tests), graft-git `MAP.md`, and `docs/adr/0050-blame-waits-for-git.md`. Needing another crate → commit at the
seam, report `blocked: needs <crate>` with the signature, stop.
Build with: **Sonnet**. Part 1 is a doc copied from this spec. Part 2 is a small read-only change pinned by oracle tests
on a real `--depth` clone. Nothing structural.
Scout: none. 18b's NOTES name the site (`trace/log.rs:156-157`); re-read it before editing.
Split by: planner, 2026-09-26, from 18b NOTES (Major 3 over budget; the shallow finding) and review
`docs/reviews/m7-18-19-27-git-reads.md` §Major 3.

## Read first
- 18b's NOTES: "Major 3 — over budget" and "New finding".
- `docs/design-notes/m7-power-tools.md` Pass 2b B1 (title states) and H1 (States: the end row).
- `docs/research/blame-line-history.md` §3, §4.10.
- `crates/graft-git/MAP.md`: `trace`, `RepoHandle::common_dir`, `run` (`RunError`).

## Part 1 — blame on a long file: no code. Decision and reasoning
**Measured:** `blame-longfile p95=1481.4` ms (`docs/perf/trace-readers.txt:33`): 5 000 lines, 500 commits, `-M -w`,
release, load uncontrolled. All of it is git's; our parse of the 1-line file was 14 ms.

**Accept it for M7. Keep the async reading state already specified. Record a budget in ADR-0050.** Why:
- **The wait is git's, and every tool the study names pays it too.** Study §3: "Blame cost is git's cost" (GitLab
  #586593, the 30k-line git-list report). §4.10: CLI git beats libgit2, so we already use the fastest engine. No
  surveyed tool is reported faster on the same file.
- **He already sees something while he waits.** The title reads `blaming <path>… · esc cancels` (B1; 20c "States"),
  with no keys lost (note, "Where he lands"). That meets UX principles H1 (visibility of system status). 1.5 s is past
  the 1 s flow limit, but it's far short of the 10 s point where a progress bar is needed. git can't give a percentage
  anyway: `--progress` is refused with `--porcelain` (from git's docs, unverified here).
- **Incremental blame (`--incremental`) changes nothing about the total.** What it would buy: lines drawn as git
  attributes them, and the line he pressed `B` on answered at once when that commit added it (git blames newest
  first). What it costs:
  - stdout streaming in `run`, which doesn't exist (MAP `run`: stderr only; 18 NOTES say the traces need it too);
  - a second parser and fuzz target;
  - a partial state in the blame place (graft and graft-ui);
  - checking that `--incremental` carries the `?`/`*` marks (unverified).
  It's also a **new screen state**, so pipeline steps 2 and 5 apply: the study doesn't record how tig, Magit or
  GitLens draw a blame that's still loading (my memory says tig and Magit fill it in as it arrives, but that's
  unsourced), and nothing has been prototyped. So it can't be specified from this pass.
- **Re-runs pay it again.** `w`, `i` and `^` each start a new blame, so `B ^` on such a file is about 3 s. The ADR says
  so, so this isn't hidden.

**ADR-0050 text** (the builder writes `docs/adr/0050-blame-waits-for-git.md` with exactly this content):
> **Blame waits for git; the wait is shown, not budgeted as a frame.** Status: accepted (M7, 2026-09-26).
> Context: `git blame --porcelain -M -w` on a 5 000-line file with 500 interleaved commits takes p95 1 481 ms
> (release, git 2.54, Apple M5; `docs/perf/trace-readers.txt`). The study (§3) finds blame's cost is git's in every
> tool that uses it.
> Decision: blame is a job, not a frame (ADR-0021's frame budgets don't apply to it). While it runs, the title reads
> `blaming <path>… · esc cancels` and `Esc` cancels. `w`, `i` and `^` each re-run it and pay the same wait.
> Budget: the milestone review's release run reads `blame-longfile p95` from `scripts/m7-perf-blame-longfile.sh`. Over
> **3 000 ms** (2× today) is a regression → a finding.
> Revisit, starting with a researcher addendum to `blame-line-history.md` §3 (how tig, Magit and GitLens show a blame
> still loading, with links), then a prototype, when **either** a field note reports waiting on blame **or** a real
> file he blames measures over 1 000 ms.
> Rejected for now: `--incremental` (cost above; a new screen state with no research or prototype yet); libgit2
> blame (study §4.10); turning `-M` off by default (study §4.3 takes it; its cost is unmeasured, so the cost alone is
> no reason to drop it).

**Question for him** (the conductor relays it at the checkpoint; the default is accept): "On a big, much-edited
file, blame shows 'blaming…' on an empty screen for about 1½ s. Fine, or worth a bigger job so the lines appear while
it works?"

## Part 2 — a shallow clone's boundary

**What he sees today** (18b, observed): the history of lines 18–29 in a `--depth 2` clone ends with
`the file starts here (<Priya> added it)`. That's false: Tom added them. Git diffs the boundary commit against
nothing, so all of its lines count as adds. `file_history` does the same: the boundary's raw status is `A`.

**Decided:**
- **Detection:** read `<common_dir>/shallow` (git's own list of boundary ids, one hex id per line).
  - Missing (`NotFound`) → not shallow.
  - Any other read error → `Err`; never a silent `Start` (rule 3).
  - Why not `rev-parse --is-shallow-repository`: it answers yes or no, not *which* commits are boundaries.
  - Read once per trace, after git exits. No spawn.
- **Rows:** drop every row whose id is a boundary. Its `+/−` counts and its file-pane marks come from diffing against
  nothing, so we can't know them. Dropping it claims nothing false.
- **End:** if any row was dropped, or `Start{added_by}` names a boundary, the end is
  `TraceEnd::GitStops(SHALLOW_END.into())`, with `pub const SHALLOW_END: &str = "this clone is shallow; nothing older
  was fetched";`. The history place already draws `git stops tracing here: <reason>` (H1), so graft doesn't change.
  - `Capped` still wins when the cap is reached first.
  - A trace that reaches its real adding commit inside the fetched history still ends as `Start`.
- **The `(_, false)` arm** (`log.rs:157`, a log that ends on a change): keep what it does, but stop it guessing "a
  shallow clone?", since shallow is now detected. The new words are `history ends here without the commit that added
  these lines`. Update M17's test if it asserts the old text.
- **Error:** add `TraceError::Shallow { path: PathBuf, source: std::io::Error }`, `Display`
  `couldn't read <path>: <source>`. Don't reuse `RunError::Spawn`, whose words say "could not run git".

**Signatures** (graft-git; export `SHALLOW_END` from `trace`):
```rust
pub const SHALLOW_END: &str = "this clone is shallow; nothing older was fetched";
pub enum TraceError { Run(RunError), Parse(ParseError), Shallow { path: PathBuf, source: std::io::Error } }
pub(crate) fn shallow_boundaries(repo: &RepoHandle) -> Result<BTreeSet<Oid>, TraceError>;
// line_history / file_history: signatures unchanged.
```

**Out of scope:** blame in a shallow clone. It likely credits pre-boundary lines to the boundary commit, the same
false name, and `^` there would say "nothing before it". This is **unverified**, so it's a question, not a defect.
The builder observes it (below) and reports. The fix is app-side (graft, graft-ui) and goes to its own ticket or a
KNOWN_GAPS row.

## Acceptance (each box names its test, red first where marked)
All in `crates/graft-git/tests/it/trace.rs`. Clones use `git clone --depth N file://<Session fixture>`: a plain local
path ignores `--depth`.
- [x] Red first (seen red: `Start{Priya}`-shaped end, line 566): `a_shallow_boundary_ends_line_history_as_shallow_not_start`. Depth 2, lines 18–29: no row for the
  boundary id; the end is `GitStops(SHALLOW_END)`. Today it gives `Start{Priya}`.
- [x] Red first (seen red): `a_shallow_boundary_ends_file_history_as_shallow_not_start`.
- [x] `a_depth_one_clone_traces_no_rows_and_says_shallow` (seen red): empty sink, `GitStops(SHALLOW_END)`.
- [x] `a_shallow_clone_deep_enough_still_ends_at_the_add` (depth 6; asserts `shallow` lists Ana's commit): pick the depth so Tom's commit is inside the history and
  isn't a boundary → `Start{Tom}`. The ids match `git log -L` run in the same clone.
- [x] `a_linked_worktree_of_a_shallow_clone_sees_the_boundary` (seen red). Oracle: `git rev-parse --git-path shallow` in the
  worktree names the same file as `<common_dir>/shallow`.
- [x] `an_unreadable_shallow_file_is_an_error` (red run was cancelled by nextest fail-fast, so red not observed) (e.g. `shallow` is a directory) → `Err(Shallow{..})`, not
  `Start`.
- [x] Still green (`trace` filter, 34/34):  `trace_end_is_start_on_the_adding_commit`, `trace_capped_says_how_many`, M17's
  `a_log_ending_on_a_change_not_an_add_is_not_a_start` (its expected text updated to the new words, via `NO_ADD`).
- [x] NOTES only (see NOTES): in a depth-2 clone, `blame` at HEAD. Which commit does line 18 name, and is `BlameCommit::boundary`
  true? Observed on git 2.54 (and 2.38.5 if 18b's build is still under `$TMPDIR`); say which.
- [x] `docs/adr/0050-blame-waits-for-git.md` holds Part 1's text (a doc, not tested). MAP.md `trace`: `SHALLOW_END`,
  `TraceError::Shallow`, the boundary rule.
- [x] Gates, workspace-wide (fmt ok, clippy ok, nextest 3839/3839, doc ok): fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches (check each; NOTES says how)
- **`TraceError`'s new variant:** grep `TraceError` in every crate outside graft-git. An exhaustive `match` there →
  `blocked: needs graft`, naming the arm. A `Display`-only use is fine.
- **The history place (graft, graft-ui):** it draws `GitStops` as `git stops tracing here: …` (H1). Grep the
  `GitStops` handling to confirm the reason is drawn verbatim. **Zero rows** (depth 1) is new input for the place:
  grep for index-0 or cursor-on-newest assumptions on the rows. Report what you find; don't fix it (not your crate).
- **Fuzz targets** `line_log` and `follow_log`: the parsers are unchanged, so they aren't re-fuzzed. Say so.
- **Captures and walks:** none. The walks use the non-shallow Session fixture. Check by grepping `--depth` / `shallow`
  in `crates/graft/tests`.
- **Keymap, journal, undo:** none (read-only).
- **Design note H1 "End row":** its `<git's reason>` covers this. No edit.

## Size
About 30k. One context.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. No perf run in this ticket.

## Added by conductor, 2026-09-26
- [x] **Probe, report only** (see NOTES): in a real `--depth 2` clone, what does `blame` attribute to lines older than the boundary? Does the porcelain `boundary` flag reach `BlameCommit`, and would the screen name the boundary commit's author as the writer? Record the probe output in NOTES. Do not fix it; the conductor files it.

## NOTES (builder, 2026-09-26)
- No scout: ticket says none; site re-read directly (`trace/log.rs` `finish`).
- Design: `finish` takes a `boundaries` closure, called once after git exits (skipped on stop/cap kill). Boundary rows dropped also on git's failure path. Cap check runs after dropping, so `Capped` means more non-boundary rows really exist.
- Depth 5 left Tom as the boundary (main has 5 commits from HEAD to Tom); depth 6 used, Ana's commit is the boundary (fixture has an older root).
- **Blame probe** (git 2.54.0 only; 2.38 build not used): depth-2 clone, `blame` at HEAD. Lines 17–25, 27–30 name `0a27627` **Priya** (the boundary), `BlameCommit::boundary == true`; line 26 names HEAD. Porcelain for line 18 carries `boundary`. So the flag reaches `BlameCommit`; `grep -rn "\.boundary"` in graft/graft-ui: no hits, so a screen drawing the author would name Priya as writer (unverified on screen: no blame place consumer yet). Question for the conductor, not fixed.
- Touches: `TraceError` grep outside graft-git: none (no exhaustive match). `GitStops`/trace in graft, graft-ui: no hits — history place not built yet, so zero-row input has no consumer to check. Fuzz `line_log`/`follow_log`: parsers unchanged, not re-fuzzed. `--depth`/`shallow` in `crates/graft/tests`: none. Keymap/journal/undo: none.
- No compaction.
