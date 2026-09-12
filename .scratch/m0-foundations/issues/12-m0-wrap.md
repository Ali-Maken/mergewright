# m0-foundations/12 — M0 wrap: README, CHANGELOG, gaps sweep

Status: done
Build with: Sonnet
Blocked by: 02, 11

Why: PROMPT.md §8 — every milestone ends with gates green, README + CHANGELOG updated, short demo steps, KNOWN_GAPS.md current. The reviewer checks this next.

## Read first
- PROMPT.md §8 preamble (the end-of-milestone list)
- KNOWN_GAPS.md as it stands
- `git log --oneline` for what M0 actually shipped

## Goal
Close out M0 so the reviewer and the user can verify it in minutes.

## Scope
In:
- README.md: what exists after M0 (open a repo, status screen, tier detection, clean quit), how to build/run, and the demo steps: `cargo run -- <repo>`, what you should see, `q` to quit, the not-a-repo case. Codename note per CLAUDE.md (no public naming).
- CHANGELOG.md: an M0 entry (Keep-a-Changelog style, no version number — the name and versioning land in M8).
- KNOWN_GAPS.md sweep: confirm every M0 stub/deferral has a row with an expiry (probe typeahead + resize re-probe should already be there as M1 gaps; add anything tickets 01–11 left in Comments). Remove nothing that isn't actually closed.
- Run all three gates across the workspace one final time and record the timings in this ticket's Comments (fast tier must be under 3 min — PROMPT.md §7.1).

Out:
- TESTING.md (its manual tier starts at M1). Ticking PROGRESS.md milestone boxes (conductor's job). Any code changes beyond doc fixes — if a gate fails here, that's a reopened ticket, not a wrap fix.

## Acceptance
- [x] README demo steps work verbatim on a fresh clone (walk them yourself).
- [x] CHANGELOG lists M0's user-visible behaviour, not internals.
- [x] KNOWN_GAPS.md has no stale M0 rows and no missing ones.
- [x] All gates green, timings recorded.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Comments

### Gates (2026-09-12)

Run one at a time. Cold = straight after `cargo clean` (2.1 GiB, 29172 files
removed), which is what a fresh clone gets. Warm = the same commands again,
nothing changed.

| Gate | Cold | Warm | Result |
|---|---|---|---|
| `cargo fmt --check` | 0.07 s | 0.09 s | clean |
| `cargo clippy --all-targets -- -D warnings` | 13.67 s | 0.17 s | no warnings |
| `cargo test` | 13.38 s | 0.57 s | **268 passed, 0 failed, 0 ignored** |

Fast tier cold total ~27 s against the 3-minute cap (PROMPT.md §7.1). Machine
stayed quiet; nothing ran in parallel.

### The demo steps, walked verbatim

Under a `pty.fork` harness with `TIOCSWINSZ` + `TIOCSCTTY` (so the binary sees a
real tty), against this repo:

- `cargo run -- <repo>` → status screen, then `q` → exit 0, last bytes
  `ESC[?1049l`, alternate screen left.
- `cargo run -- /tmp` → `graft: /tmp is not inside a git repository`, exit 1.
- `cargo run -- --help` → usage, exit 0.
- `NO_COLOR=1 cargo run -- .` → theme line reads `no colour (NO_COLOR)`; the
  only SGR left in the byte stream is the reset (`ESC[m` / `ESC[0m`), no colour
  parameters at all.
- `cargo run -p graft --example capability_probe` → `tier Unicode`, every query
  `Silent`, `probe TimedOut in 200.496625ms`, `cell 8x16 (assumed)`,
  `theme Dark (assumed, not reported)`.

A bare pty is not an emulator: nothing answered any probe, and no human has
looked at the screen. README and KNOWN_GAPS both say so plainly.

### Found while walking the demo — not fixed here

The status screen prints `Rendering  unicode, detected, …` in the same pty run
where the probe reported `TimedOut` and every query `Silent`. `tier_forced`
(`session.rs`) only records a config override, so `status_screen::rendering`
prints `detected` to mean "not forced". The cell size and theme carry honest
provenance; the tier does not. That is a code change, which this ticket's scope
excludes, so it is a KNOWN_GAPS row with expiry M1 (M1 owns the rendering line).
The README's demo section warns about the word.

### KNOWN_GAPS after the sweep

Every M0 row re-checked against the code, not against the ticket that wrote it.
Nothing was closed — no row turned out to be stale-because-done. Corrected the
"screen only seen in a synthetic pty" row with m0/12's evidence. Added four
rows: the `detected` wart above; the six places the code diverges from DESIGN.md
with no ADR (expiry M0 review); `graft-graph`/`graft-ops` still empty (M1/M3);
only the fast test tier exists — no heavy tier, criterion, cargo-fuzz, §7.2
corpus or CI (M1). Added a preamble to `## M0 gaps` stating what M0 has and has
not been seen doing, and that the four **M0 review** rows are the checkpoint's
job, not a builder's.

Also added item 4 to PROGRESS.md's "Open for the M0 checkpoint": the
DESIGN.md §4.2–§4.4 detection deviations need an ADR + spec edit, or a fix.
