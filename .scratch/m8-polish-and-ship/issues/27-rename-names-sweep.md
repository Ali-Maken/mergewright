# G-rename 2 — the binary is `trib`, the app says Tributary, env vars are `TRIB_*`

Status: split (spec-only pass 2026-10-10, after scout `scouts/27-rename-sweep.md`: too big for one context) →
`27a-rename-command-and-app-name` → `27b-rename-env-names` → (`27c-rename-user-docs-and-scripts` ∥
`27d-rename-recaptures`). This file keeps the decisions; the four specs refine them (no `[[bin]]` exists today — 27a
adds one; scout's unlisted sites assigned there). Not built as one ticket. Was: ready.
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Sonnet, alone**: mechanical, held by a grep test.
Blocked by: 26
Needs: `rename-on-disk-migration`. Last before the acceptance ticket.

## Read first
- `docs/adr/0057-tributary-and-what-ships.md` §1–2
- `rename-on-disk-migration`'s NOTES (the collected "G-rename list")
- `crates/graft/MAP.md` `## cli` (`ASKPASS_SOCKET_VAR`), lib.rs examples list

## Decisions (planner)
- Binary `[[bin]] name = "trib"`; `APP_NAME = "Tributary"`; `--version` prints `trib <version>`; dist app/formula/
  archive names `tributary`; man page `docs/man/trib.1` (`--generate-man` output's NAME line `trib`).
- Env vars: every `GRAFT_*` read by the binary, tests and scripts → `TRIB_*` (`TRIB_PERF_TRACE`,
  `TRIB_ASKPASS_SOCKET`, `TRIB_FRAME_TRACE`, …). The old names are not read (nothing shipped under them).
- **Crate and package names stay `graft-*`** this milestone: they are not published (ADR-0057) and renaming them would
  break every `crates/graft/...:line` citation in the docs and gaps. A KNOWN_GAPS row for the conductor: "crate names
  carry the codename; rename before the source is published".
- History documents keep their words (ADRs, logs, reviews, field notes, KNOWN_GAPS, design notes). User-facing docs
  change: README, TESTING.md instructions, man page, `--help`. CLAUDE.md and PROMPT.md are the user's/conductor's —
  NOTES lists the lines that would change; this ticket does not edit them.

## Scope
In: binary name, `APP_NAME`, env vars, `CARGO_BIN_EXE_*` uses, scripts calling the binary, dist names, README,
TESTING.md commands, man page file, captures that print the name (re-shot). Out: crate names; history docs.

## Signatures
`pub const APP_NAME: &str = "Tributary";` (graft-ui). Nothing else.

## Acceptance
- [ ] Grep test: no `GRAFT_` env read and no user-visible `graft` string in non-test code of any crate (allow-list:
      crate/module paths, `refs/graft` inside `rename`'s old-name mapping) — test named.
- [ ] `cargo build` produces `target/debug/trib`; `trib --version`, `trib --help`, `trib --generate-man` outputs quoted.
- [ ] Every script under `scripts/` runs the new binary name (grep quoted).
- [ ] Captures showing the name or a `GRAFT_` var re-shot, listed in NOTES.
- [ ] `docs/release/plan.txt` regenerated: archive and formula named `tributary`.

## Touches
- Every test using `CARGO_BIN_EXE_graft`; m3–m7 walk examples and pty scripts; askpass child env; G-pace/G-trace env
  names; README; TESTING.md; dist metadata; MAP.md entries naming env vars or the binary.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens.

## Conductor addition (from 25 architecture pass, docs/reviews/m8-architecture.md M4 — as found, no new decision)

- On-disk names are format contracts. Two the pass named for 26/27: the `Graft-Link:` backup trailer — readers accept the old and new spelling, or 27's grep test allow-lists it, with a reason; and the rewrite's `plan.json` directory — migrated before a relaunch reads it. Each gets a box with a test in whichever of 26/27 owns it.
- Order (conductor): health 33–38 merge before 26; 39–42 need 27's final names, so they run after 27 (they are the exception to "G-health merged" above). 05 is deferred to last by the user (2026-10), so it is not a blocker here.
