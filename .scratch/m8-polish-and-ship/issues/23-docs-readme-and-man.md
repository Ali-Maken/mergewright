# G-docs — README, `--help` and a man page from one table

Status: done
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the man page generator and its staleness test
are code; the README is writing against the built app.
Blocked by: 09, 16, 18, 19
Needs: `keymap-table-and-translate` (`defaults()`, `Binding`), `theme-overrides`, `tour-practice-repo`,
`first-run-clone-and-first-launch` (all documented as built). Before `release-dist-and-matrix` (the archive ships
the man page).

## Read first
- `PROMPT.md` §4 "deferred" list and §9 bullet "nothing in the README or the first-run tour implies it exists"
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows "Man page", "homebrew-core; clap_mangen; completions";
  Journeys J16, J17; Interfaces `graft::cli`
- `docs/research/install-docs.md` §1 (ripgrep row), §6 (journeys)
- `crates/graft/MAP.md` `## cli` (`USAGE`, `parse`, askpass mode), `## config`

## Decisions (planner)
- One CLI table, `FLAGS`, feeds `--help`, the man page and the README's flag list. `USAGE` is generated from it.
- `--generate-man` (hidden from `--help`) prints roff to stdout: NAME, SYNOPSIS, DESCRIPTION, OPTIONS (from `FLAGS`),
  KEYS (one subsection per `ContextId`, from `defaults()` — default chords, label, config id), CONFIGURATION
  (`[ui]`, `[graph]`, `[diff]`, `[radar]`, `[keys]` incl. `suspend_on_ctrl_z` and `[keys.<context>]`, `[theme.<kind>]`),
  ENVIRONMENT, FILES (config dir, recents, launches, per-repo dirs), SEE ALSO `git(1)`. Committed copy `docs/man/
  graft.1` (G-rename renames it); a test fails when the committed copy differs from the generator's output.
- No shell completions (R in Prior art). No clap (keep the hand-written parser).
- README: what it is (one paragraph + one capture), install (brew tap line, curl installer line, `cargo install
  --locked --path crates/graft` — all marked "after the first release" until publishing is allowed, ADR-0057), first
  run (`?`, `⌃P`, tour), daily keys (top 12 from the table), config (each section with one example), what it doesn't do
  (PROMPT §4's deferred list plus pixel-tier image diffs, Map view mouse and remote rows, undo-backup pruning — past
  v1, ADR-0057), terminals tested. No claim of a feature not built.

## Scope
In: `FLAGS`, generated `USAGE`, `--generate-man`, `docs/man/graft.1`, staleness test, README rewrite, TESTING.md row
J17 (`man ./docs/man/graft.1` reads). Out: website; completions; translations; the release archive (G-release).

## Signatures
```rust
// graft::cli
pub struct Flag { pub long: &'static str, pub short: Option<char>, pub value: Option<&'static str>,
    pub help: &'static str, pub hidden: bool }
pub const FLAGS: &[Flag];
pub fn usage(flags: &[Flag]) -> String;
pub fn man_page(flags: &[Flag], keys: &[Binding], config: &[ConfigKeyDoc]) -> String;
pub struct ConfigKeyDoc { pub section: &'static str, pub key: &'static str, pub values: &'static str,
    pub default: &'static str, pub help: &'static str }
pub const CONFIG_KEYS: &[ConfigKeyDoc];        // test: every key the loader accepts has one row, and the reverse
```

## Acceptance
- [x] `--help` output from `FLAGS` (test compares to `usage(FLAGS)`); every parsed flag has a row and the reverse (test).
      Tests: `cli::docs_tests::help_is_usage_of_flags`, `it::cli_docs::help_prints_usage_of_flags` (the real binary's
      stdout, `-h` and `--help`), `cli::tests::help_prints_the_words_it_always_did` (old `USAGE` text, unchanged),
      `cli::tests::refusals_keep_their_words_and_carry_the_usage`; both ways:
      `cli::docs_tests::every_flag_row_parses_in_each_spelling_and_every_action_has_a_row` (parse acts only on a row
      it finds in `FLAGS`, so an unrowed flag is `UnknownOption` by construction).
- [x] `man_page` golden = `docs/man/graft.1` (test); `mandoc -T lint docs/man/graft.1` (or `groff -man -ww`) output
      quoted, 0 errors.
      Tests: `cli::docs_tests::committed_man_page_is_the_generated_one` (seen red with a stray line appended, green
      after `cargo run -q -p graft -- --generate-man > docs/man/graft.1`), `it::cli_docs::generate_man_prints_the_man_page_and_nothing_else`,
      `cli::docs_tests::man_page_is_ascii_and_names_every_action_flag_and_key`,
      `cli::docs_tests::every_chord_the_man_page_writes_reads_back_as_itself`. mandoc 2026-10-09, whole output:
      `mandoc: docs/man/graft.1:1:13: WARNING: missing date, using "": TH` — 0 errors, 1 warning, kept on purpose
      (a date makes the golden stale daily or false). No groff on this machine.
- [x] `CONFIG_KEYS` ↔ loader (test both ways). `cli::docs_tests::config_keys_and_the_loader_name_the_same_keys`
      (set equality with `config::load::SCHEMA` + `keys.suspend_on_ctrl_z`; patterns `keys.<context>`,
      `theme.<kind>`), `cli::docs_tests::every_documented_default_loads_cleanly_as_the_real_default` (each literal
      default loads with no warning and equals `Config::default()`).
- [x] README: every key or config named in it exists (test greps README's backticked keys against `defaults()` and
      `CONFIG_KEYS`); no deferred feature named as present (checklist quoted at tick).
      Tests: `cli::docs_tests::readme_daily_keys_are_the_tables_keys` (17 rows: action exists, key is one of its
      default labels), `readme_config_names_only_real_keys`, `readme_example_config_loads_without_a_word` (README's
      TOML loads with no loader or keymap warning), `readme_backticked_names_exist`. Deferred checklist — README
      "What it does not do" lists, and nothing else in it claims: visual bisect; worktree switcher; LFS beyond git's
      filters; sparse-checkout; repos in tabs; submodule update/sync; pixel-tier image diffs; Map view mouse and
      remote rows; undo-backup pruning (ADR-0057); Windows. Checked by reading, not a test.
- [x] J17 `man` (person step): TESTING.md row. `TESTING.md` "## M8: read the manual (J17, m8-polish-and-ship/23)";
      his read is the unticked box there.

## NOTES (builder, 2026-10-09)
- Scout used: `.scratch/m8-polish-and-ship/scouts/23-docs.md`.
- Env vars in the man page's ENVIRONMENT: `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `HOME`, `NO_COLOR`, `TERM`/
  `COLORTERM`/`TERM_PROGRAM` (+ the detection set), `GRAFT_PERF_TRACE` (user-facing: TESTING.md's trace run asks him
  to set it), `GRAFT_ASKPASS_SOCKET` (named as set by graft, not by hand). Left out as test-only: `GRAFT_NOW` (19e
  R1: pins the drawn clock for the pty harness), `GRAFT_PRACTICE_EPOCH` (dates the practice repo for captures),
  and the scout's test-only list.
- `Invocation::Usage` now carries a `String` (was `&'static str`); main.rs the only caller.
- Man page names the preview place "Preview box": `preview::no_user_facing_string_says_ghost` scans `graft/src`.
- Config value words avoid a leading "a " (`no_literal_keys` reads it as the key `a`).
- Flaky, not mine: `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` failed once in the full run
  (749 s, loaded machine), passed twice alone; 19e owns that walk's settle.
- README capture is an excerpt (110 cols) of `docs/shots/m8/fork-follows-banner-187x44.txt`; it shows a fixture path.
- G-rename must rename: `docs/man/graft.1`, `.TH GRAFT`, man/README `graft` names, `$XDG_*/graft` paths in
  `cli/man.rs` text, the test's `include_str!` path.

## Touches
- `cli::parse` and askpass mode (unchanged behaviour); `USAGE` callers; config loader; README captures (one, from
  `docs/shots/`); G-rename list (`docs/man/graft.1`, README names); MAP.md graft.

## Scout questions
- Every flag `parse` accepts today, and any env var the binary reads (file:line) — both go in the man page.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.
