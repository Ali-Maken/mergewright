# G-docs — README, `--help` and a man page from one table

Status: ready
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
- [ ] `--help` output from `FLAGS` (test compares to `usage(FLAGS)`); every parsed flag has a row and the reverse (test).
- [ ] `man_page` golden = `docs/man/graft.1` (test); `mandoc -T lint docs/man/graft.1` (or `groff -man -ww`) output
      quoted, 0 errors.
- [ ] `CONFIG_KEYS` ↔ loader (test both ways).
- [ ] README: every key or config named in it exists (test greps README's backticked keys against `defaults()` and
      `CONFIG_KEYS`); no deferred feature named as present (checklist quoted at tick).
- [ ] J17 `man` (person step): TESTING.md row.

## Touches
- `cli::parse` and askpass mode (unchanged behaviour); `USAGE` callers; config loader; README captures (one, from
  `docs/shots/`); G-rename list (`docs/man/graft.1`, README names); MAP.md graft.

## Scout questions
- Every flag `parse` accepts today, and any env var the binary reads (file:line) — both go in the man page.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.
