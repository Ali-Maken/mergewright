# m0-foundations/07 — Config loading

Status: done
Build with: Sonnet
Blocked by: 04, 06

Why: the tier and theme overrides in docs/design/04-terminal-capability-detection.md §4.2 / docs/design/05-visualization.md §5 need somewhere to live; PROMPT.md fixes the mechanism (TOML + serde) and the path.

## Read first
- PROMPT.md Technology section ("Config:" line) and §5 (`ui.icons`)
- docs/design/04-terminal-capability-detection.md §4.2 (`graph.renderer` override semantics)

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
- [x] Tests with `tempfile`: missing → defaults; valid → parsed; malformed → error names file and problem; unknown key → warning, still loads.
- [x] No test touches `~`.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Done (2026-09-12)

`crates/graft/src/config/` — `mod.rs` and `tests.rs` (18 tests). Public shape:
`load()`, `load_from(&Path)`, `default_path()`, `Loaded { config, path,
warnings }`, `ConfigError::{Read, Parse}`, `ConfigWarning::{UnknownKey,
NoConfigHome}`.

- Two passes over the same text: serde for the typed `Config` (unknown keys
  ignored), then a `toml::Table` walk against a small `SCHEMA` table to report
  exactly what serde ignored. `deny_unknown_fields` was rejected: it makes
  tomorrow's key today's fatal error.
- Value spellings are locked to what tickets 04 and 06 built.
  `ThemeChoice::overridden() -> Option<ThemeKind>` feeds `ThemeKind::select`
  directly; `RendererChoice::overridden() -> Option<RenderingTier>`. Both
  conversions are tested with exhaustive `match`es, so a new theme or tier
  fails to compile until the config file can name it.
- An invalid value is an error, never a default. Verified by eye, not just by
  assert — `theme = "blu"` prints:

      <path>/config.toml:
      TOML parse error at line 2, column 9
        |
      2 | theme = "blu"
        |         ^^^^^
      unknown variant `blu`, expected one of `auto`, `dark`, `light`, `high-contrast`

- Missing file → defaults, silently. Any *other* read failure (a directory in
  its place, permissions) is `ConfigError::Read`, not defaults.
- `default_path()` honours `XDG_CONFIG_HOME` (ignoring a non-absolute value,
  per the XDG spec) then `$HOME/.config`. The env read is injected for tests,
  so nothing here touches a real home and nothing mutates the process
  environment.
- New deps on `graft`: `serde` (derive), `toml` 1.1 (`display` default off —
  Graft reads config, never writes it), `tempfile` (dev).

Not done, and in KNOWN_GAPS.md: nothing calls `load()` yet, so no setting
changes anything and the warnings go nowhere — ticket 11. `ui.icons` has no
consumer until M1's widgets.
