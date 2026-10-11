# Mergewright 8: M8's pty captures re-shot under the final name (27d unparked)

Status: draft. Supersedes 27d's remaining boxes (27d stays parked as the record of the classify pass).
Build with: _(blank: brownfield, ADR-0035)_. Planner expects **Sonnet** after the scout: runs existing writers; its
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
- [ ] `mwr-recapture-set.txt` committed; counts `m8 <a> · live-doc <b> · record <c> · code-name <d>` in NOTES;
      reviewer samples ≥10 `record` rows against git dates.
- [ ] Each `m8`/`live-doc` file re-shot by its writer: NOTES table `capture · writer command · result`.
- [ ] After: `rg -l` for every earlier generation's title, command and env prefix over the re-shot files returns only
      code-name matches (output to file, count quoted).
- [ ] `scripts/shot-drift.py --base <set start> --out .scratch/m8-polish-and-ship/mwr-recapture-drift.txt` summary
      quoted; drift files listed for the user's eye (rule 6), or stop rule 2.
- [ ] Writers that couldn't run: listed with the error's first line (stop rule 1), for a KNOWN_GAPS row.
- [ ] `find docs/shots -size +200k` empty (quoted). Colour unverified (rule 5).
- [ ] KNOWN_GAPS `M8/27d-shots-old-name`: the conductor's line for it in NOTES (closed, or narrowed to the writers
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
