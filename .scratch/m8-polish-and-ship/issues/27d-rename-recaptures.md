# G-rename 2d — pty captures that print the name, re-shot

Status: ready (split from 27, spec-only pass 2026-10-10)
Blocked by: 27a, 27b
Build with: **Sonnet** — runs existing capture scripts; judgement bounded by the drift script and two stop rules.
Needs: 27a (`trib` binary, `scripts/shot-drift.py`, `27-shots-left.txt`), 27b (`TRIB_*` in the pty scripts, list
re-run). Can run beside 27c.

## Read first
- 27a and 27b `## Decisions`; `.scratch/m8-polish-and-ship/27-shots-left.txt` (paths only)
- `crates/graft-fixtures/MAP.md` §capture (update var, redaction)
- Per writer script used: its header/usage only, not the whole file

## Decisions (spec-only pass; scout's "how many regenerate" resolved)
- **TestBackend captures are already done** (27a/27b regenerated them under the gates). This ticket is the rest:
  the files in `27-shots-left.txt`.
- **Re-shoot only where the app prints the name** — the app's name, the command, a `TRIB_`/`GRAFT_` env name, an
  on-disk dir or ref name the app draws. A file whose every match is a code name (temp-dir prefix such as
  `graft-capture`, crate path, package) is not re-shot: crate-name class. Classify with one script pass writing
  `.scratch/m8-polish-and-ship/27d-classified.txt` (`path · reshoot | code-name · the matching line`).
- **Every re-shoot comes from its writer**, the script that made it (`.scratch/*/pty-walk*.py`, `scripts/*-accept.sh`,
  `--capture` walks). A capture is never edited by hand: that would be a fabricated capture.
- **Stop rules.** (1) A re-shoot candidate with no writer, or whose writer no longer runs → don't fix the writer;
  list it and continue with the rest. (2) After re-shooting, `scripts/shot-drift.py` reports more than **10** drift
  files → stop before committing, report the drift file; whether old milestones' frames are re-baselined is the
  conductor's call. Why: a writer re-run can surface drift the rename didn't cause (M6 already marks stale frames).
- Captures stay ≤ 200 KB; raw terminal streams never committed. Unicode tier is the one that matters (his daily
  view); a writer that also shoots ASCII/pixel tiers runs as it does today.

## Scope
In: classifying `27-shots-left.txt`; re-running writers for `reshoot` files; committing re-shot captures; drift
report. Out: editing writers or walks (stop rule 1); TestBackend captures; contact sheet (M8 acceptance assembles it);
TESTING.md captions (27c owns TESTING.md — any caption naming the old app word goes to NOTES for 27c/conductor).

## Signatures
None.

## Acceptance (each box: the evidence named at tick time)
- [ ] `27d-classified.txt` committed; counts `reshoot <n> · code-name <m>` in NOTES; reviewer samples ≥10
      `code-name` rows.
- [ ] Each `reshoot` file re-shot by its writer: NOTES table `capture · writer command · result`.
- [ ] After: `rg -l -i 'graft|GRAFT_'` over the re-shot files returns only code-name matches (output to file, count
      quoted); 0 files print the old app name, command or env name.
- [ ] `scripts/shot-drift.py --base <27b's commit> --out .scratch/m8-polish-and-ship/27d-shot-drift.txt` summary
      quoted; drift files listed in NOTES for the user's eye (rule 6), or stop rule 2.
- [ ] Writers that couldn't run: listed with the error's first line (stop rule 1), for a KNOWN_GAPS row.
- [ ] Every committed capture ≤ 200 KB (`find docs/shots -size +200k` empty, quoted). Colour unverified (rule 5).

## Touches
`docs/shots/**` (re-shot files only); the writers (run, not edited); `.scratch/m8-polish-and-ship/27d-*.txt`;
TESTING.md captions (listed, not edited); M8 contact sheet (acceptance ticket reads the new files). Each checked by
the files above, named in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide (some writers are walk tests). Context guard: every writer's output to a file under the scratchpad
or `.scratch/`; read summary lines only, never a capture whole.

## Size
One context, ~40k tokens if the reshoot set is a few dozen files; over ~60 files → split by writer, report first.

## NOTES (builder, 2026-10-10) — partial: stopped at the size rule, report first

- No scout dispatched: the spec's scout (`scouts/27-rename-sweep.md`) plus one classify pass sufficed to size it.
- **Box 1 done:** `27d-classified.txt` written by `scratchpad/classify.py` (regex: line still says graft after
  stripping crate paths, `graft-*`/`/tmp/graft*` temp prefixes, `Graft Fixture` author, `*.git` URLs, `-p graft`,
  `graft_sandbox`). Counts: **reshoot 244 · code-name 199** (443 total). Ambiguous lines default to reshoot.
  Main reshoot classes: header `graft <path>` (APP_NAME), journal `outside Graft` / `● Graft` rows, prose like
  `Graft keeps a backup first`, `target/*/graft`, `~/.cache/graft`, stash message `graft: your edits`.
- **Size stop (ticket §Size: >60 → split by writer, report first).** 244 ≫ 60. Writer map:
  `.scratch/m8-polish-and-ship/27d-writers.txt` (filename grep over scripts/, crates/, .scratch/ .py/.sh/.rs).
  **209 of 244 have no writer found by filename** — likely built from format strings (`m6/{name}-{w}x{h}`) or
  hand-assembled contact sheets/`.md` captions; mapping them needs a scout per directory
  (m6 76, m5 56, m2 52, m7 49, 35-* 48, m8 34, m3 28, m4 21 across both classes).
- Also: several reshoot files are contact sheets (`m3-contact-sheet.txt`, `m5-contact-sheet-2.txt`,
  `m7-contact-sheet-*.txt`) — assembled, not shot; regenerate from parts after the parts are re-shot.
- Some candidates are written by Rust tests (`m7_walk/journal.rs`, `history_place.rs`, …) — 27a said TestBackend
  was done; these still print `Graft` in journal rows, so either the app string in the journal (`outside Graft`)
  was not renamed in 27a, or the capture wasn't updated. **Could not confirm which** — not checked.
- Remaining boxes 2–6 open. Next step: conductor splits by directory; a scout maps each directory's writer; then
  re-shoot per writer. Nothing re-shot, no capture touched; gates not run (no code changed).
