# 01 — codename rename: `graft` → `mergewright` in code, crates, docs

Status: ready (conductor, 2026-10-18; user: "first codename rename", quota-saving — one helper, mostly scripted)
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
