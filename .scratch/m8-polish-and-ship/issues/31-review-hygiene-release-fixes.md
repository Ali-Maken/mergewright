# 31 — fixes from review m8-hygiene-release

Status: done (builder 2026-10-09)
Build with: Sonnet. Every fix is spelled out in `docs/reviews/m8-hygiene-release.md`; nothing here decides a product question.
Source: `docs/reviews/m8-hygiene-release.md` Minors m1–m8 and Q1 (conductor, as found, no new decision; Q1 applies the standing CLAUDE.md codename rule).

## Crate set

graft (`main.rs`, `cli/flags.rs`, `cli/docs_tests.rs`, `update/tests/purity.rs`, the four stale-comment files, `update/tests/copy_and_compare.rs`), `README.md`, `docs/man/graft.1` (regenerated), `scripts/cargo-install-check.sh`, `.github/workflows/release.yml`, ticket files named in m1.

## Boxes (each names its test or check when ticked)

- [x] m1 (checks: `git show 843be1d8:…empty_state.rs | wc -l` 373, `…/palette.rs` 149; `git show 2bc836bc:…update/tests.rs | wc -l` 128; 5+1 counts quoted from `21b-7-moves-remainder.md:231–244`, HEAD `wc -l` layout.rs 765 and view/rows.rs 893 still over by ruling 3, said in the tick; 30 now "53 of 56") — tick `21b-5-additions-18e.md` 5i naming `843be1d8`; tick `21b-hygiene-round-2.md` 5+1 naming 21b-7b `0c865a9a`; set 21b-5's `[~] 1–4` to `[x]` naming 21b-7a `2bc836bc`; correct `30-linux-fast-tier-red.md:3` to "53 of 56". Check: `wc -l` quoted for each file the box claims.
- [x] m2 (`cli::docs_tests::readme_command_line_is_the_usage_of_flags`, seen red then green; README `## Command line` holds `usage(FLAGS)` verbatim) — README flag list fed from `FLAGS` (or the sentence points at `--help`), pinned by a test like `readme_daily_keys_are_the_tables_keys`. Test: name it, seen red.
- [x] m3 (script re-run at `1837a68b`: `install: ok in 165s`, `graft --version: graft 0.1.0`; ticket 24 box ticked naming it) — `scripts/cargo-install-check.sh` prints `--version`; stale comment dropped. Re-run the script; quote its output, then tick ticket 24's open cargo-install box (`24-release-dist-and-matrix.md:57`) naming it.
- [x] m5 (`update::tests::purity::the_tests_module_match_sees_every_spelling`, incl. the `#[cfg(feature = "x")] mod tests;` row; seen red with the old start-of-line match planted back (`#[cfg(test)] mod tests;` → (0,[]) ≠ (1,[])), then green; `every_tests_module_is_cfg_test` now walks `update/`, `empty_state/` and `empty_state.rs`, green) — purity guard: `declares_tests` matches `mod tests` anywhere on a non-comment line and requires `#[cfg(test)]` on the same line or directly above; the guard also walks `empty_state/`. Doc at `purity.rs:7-12` says both. Test: seen red on a planted one-line `#[cfg(feature = "x")] mod tests;` fixture.
- [x] m6 (`tests/it/cli_docs.rs::printing_into_a_closed_pipe_exits_clean`: `--generate-man`, `--help`, `--version` into a pipe whose reader is closed; seen red, exit 101 `failed printing to stdout: Broken pipe`; green after `main.rs` `print_to_stdout`) — `--generate-man` and `--help` write to locked stdout; `BrokenPipe` is a clean exit, with a comment saying why it's safe (rule 3). Test: name it (e.g. run the binary into a closed pipe, exit 0, no panic text).
- [x] m7 (`cli::docs_tests::committed_man_page_is_the_generated_one` red, then man page regenerated with `graft --generate-man`, green; `cli::tests::help_prints_the_words_it_always_did` pin updated with a comment) — `cli/flags.rs:28` "print the usage and exit"; man page regenerated; its test green.
- [x] m8 (`rg -n 'DBG|update::tests::update_cannot_reach' crates` → no hits, exit 1; resolver/tests.rs comment reworded: the scan no longer skips by name only) — remove `eprintln!("DBG …")` at `copy_and_compare.rs:166`; fix the four stale purity-path comments (`config/tests.rs:28`, `view/resolver/tests.rs:6`, `update/preview/rect.rs:34`, `update/resolver/tests.rs:667`). Check: `rg -n 'DBG|<old path>' crates` → no hits, pasted.
- [x] Q1 (`rg -n 'push:|pull_request' .github/workflows/release.yml` → no hits, exit 1; comment names the re-arm by `dist`) — `.github/workflows/release.yml`: trigger becomes `workflow_dispatch` only, with a comment: "armed by the rename ticket (27/G-rename); the codename is never published (CLAUDE.md)". Check: `rg -n 'push:|pull_request' .github/workflows/release.yml` → no hits; `dist` regenerating would re-arm it, so name that in the comment.
- [x] Gates workspace-wide green (fmt --check, clippy --all-targets -D warnings clean; nextest 6216/6216; test --doc ok); MAP.md: no public item changed (`print_to_stdout` private in main.rs, tests private), so no edit.

## NOTES — 31 (builder, 2026-10-09)

- Scout: none; the review's citations, each re-read at HEAD before editing.
- Unverified knock-on: `dist` is not installed here. `dist plan`/`dist build` (scripts/release-local.sh) may refuse a hand-edited `release.yml` as out of date; if so, `allow-dirty = ["ci"]` in root `Cargo.toml` (outside this crate set) is the fix.
- 21b-5 5+1 ticked with `layout.rs` 765 and `view/rows.rs` 893 still over 400, per ruling 3, as stated in the tick.

Out of scope here: m4 (rows re-routed by the conductor), Q2/Q3 (user's call / unverified, held for the stop).

Touches: `--help` output, man page + its test, README test, purity guard (every file under `update/` and `empty_state/`), release script, workflow.
