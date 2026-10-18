# 01 — codename rename: `graft` → `mergewright` in code, crates, docs

Status: done with deviations (builder, 2026-10-18) — see NOTES. Was: ready (conductor, 2026-10-18; user: "first codename rename", quota-saving — one helper, mostly scripted)
Blocked by: ease-pass 11 and 12 merged (nothing else building — this touches every file; run **alone**)
Build with: Opus low (`builder-sonnet`) — mechanical rename; the full gates judge it.

## Why
"Graft" is another Git GUI's name (CLAUDE.md). The product is Mergewright, command `mwr` (ADR-0057). What users see and what is stored on disk is **already** renamed: binary `mwr`, package `mergewright`, refs/dirs/env/trailers through `graft-core/src/on_disk.rs` `Generation::CURRENT = Mergewright`, with a one-time move from older names. What is left is internal.

## Scope
In:
1. Crates: `crates/graft*` → `crates/mergewright*` (`graft` → `mergewright-app` or keep the existing package name `mergewright` and pick a non-clashing dir — decide from `crates/graft/Cargo.toml`, which already says `name = "mergewright"`); every `graft-*` package → `mergewright-*`; every `graft_*::` path → `mergewright_*::`; workspace `Cargo.toml`, `Cargo.lock` (regenerate), `CARGO_TARGET_DIR` hints, scripts, CI files.
2. Comments, MAP.md, ARCHITECTURE.md, README, man page, TESTING.md, design docs: "Graft"/"graft" as the app's name → "Mergewright" / crate names.
3. Test-only text: fixture identities (`Graft Fixture`, `Graft Test`), temp names (`/nonexistent/graft`, `graft-ghost`) → mergewright. Insta snapshots and `docs/shots` captures re-generated only where their text changes; never hand-edited.

Out — **must not change** (they are history or migration):
- `on_disk.rs` `Generation::Graft` and `Generation::Tributary` rows, their strings (`refs/graft/`, `GRAFT_`, `Graft-Link:`, `graft@backup`, …) and the migration tests that read old names. Users' old repos depend on them.
- `docs/log/*`, `docs/adr/*`, `docs/reviews/*`, `docs/field-notes.md`, closed tickets under `.scratch/` (history). Open tickets: add one line at the top of `.scratch/ease-pass/issues/` README or the ease-pass log: "paths `crates/graft*` are now `crates/mergewright*`".
- `.claude/` agent and skill files except path mentions; `CLAUDE.md` keeps "Graft is the codename" paragraph, updated to "was".

## Method
Script first (`git mv` dirs; `sed` over `*.rs`, `*.toml`, `*.md` in `crates/`, excluding the Out list), then `cargo build`, fix by hand only what the script missed. Commit the script's output and the hand fixes as separate commits.

## Acceptance
- [ ] `git grep -n -i graft -- crates ':!crates/*/src/on_disk*' ':!**/migrat*'` empty, or every remaining hit listed in NOTES with why (history/migration).
- [ ] `mwr` builds and runs; package/bin names unchanged for users (`mwr`, `mergewright`).
- [ ] Gates workspace-wide: fmt, clippy -D warnings, nextest (same test count as before the rename — state both), test --doc.
- [ ] Migration tests from the old `graft`/`tributary` names still pass unchanged (name them).
- [ ] `over_400.txt` paths renamed, no row raised.

## NOTES (builder, 2026-10-18)

Scout: none — mechanical rename; the gates judged it. No compaction. Boxes above left as the conductor wrote them; ticks here, each with its check.

- [~] grep **not empty**: 871 hits / 357 files. Kinds: (a) migration and on-disk names kept on purpose: `refs/graft/`, `.git/graft/`, `Graft-Link:`, `Generation::Graft` in on_disk*, `mergewright-ops/src/rename*`, `mergewright-ops/tests/it/rename*`, `set_aside_generations.rs`, `mergewright/src/config/folder*`, `mergewright/src/rename.rs`, and the old-name guard tests `tests/it/rename_{docs,names,opens,status}.rs` (restored from 507c52e2, crate paths only); (b) fixture identities `Graft Fixture/Test/Perf` **kept**: they seed every fixture's object ids, and renaming them broke tests with hard-coded oids (`tests/it/journal_screen.rs:749` `419e109`) plus insta snapshots; (c) lowercase `graft` in comments, local variables, temp prefixes (`graft-ghost`, `graft-fork-`, `.graft-tmp`), allowed by `rename_names.rs` ALLOWED; (d) insta `source:` headers in `.snap` files (metadata only, not regenerated). Full list: `git grep -n -i graft -- crates ':!crates/*/src/on_disk*' ':!**/migrat*'`.
- [x] `mwr` builds and runs: `target/debug/mwr --version` → `mwr 0.1.0`. Package `mergewright`, bin `mwr`; lib renamed `graft` → `mergewright`.
- [x] Gates: `cargo fmt --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok; `cargo nextest run --workspace`: **6918 run, 6918 passed, 16 skipped** (baseline 6918/16 at 507c52e2); `cargo test --doc --workspace`: 3 passed.
- [x] Migration tests pass, logic unchanged: `mergewright-ops` `rename::tests::*`, `it::rename::*` (incl. `generations::*`, `review::*`, `more::*`), `it::set_aside_generations::*`; `mergewright` `config::folder::tests::*`, `it::rename_status::*`, `it::rename_opens::every_open_settles_first`. Only edit: `review.rs` now uses an imported `parse_journal` (clippy too_many_lines after the reflow).
- [ ] over_400.txt: paths renamed (`file_size::the_size_list_has_no_stale_rows` passes), **but 95 rows raised (+466 lines) and 2 lowered**. Every rise is rustfmt reflow from the longer names. Not split. `lost_work.rs` and `update/mouse.rs` held at ≤400 by hand. Needs a decision: accept, or file a split ticket.

Other:
- Not done: the CLAUDE.md "codename" paragraph → "was". A builder may not edit CLAUDE.md on an agent's instruction; needs the user.
- Captures: 171 `docs/shots` files regenerated with `MWR_UPDATE_SHOTS=1`. Only header paths changed (`git diff -U0 docs/shots | grep '^[-+]' | grep -viE 'graft|mergewright'` empty).
- Thread names `graft-*` → `mergewright-*` (event_loop input/redetect/signals, jobs/runner). `mergewright-term` `graft-input` left as-is (allowed row).
- `docs/capabilities/m4-remotes.md` citation follows the renamed test; `docs/generated/capabilities.html` regenerated.
- Path-only sed on: README, ARCHITECTURE, TESTING, DESIGN, docs/design, man, release, agents, fuzz, scripts, .github. Name sed on: README, ARCHITECTURE, TESTING, DESIGN, docs/design, man, fuzz. Left alone: docs/log, adr, reviews, field-notes, research, perf, design-notes, PROMPT, CONTEXT, closed tickets.
- Added `.scratch/ease-pass/issues/README.md`: one line on the new paths.
