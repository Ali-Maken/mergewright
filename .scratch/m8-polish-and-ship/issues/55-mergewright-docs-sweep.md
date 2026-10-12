# Mergewright 7: README, TESTING.md, CHANGELOG, help docs and design-doc UI quotes say Mergewright and `mwr`

Status: ready
Blocked by: 52, 53
Build with: **Sonnet**. Line triage by a written rule, held by a grep test and a listed remainder.
Needs: `mergewright-git-messages-and-journal-column` and `mergewright-release-and-formula` merged (final words in the
app, README Install owned there). Beside: `mergewright-recaptures` (it commits only `docs/shots/` captures; this
ticket runs cargo only for its test and the gates).

## Read first
- ADR-0057 Amendment (2026-10-11) §12 (where each name goes)
- `issues/27c-rename-user-docs-and-scripts.md`: Decisions (the per-line rule), NOTES
- `.scratch/m8-polish-and-ship/27c-testing-kept.txt` (classes only)
- README.md, TESTING.md, CHANGELOG.md, `docs/design/**`, DESIGN.md: only the lines
  `rg -n '\b(Tributary|trib|TRIB_)\b|\bGraft\b'` returns, ±3 lines
- `docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md` (this set's journeys)

## Decisions
- **27c's rule, per line:** change it if a person following it today would type it or read it as the app's name;
  keep it if it records the past. Package, crate and dir names stay (`crates/graft`, `graft-ui`); `-p mergewright`
  is already done (ticket 2).
- **README:** product name, command, the "outside a repo" journey (`mwr ⏎`), man page name, config path
  (`~/.config/mergewright/`), on-disk names (`refs/mergewright/`, `.git/mergewright/`). Install section is not
  touched (ticket 6 owns it).
- **CHANGELOG.md:** a new top entry `Unreleased: renamed to Mergewright (command mwr); on-disk names move on first
  launch, nothing lost` (ADR-0057). Older entries are records.
- **TESTING.md:** instructions and checklists; records stay. J14/J16/J17 rows (`mwr ⏎`, `brew install
  <owner>/tap/mergewright`, `man mwr`). Captions that name the old app word (handed over by tickets 3 and 5).
  Remainder file as 27c's.
- **Design docs (§12):** prose keeps the codename. Only UI strings the app draws, quoted in mock-ups or in quotes,
  follow the app (J rows, empty state, refusal words). One file lists every changed quote:
  `.scratch/m8-polish-and-ship/mwr-design-quotes.txt` (`file:line · old → new`).
- **The M8 design note's fence comments** `open-recent-outside … · trib ⏎` and `open-second-recent … · trib j ⏎` →
  `mwr` (comment text only; counts unchanged; the acceptance ticket adds new rows). The rest of that note is a record.
- **Scripts' printed words** (echo, usage, comments naming the app) → Mergewright/`mwr`, except `release-local.sh`
  and `formula.py` (ticket 6).
- **Not edited:** CLAUDE.md, PROMPT.md, `.claude/**` (`mergewright-protected-lines`); records (ADRs, logs, reviews,
  field notes, KNOWN_GAPS, closed gaps, past tickets, design notes other than the fence comments).

## Scope
In: README.md (not Install), CHANGELOG.md (new entry), TESTING.md (non-record lines), `scripts/**` words,
`docs/design/**` and DESIGN.md UI quotes, the fence comments, CAPABILITIES.md / `docs/capabilities/*` if generated
from live words (regenerate, don't hand-edit). Out: as above; captures.

## Signatures
None (docs and scripts). Test: `crates/graft/tests/it/rename_docs.rs`.

## Acceptance (each box: the test named at tick time)
- [ ] `rename_names::user_docs_and_scripts_name_the_command` extended from the table: README.md, CHANGELOG.md's top
      entry and `scripts/**` have no earlier generation's title or command outside the allow-list (crate names, `-p`
      already handled, `rename_docs`' reasons). Finder: `rename_docs::the_word_finder_reads_words_only` gains a
      `Tributary`/`trib` case.
- [ ] TESTING.md remainder: `.scratch/m8-polish-and-ship/mwr-testing-kept.txt`, one line per kept match
      `line · class`; counts in NOTES; no kept line is an instruction (reviewer samples ≥10).
- [ ] `mwr-design-quotes.txt` committed; count in NOTES. Each changed quote matches a string the app draws today
      (grep of the source string or a capture line, named per row).
- [ ] `bash -n` / `ast.parse` over touched scripts: summary quoted.
- [ ] `cli::docs_tests::readme_command_line_is_the_usage_of_flags` still green (README usage block).

## Touches
- **Callers:** scripts that print words other scripts parse (grep `m8-accept.sh`, `m7-accept.sh` for the words they
  match on; any parsed word kept or both sides changed together).
- **Screens / keymap / journal / undo:** none (docs). `docs/design/02-keymap.md` quote changes only; no key changed.
- **Captures:** none committed; TESTING.md captions follow the captures `mergewright-recaptures` shoots (captions for
  re-shot files are written after that ticket lands, or listed for it).
- **Docs:** README, CHANGELOG, TESTING.md, DESIGN.md/`docs/design/**`, the fence comments, CAPABILITIES.
- **Crates:** test files only.
- Each is checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide.

## Size
One context, ~40k tokens.

## Added by the conductor (review 51, 2026-10-12)
`docs/reviews/m8-mergewright-51.md`:
- **Major 1:** the git-git cache moved to `mergewright/`, readers did not: `graft gallery.rs:677` defaults to
  `graft/git-git.git` (its doc says it follows the script), `m1-checkpoint.sh:78`, `TESTING.md:595`. Take the path from
  the table where code reads it; test that the gallery default equals the script's path (red first).
- Minors: a helper in `tour/tests.rs` (ratchet +47 back down); `tour.rs:64` comment claims older folders clean up but
  `build.rs:71` always uses CURRENT — say what is true; when both caches exist, say so once; `tour_walk.rs` prefix
  literals from the table.
