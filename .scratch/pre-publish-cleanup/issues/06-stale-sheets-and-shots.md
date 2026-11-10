# pre-publish: delete the stale contact sheets and the three-pane captures

Status: blocked (stop condition)
Blocked by: none
Build with: Sonnet (`builder-sonnet`, model sonnet). Why: deletions of named files. A drift script and grep are the check; no code.
Scout: no. Why: there's no code to scout. The files are named below, and `scripts/shot-drift.py` plus `git grep` are the brownfield check (ADR-0035's purpose: know what reads them before touching).

## Closes
- `ease/11b-three-pane-shots` (**broken**, stale): "Old `docs/shots/m6/fix-three-panes-*` captures show the removed three-pane layout."
- `ease/12b-m6-sheet-stale` (**broken**, stale): "m6 contact sheet stale after 12b's captures". Expiry: "pre-publish cleanup (delete, not rebuild — user 2026-10-19)".
- `ease/06i-m8-sheets` (**broken**, stale): "m8 contact sheets not reassembled after 06i captures". Expiry: "pre-publish cleanup (delete, not rebuild — user 2026-10-19)".
- `ease/14-m3-sheet-stale` (**unverified**): "m3-05-commit-refused captures re-shot by 14c (waiter now the commit screen's words, words.rs:191); `m3-contact-sheet.txt` not regenerated".

## Rulings
- User, 2026-10-19: "old milestone contact sheets (m6, m8…) and past-milestone docs are not rebuilt or edited; only test-read captures (redrawn, a gate) and shipping docs."
- Rows 12b and 06i: "delete, not rebuild".
- The m3 sheet is the same class (a stale old-milestone sheet), so it's deleted too. The conductor applies the user's rule; 00-index lists it.

## Files
- `docs/shots/m6/fix-three-panes-180x50.txt`, `-187x50.txt`, `-200x50.txt`
- the m6 contact sheet, the m8 contact sheet(s), and `docs/shots/m3-contact-sheet.txt`. Find the exact sheet names with `git ls-files docs/shots | grep -i -E 'sheet|contact'`, and list them in NOTES before deleting.

## Steps
1. For each file: run `scripts/shot-drift.py`, the authoritative check that no test writes it. Then `git grep -n <basename>` for readers: tests, scripts, TESTING.md, README, `docs/design/`, captions TSVs.
2. Delete the files with `git rm`.
3. A **shipping** doc (TESTING.md, README, `docs/design/`, `docs/man`) that links a deleted file: remove that one link line, and quote it in NOTES.
4. **Private** docs (`docs/log`, `docs/reviews`, `.scratch`, KNOWN_GAPS, ADRs) keep their references. They're history.
5. A captions TSV that only feeds a deleted sheet: leave it (not on the list) and note it.

## Stop conditions (report, don't work around)
- A test, or a shipped script under `scripts/` (export inventory §3 SHIP list), reads or writes a file on the list. Stop and report the line. Don't edit the test to skip it.
- `scripts/contact-sheet.sh` is called by a test or a gate that would regenerate a deleted sheet. Report it.

## Scope
In: the files above, and link lines in shipping docs.
Out: every other capture; the other stale captures in `docs/release/export-inventory.md` §9 (the licence/release step's); the m4 and m7 sheets (rows `M8/m4-07-future-rows`, `M8/30-captures-changed`, other expiries). If the drift check shows any other old sheet stale, list it in NOTES and don't delete it.

## Acceptance (tick each with its check, inline)
- [ ] Sheet names listed before deletion ← `git ls-files` output in NOTES
- [ ] No test or shipped script writes or reads any listed file ← `scripts/shot-drift.py` output + `git grep` output quoted
- [ ] Files deleted ← `git status --short` in NOTES
- [ ] No shipping doc references a deleted file ← `git grep -n` over shipping docs, empty, quoted
- [ ] Fast tier and walk tier green ← `cargo nextest run` count + `scripts/walks.sh` SUMMARY quoted

## Touches (check each, say how)
- Tests reading captures (the fixtures crate `capture.rs`, which references `docs/shots`).
- `scripts/contact-sheet.sh`, captions TSVs, TESTING.md links, README.
- `published_repo` (unaffected by deletion; confirm green).
- No Rust changed: `git diff --stat crates` empty.

## Read first
- This ticket.
- `docs/release/export-inventory.md` §4, §9, and its "Conductor rulings".
- The usage header of `scripts/shot-drift.py`.

## Gates
Workspace-wide: `cargo nextest run`, `cargo test --doc`, `scripts/walks.sh`. fmt and clippy are unaffected (no Rust), but run them anyway, since they're cheap.

## Size
~15k tokens.

## NOTES (builder, 2026-11-10) — stopped at a stop condition, nothing deleted
Sheet names (`git ls-files docs/shots | grep -i -E 'sheet|contact'`, the in-scope ones):
`docs/shots/m3-contact-sheet.txt`; `docs/shots/m6/m6-contact-sheet-{1..5}.txt`; `docs/shots/m8-contact-sheet-{1,2,3}.txt`. Plus `docs/shots/m6/fix-three-panes-{180x50,187x50,200x50}.txt`.

**Stop condition hit (both):** a shipped script regenerates a listed file.
- `scripts/m8-accept.sh:79`: `if scripts/contact-sheet.sh --captions-order m8 docs/shots > "$ACCEPT/sheet.txt" 2>&1; then` — `contact-sheet.sh` (SHIP, export-inventory §3 line 59) writes `docs/shots/m8-contact-sheet-<n>.txt` (`scripts/contact-sheet.sh:29-31`); `m8-accept.sh` is on the §3 SHIP list as "current acceptance walk". Running it re-creates the m8 sheets.
- Same pattern, not on the list but worth knowing: `scripts/ease-accept.sh:116`, `scripts/m7-accept.sh:102`.
Readers found, not blockers: `crates/mergewright/tests/it/m8_walk/contact_sheet.rs:11` reads only `m8-contact-sheet-captions.tsv` (line 88), not a sheet; `m5_walk/every_scenario.rs:302` names `m6-contact-sheet-captions.tsv` (not on list). Shipping-doc links that would need removing: `TESTING.md:2359` (m6 sheet), `TESTING.md:3320-3321`, `:3394` (m8 sheet).
No test writes `fix-three-panes-*` or `m3-`/`m6-contact-sheet*` (`git grep -l` over crates/scripts: only the above). shot-drift.py not run: it diffs changes since a rev, and nothing was changed.

Decision needed (conductor): delete m8 sheets anyway and drop step 3 from `m8-accept.sh` (or accept that it regenerates them), or take m8 off the list. The m3/m6/three-panes deletions have no blocker and can proceed as-is.
