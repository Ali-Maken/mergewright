# m0-foundations/07 — Config loading

Status: todo
Build with: Sonnet
Blocked by: 04, 06

Why: the tier and theme overrides in DESIGN.md §4.2/§5 need somewhere to live; PROMPT.md fixes the mechanism (TOML + serde) and the path.

## Read first
- PROMPT.md Technology section ("Config:" line) and §5 (`ui.icons`)
- DESIGN.md §4.2 (`graph.renderer` override semantics)

## Goal
`graft` (app): load `${XDG_CONFIG_HOME:-~/.config}/graft/config.toml` into a typed `Config` with serde.

## Scope
In:
- Keys M0 needs, defaults in parentheses: `ui.theme` = auto|dark|light|high-contrast (auto); `graph.renderer` = auto|pixel|unicode|ascii (auto); `ui.icons` = plain|nerd (plain).
- Missing file → defaults, silently. Malformed file → a clear typed error naming the file, line and problem; the caller (ticket 11) exits cleanly with that message, no panic, no half-initialised terminal.
- Unknown keys tolerated, collected as warnings on the load result (forward compatibility; later milestones surface them in diagnostics).
- Path override for tests (inject the path; never read the real home dir in tests).

Out:
- Watching the config file; writing config; any other keys (add per-milestone as features need them).
- Applying the overrides (ticket 11 combines config with `Capabilities`).

## Acceptance
- [ ] Tests with `tempfile`: missing → defaults; valid → parsed; malformed → error names file and problem; unknown key → warning, still loads.
- [ ] No test touches `~`.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
