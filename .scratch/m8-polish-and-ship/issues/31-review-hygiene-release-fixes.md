# 31 — fixes from review m8-hygiene-release

Status: ready
Build with: Sonnet. Every fix is spelled out in `docs/reviews/m8-hygiene-release.md`; nothing here decides a product question.
Source: `docs/reviews/m8-hygiene-release.md` Minors m1–m8 and Q1 (conductor, as found, no new decision; Q1 applies the standing CLAUDE.md codename rule).

## Crate set

graft (`main.rs`, `cli/flags.rs`, `cli/docs_tests.rs`, `update/tests/purity.rs`, the four stale-comment files, `update/tests/copy_and_compare.rs`), `README.md`, `docs/man/graft.1` (regenerated), `scripts/cargo-install-check.sh`, `.github/workflows/release.yml`, ticket files named in m1.

## Boxes (each names its test or check when ticked)

- [ ] m1 — tick `21b-5-additions-18e.md` 5i naming `843be1d8`; tick `21b-hygiene-round-2.md` 5+1 naming 21b-7b `0c865a9a`; set 21b-5's `[~] 1–4` to `[x]` naming 21b-7a `2bc836bc`; correct `30-linux-fast-tier-red.md:3` to "53 of 56". Check: `wc -l` quoted for each file the box claims.
- [ ] m2 — README flag list fed from `FLAGS` (or the sentence points at `--help`), pinned by a test like `readme_daily_keys_are_the_tables_keys`. Test: name it, seen red.
- [ ] m3 — `scripts/cargo-install-check.sh` prints `--version`; stale comment dropped. Re-run the script; quote its output, then tick ticket 24's open cargo-install box (`24-release-dist-and-matrix.md:57`) naming it.
- [ ] m5 — purity guard: `declares_tests` matches `mod tests` anywhere on a non-comment line and requires `#[cfg(test)]` on the same line or directly above; the guard also walks `empty_state/`. Doc at `purity.rs:7-12` says both. Test: seen red on a planted one-line `#[cfg(feature = "x")] mod tests;` fixture.
- [ ] m6 — `--generate-man` and `--help` write to locked stdout; `BrokenPipe` is a clean exit, with a comment saying why it's safe (rule 3). Test: name it (e.g. run the binary into a closed pipe, exit 0, no panic text).
- [ ] m7 — `cli/flags.rs:28` "print the usage and exit"; man page regenerated; its test green.
- [ ] m8 — remove `eprintln!("DBG …")` at `copy_and_compare.rs:166`; fix the four stale purity-path comments (`config/tests.rs:28`, `view/resolver/tests.rs:6`, `update/preview/rect.rs:34`, `update/resolver/tests.rs:667`). Check: `rg -n 'DBG|<old path>' crates` → no hits, pasted.
- [ ] Q1 — `.github/workflows/release.yml`: trigger becomes `workflow_dispatch` only, with a comment: "armed by the rename ticket (27/G-rename); the codename is never published (CLAUDE.md)". Check: `rg -n 'push:|pull_request' .github/workflows/release.yml` → no hits; `dist` regenerating would re-arm it, so name that in the comment.
- [ ] Gates workspace-wide green; MAP.md updated.

Out of scope here: m4 (rows re-routed by the conductor), Q2/Q3 (user's call / unverified, held for the stop).

Touches: `--help` output, man page + its test, README test, purity guard (every file under `update/` and `empty_state/`), release script, workflow.
