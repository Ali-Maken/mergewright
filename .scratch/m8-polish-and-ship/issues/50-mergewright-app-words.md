# Mergewright 3: the app calls itself Mergewright, in every word it draws or prints

Status: done
Blocked by: 49
Build with: **Sonnet**. A word swap held by one grep test over string literals; layout drift goes to the user's eye.
Brownfield: 27a's NOTES ("Scope decisions to check") list the sites; conductor confirms no new scout.
Needs: `mergewright-package-and-command` merged (`graft::cli::COMMAND`; same test files, so sequential).
Alone: it touches string literals in every crate.

## Read first
- ADR-0057 Amendment (2026-10-11) §1, §2 (the three exceptions), §10, §12
- `issues/27a-rename-command-and-app-name.md` NOTES "Scope decisions to check"
- `crates/graft-ui/MAP.md` `APP_NAME` line, `## journal_screen` (legend only); `crates/graft/MAP.md` `## rename`,
  `## update::palette`, `## update::tour`
- `docs/design/02-keymap.md`: only to confirm no key changes (this ticket binds none)

## Decisions
- **`APP_NAME` from the table:** `pub const APP_NAME: &str = Generation::CURRENT.title();` (graft-ui already depends
  on graft-core; confirm in ARCHITECTURE.md "The crates", stop if not).
- **Prose stays literal** (§2): every `Tributary` in non-test string literals → `Mergewright`; every literal `trib:`
  stderr prefix in crates other than graft → `mwr:` (graft uses `COMMAND`). Error and refusal words (`… Mergewright
  cannot …`, `Mergewright keeps a backup first`), journal `commands Mergewright ran`, status screen title, tour README,
  palette and help action descriptions, `?` help text, keymap-derived hint words, the empty state, the first-launch
  line, the clone prompt.
- **The journal column and its legend stay allow-listed here**; `mergewright-git-messages-and-journal-column` owns
  them (§10). Messages written into git likewise.
- **Width.** `Mergewright` is 2 cells longer than `Tributary`. Where a test pins a width or a cut (`cut_at_word`,
  footers shedding clauses), the test follows the new words; no layout rule changes. A line that no longer fits at
  80 columns and sheds a clause it did not shed before is listed in NOTES with its capture, for the user's eye.
- **Grep test generalised (§2):** `rename_names::no_old_name_in_user_visible_strings` reads every generation in
  `Generation::CURRENT.older()` and fails on its `title()` (word-bounded, case-sensitive) or `command()` as a command
  word (`\btrib\b`, `\bgraft:` prefix) in non-test string literals of every crate. Its allow-list keeps 27a's code-name
  entries, the on-disk readers (`on_disk.rs`), and the two entries above, each with its reason and the ticket that
  removes it.

## Scope
In: string literals in `crates/*/src/**` and `crates/*/examples/**`; the test expectations that pin them; TestBackend
captures; the grep test; MAP lines that quote a changed string. Out: env names, git messages, the J column and legend,
docs and README, pty captures, PROMPT.md/CLAUDE.md.

## Signatures
```rust
// graft-ui graph_screen::header (path unchanged)
pub const APP_NAME: &str = graft_core::on_disk::Generation::CURRENT.title(); // "Mergewright"
```

## Acceptance (each box: the test named at tick time)
- [x] `rename_names::no_old_name_in_user_visible_strings`: every earlier generation's title and command, from the
      table; allow-list entries each with a reason (count in NOTES). `rename_names::the_word_test_reads_words_only`
      still passes and gains a `Tributary` case.
- [x] Empty state reads `Mergewright · no repository here` at 80×24 and 120×36 (Unicode tier): the existing
      `docs/shots/m8-look-outside-repo-*.txt` and `docs/shots/m8/first-launch-empty-state-120x36.txt` regenerated;
      named in NOTES (rule 6, the user's eye; colour unverified, rule 5).
- [x] Captures: `TRIB_UPDATE_SHOTS=1` run, then a plain run, green. `scripts/shot-drift.py --base <base> --out
      .scratch/m8-polish-and-ship/mwr-words-shot-drift.txt` summary quoted; every drift file listed with whether a
      clause was shed.
- [x] `?` help and `⌃P` palette: a TestBackend test lists every action description and asserts none names an earlier
      generation: `palette::tests::no_action_names_an_old_product` (or the nearest existing listing test, named).
- [x] `27-shots-left.txt` re-run: count in NOTES (input to `mergewright-recaptures`).

## Touches
- **Screens:** empty state, first launch, status screen, every refusal/error box naming the product, journal's
  expanded heading (`commands Mergewright ran`), tour card/README, help and palette rows. Each has a TestBackend test
  or capture; the list goes in NOTES.
- **Keymap:** text only (descriptions), no binding. `docs/design/02-keymap.md` unchanged.
- **Journal / undo:** none (words only; the `Migrate` row's words come from the table, ticket 1).
- **Crates:** graft-ui, graft, graft-ops (refusal words), any crate the grep finds.
- **Docs:** MAP lines quoting changed strings. TESTING.md captions naming the old word go to NOTES for
  `mergewright-docs-sweep`.
- **File-size ratchet** if a file grows.
- Each is checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide. Capture runs to files; summary lines only.

## Size
One context, ~40k tokens.

## NOTES (builder, 2026-10-12)

No new scout: 27a's NOTES "Scope decisions to check" were the scout; the grep test listed every site (27 literals, 22 src files).

**Ticks, with tests**
- Grep test: `rename_names::no_old_name_in_user_visible_strings` reads `Generation::CURRENT.older()` and fails on each
  one's `title()` or `command()` as a whole word (case-sensitive) in non-test literals of `crates/*/src` **and
  `crates/*/examples`**. `rename_names::the_word_test_reads_words_only` gains `Tributary` / `trib:` cases. Allow-list:
  23 entries, each with a reason (21 kept from 27a/49 incl. journal column + legend + messages written into git; 2 new
  for perf examples: `stash_only_cost.rs` "Graft Perf" identity, `snapshot_cost.rs` temp prefix + `.git/graft` scratch).
  Now case-sensitive: an all-caps `GRAFT` literal would no longer be caught (none exists; env names out of scope).
- Empty state: `outside_repo::outside_repo_captures`, `first_launch::first_launch_captures` regenerated
  `docs/shots/m8-look-outside-repo-{120x36,187x50,no-recent-80x24,open-prompt-80x24}.txt` and
  `docs/shots/m8/first-launch-empty-state-120x36.txt`: all read ` Mergewright · no repository here`. User's eye;
  colour unverified (rule 5).
- Captures: `TRIB_UPDATE_SHOTS=1` run (the multi-file journal walk needed 5 passes), then a plain full run green.
  `scripts/shot-drift.py`: `changed 41 · name-only 1 · drift-files 40 · png 0` →
  `.scratch/m8-polish-and-ship/mwr-words-shot-drift.txt`.
  **Clause shed at 80 cols (the user's eye):** the journal footnote "a branch deleted outside Mergewright and never
  checked out leaves no trace here" now ends `leaves no trace…` in `docs/shots/m3-journal-80x24.txt`,
  `docs/shots/m8-q1-journal-replaced-ui-80x24.txt`, `docs/shots/m8/21c-journal-partial-fetch-80x24.txt`.
  Every other drift is the word plus padding (boxes keep their width).
  **Question (no line found, rule 7):** `docs/shots/m8-tour-{80x24,187x50}.txt` header `◇ workar…` became
  `◇ workri…`: two letters seem to show through the header's gap, now from `Mergewright` instead of `Tributary`.
  Pre-existing; not chased.
- Palette/help: `update::palette::names_tests::no_action_names_an_old_product` (new file): every `?` row over a
  stopped merge plus every binding's label and search words; none names an older title or command.
- `27-shots-left.txt` re-run (`grep -rlwE "graft|Graft|trib|Tributary" docs/shots`, 49's rule): **450** (was 463).

**Touches**
- Screens: empty state, first launch, status screen title, refusal/error boxes (backup-first, stash, take-merge,
  fork refusal), journal expanded heading + footnote, credential box, tour README, theme-problems line. Each covered
  by the regenerated captures above or `status_screen` insta snapshots (updated).
- `APP_NAME` = `Generation::CURRENT.title()`; the status screen title now uses it (was its own literal). graft-ui
  already depends on graft-core (`crates/graft-ui/Cargo.toml:10`).
- Keymap: no binding changed; `docs/design/02-keymap.md` untouched.
- `docs/man/mwr.1` regenerated (pinned by `cli::docs_tests::committed_man_page_is_the_generated_one`).
- Ratchet: `crates/graft/src/update/palette/mod.rs` 404 → 406 (`over_400.txt`).
- Left on purpose: `graft-ops/src/rename.rs` + `rename/tests.rs`, `graft-ops/tests/it/rename/**`,
  `config/folder/tests.rs`, `view/journal/tests.rs` name the Tributary *generation*. `PRACTICE_PREFIX`
  (`tributary-practice-`, `graft-ops/src/tour.rs:62`) is a temp dir name the tour header path shows; the grep test
  does not read lowercase dir names. For a later ticket.
- For `mergewright-docs-sweep`: TESTING.md has 35 lines naming Tributary (title line 1; captions e.g. 1791, 3025).
- MAP: `crates/graft-ui/MAP.md` `APP_NAME` line.
