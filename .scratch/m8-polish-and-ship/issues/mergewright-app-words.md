# Mergewright 3: the app calls itself Mergewright, in every word it draws or prints

Status: draft
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
- [ ] `rename_names::no_old_name_in_user_visible_strings`: every earlier generation's title and command, from the
      table; allow-list entries each with a reason (count in NOTES). `rename_names::the_word_test_reads_words_only`
      still passes and gains a `Tributary` case.
- [ ] Empty state reads `Mergewright · no repository here` at 80×24 and 120×36 (Unicode tier): the existing
      `docs/shots/m8-look-outside-repo-*.txt` and `docs/shots/m8/first-launch-empty-state-120x36.txt` regenerated;
      named in NOTES (rule 6, the user's eye; colour unverified, rule 5).
- [ ] Captures: `TRIB_UPDATE_SHOTS=1` run, then a plain run, green. `scripts/shot-drift.py --base <base> --out
      .scratch/m8-polish-and-ship/mwr-words-shot-drift.txt` summary quoted; every drift file listed with whether a
      clause was shed.
- [ ] `?` help and `⌃P` palette: a TestBackend test lists every action description and asserts none names an earlier
      generation: `palette::tests::no_action_names_an_old_product` (or the nearest existing listing test, named).
- [ ] `27-shots-left.txt` re-run: count in NOTES (input to `mergewright-recaptures`).

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
