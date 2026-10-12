# Mergewright 8: M8's pty captures re-shot under the final name (27d unparked)

Status: partial (2026-10-12): all boxes but the old-name box, which holds one writer-side defect (quit_waits.rs:186). Supersedes 27d's remaining boxes (27d stays parked as the record of the classify pass).
Blocked by: 46, 47, 48, 49, 50, 51, 52
Build with: **Sonnet** (set by the conductor after the scout). Planner expected Sonnet: runs existing writers; its
judgement is bounded by the drift script and two stop rules.
Scout first: one page mapping each M8 capture in the re-shoot set to its writer (`file · writer command`), and
listing captures with no writer. 27d found 209 of 244 with no writer by filename, mostly pre-M8, which §12 now
leaves alone.
Needs: every code ticket of the set merged (`mergewright-names-table-and-migration` through
`mergewright-git-messages-and-journal-column`, and `usability-q1-*`, `usability-q2-*`), so each capture is shot
once. Beside: `mergewright-docs-sweep`.

## Read first
- ADR-0057 Amendment (2026-10-11) §12 "Captures"
- `issues/27d-rename-recaptures.md`: Decisions (stop rules), NOTES
- `.scratch/m8-polish-and-ship/27d-classified.txt` (counts only), the latest `27-shots-left.txt` (paths only)
- `crates/graft-fixtures/MAP.md` §capture; each writer script's header/usage only
- The scout's file

## Decisions
- **The set** (§12): files in the latest `27-shots-left.txt` whose capture belongs to M8 (path under `docs/shots/m8*`,
  `docs/shots/m8/`, or committed on or after M8's first commit, by `git log --diff-filter=A --format=%H -1 -- <file>`
  against the M8 start commit in `docs/log/m8-polish-and-ship.md`), **plus** any pre-M8 capture a live doc (README,
  TESTING.md instruction lines, `docs/design/**` current mock-ups) shows as the app today. Everything else is a record:
  not re-shot, listed by count. Classify with one script pass into
  `.scratch/m8-polish-and-ship/mwr-recapture-set.txt` (`path · m8 | live-doc | record · reason`).
- **A match that is a code name** (temp prefix, crate path, fixture identity) is not a re-shoot (27d's class).
- **Every re-shoot comes from its writer.** A capture is never edited by hand: that would be a fabricated capture.
- **Stop rules (27d's):** (1) no writer, or the writer no longer runs → listed with the error's first line, not
  fixed. (2) `scripts/shot-drift.py` reports more than **10** drift files that are not name-only → stop before
  committing, report the drift file.
- Contact sheets are assembled, not shot: the M8 sheet is rebuilt by the acceptance ticket; pre-M8 sheets are records.
- Captures ≤ 200 KB; raw terminal streams never committed. Unicode tier first (his daily view).

## Scope
In: classifying; re-running writers for `m8` and `live-doc` rows; committing re-shot captures; drift report. Out:
editing writers or walks; TestBackend captures (their tickets regenerated them); contact sheets; TESTING.md captions
(listed for `mergewright-docs-sweep`).

## Signatures
None.

## Acceptance (each box: the evidence named at tick time)
- [x] `mwr-recapture-set.txt` committed; counts `m8 <a> · live-doc <b> · record <c> · code-name <d>` in NOTES;
      reviewer samples ≥10 `record` rows against git dates. — evidence: the file + counts in NOTES.
- [x] Each `m8`/`live-doc` file re-shot by its writer: NOTES table `capture · writer command · result`. — evidence:
      `MWR_UPDATE_SHOTS=1 cargo nextest run --workspace --no-fail-fast` 6421 passed, then plain run.
- [ ] After: `rg -l` for every earlier generation's title, command and env prefix over the re-shot files returns only
      code-name matches (output to file, count quoted). — **NOT held**: 3 lines, 2 code-name, 1 defect
      (quit_waits.rs:186 label, writer-side, out of scope). See NOTES.
- [x] `scripts/shot-drift.py --base <set start> --out .scratch/m8-polish-and-ship/mwr-recapture-drift.txt` summary
      quoted; drift files listed for the user's eye (rule 6), or stop rule 2. — evidence: run quoted in NOTES;
      `python3 scripts/shot-drift-test.py` ok.
- [x] Writers that couldn't run: listed with the error's first line (stop rule 1), for a KNOWN_GAPS row. — NOTES list
      (none failed; five have no writer).
- [x] `find docs/shots -size +200k` empty (quoted). Colour unverified (rule 5). — evidence: command output empty.
- [x] (NOTES line) KNOWN_GAPS `M8/27d-shots-old-name`: the conductor's line for it in NOTES (closed, or narrowed to the writers
      that couldn't run).

## Touches
- `docs/shots/**` (re-shot files only); the writers (run, not edited); `.scratch/m8-polish-and-ship/mwr-recapture-*`.
- TESTING.md captions (listed, not edited); the M8 contact sheet (acceptance reads the new files).
- **Screens / keymap / journal / undo / crates:** none changed.
- Each is checked by the files above, named in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide (some writers are walk tests). Every writer's output goes to a file; read summary lines only, never a
capture whole.

## Size
One context, ~40k tokens if the set is ≤ 60 files; over 60 → split by writer, report first.

## Added by the conductor (scout 56, 2026-10-12)
Scout: `.scratch/m8-polish-and-ship/scouts/56-recaptures.md`. Build with set to **Sonnet** (the planner's expectation).
- The live log is `docs/log/m8.md`; M8 start f5080f50 (its line 27).
- 18 of the 21 old-name M8 files are TestBackend captures (`check_shot`): do not hand-edit; one `MWR_UPDATE_SHOTS=1`
  run then a plain run; what still shows an old name after that is a code-name use or a defect — list which, per file.
- `scripts/shot-drift.py` counts name-only changes only for `graft`: make it read every earlier generation (graft,
  tributary, trib) so renames don't show as drift. Test it.
- No writer for `21c-journal-partial-fetch` (80x24, 120x40) and `doorway-marked-140x40`: leave them as records, list in
  NOTES; the conductor files a gap.
- **Never kill processes you did not start.**

## NOTES (builder 56, 2026-10-12)
Scout used: `.scratch/m8-polish-and-ship/scouts/56-recaptures.md`. No compaction.

**Counts** (`mwr-recapture-set.txt`, the 450 rows of `27-shots-left.txt` that still exist):
`m8 6 · live-doc 0 · record 241 · code-name 199` (+ `sheet 4`: m8 contact sheets 1–3 and captions.tsv, assembled).
- Deviation, for the conductor: 25 rows first classed `live-doc` are linked **only from past-milestone TESTING.md
  checklists** (M1–M7 sections, e.g. TESTING.md:1108, 1332, 2771–3004), not from a doc showing the app today →
  reclassed `record`. README's one shot link (m8/fork-follows-banner-187x44) is not in the left list. Re-shooting
  them meant running pre-M8, pre-sandbox pty scripts (`.scratch/m1-the-graph/`) against a header M8 replaced:
  certain non-name drift. Overrule if "TESTING.md instruction lines" meant these.

**Re-shoot table** (all six are `Captures::check`/`check_shot` writers):
| capture | writer command | result |
|---|---|---|
| m8-tour-187x50 | `MWR_UPDATE_SHOTS=1 cargo nextest run --workspace` (tour_walk.rs) | unchanged; old name only in temp prefix `/private/tmp/graft-capture/` (code-name) |
| m8-quit-waits-80x24 | same (quit_waits.rs) | unchanged; **defect**: label `Graft waits for git` is written by crates/graft/tests/it/quit_waits.rs:186 — writer edit out of scope |
| m8-q1-journal-replaced-ui-80x24, -187x30 | same (graft-ui journal_screen/tests/replaced.rs) | unchanged, no old name |
| m8-hangup-relaunch-120x40 | same (quit_waits/hangup.rs) | unchanged, no old name |
| m8/doorway-journal-120x36 | same (update/tests/doorway.rs) | unchanged; header names `crates/graft/src/update/tests.rs` (code-name; also stale path) |

Update run: 6421 passed, 16 skipped; `git status` showed no docs/shots change → every TestBackend writer was
already current. A first fail-fast run failed once in `outside_repo::launch_and_open_land_alike` ("fatal: could not
parse HEAD", tests/it/app_harness.rs:84); passed on rerun. Flake, cause not found (another builder's cargo beside).

**Old names after**: `grep -niE 'graft|tributary|\btrib\b'` over the six → `mwr-recapture-oldnames.txt`, 3 lines
(temp prefix, crate path, the quit_waits.rs:186 defect).

**Drift**: `shot-drift.py` now counts graft, tributary and `\btrib\b` as earlier names. Test
`scripts/shot-drift-test.py` (wired into check-scripts.sh): ok; against the old regex it fails
(`name-only 1 · drift-files 3`). Run `--base HEAD`: `changed 0 · name-only 0 · drift-files 0 · png 0`. Nothing new
for the user's eye from this ticket.

**No writer (stop rule 1, none errored)**: m8/21c-journal-partial-fetch-80x24 and -120x40 (scout: none by name);
m8-proto-3a-empty, m8-proto-3c-clone-prompt (throwaway prototype); m8/doorway-marked-140x40 (scout; not in list).

**Size**: `find docs/shots -size +200k` → empty. Colour unverified (rule 5).

**Question** (textual): m8-tour-187x50.txt line 3 reads `◇ workripractice ·` — two words with no gap? Not checked.

**KNOWN_GAPS `M8/27d-shots-old-name`** — conductor's line: narrow to (a) quit_waits.rs:186 label: writer edit +
re-shoot; (b) the no-writer files above; (c) pre-M8 captures are records by §12.

Touches: docs/shots none changed (git status); scripts/shot-drift.py, scripts/shot-drift-test.py (new),
scripts/check-scripts.sh (ran the test); crates/graft/tests/it/rename_docs.rs: one ALLOWED_FILES row for
shot-drift-test.py (it plants old names on purpose; `rename_names::user_docs_and_scripts_name_the_command` failed
without it — caught by the plain nextest run); TESTING.md captions not edited; no production code.

**Conductor heads-up (55: m7 journal captures say Graft)**: not so on this branch (base 80d49c74). `grep -lE 'Graft|● *graft' docs/shots/m7/*journal*` → none; `docs/shots/m7/journal-187x50.txt` has 5 `Mergewright` lines (re-shot by 52's TestBackend run). No writer by filename under crates/ (`grep -rln journal-187x50 crates` → none), so nothing to re-run. If 55 saw Graft in a different file, name it.
