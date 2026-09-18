# m1-the-graph/19 — The ASCII tier and the renderer override

Status: done
Build with: **Sonnet** (medium) — a glyph table over an existing renderer, a config key, a key binding
Blocked by: 13
Leaves runnable: yes — `graph.renderer = ascii` works, and a key drops a tier when output looks garbled.

Why: a user must always be able to force or drop a tier, and piped output must never be escape soup.

## Read first
- PROMPT.md §1.1 (the ASCII tier paragraph only)
- docs/design/04-terminal-capability-detection.md §4.2 and §4.6
- ADR-0016
- Code: 08's widget and 13's shape channel, `crates/graft-term/src/detect/` (tier decision, public API only), the config module

## Scope
In:
- ASCII glyphs (`* | / \ - +`) as a glyph table in 08's renderer, not a second renderer.
- Lane identity from 13's shape channel; `+N` drawn in ASCII.
- `graph.renderer = auto | unicode | ascii`, applied at the app layer over what `detect` returned. (`pixel` is added by 24.)
- One key drops one tier and says what it did in the status line.
- Non-TTY stdout lands on ASCII without complaint.

Out:
- Pixel tiers (22–25, 29). Changing detection.

## Acceptance
- [x] Snapshots of criss-cross, foxtrot, octopus and the widest fixture in ASCII at 80×24, with and without colour. (`crates/graft-ui/tests/ascii_tier.rs`)
- [x] Each `graph.renderer` value forced from config is snapshot-tested. (`crates/graft/src/view.rs::tests::every_graph_renderer_config_value_draws_as_it_says`)
- [x] The tier-drop key works from the graph screen and leaves the terminal sane (pty). Chose **F3** (unused key). Verified with a real pty (`.scratch/m1-the-graph/pty-tier-drop-check.py`, pattern from `.scratch/m0-foundations/pty-signal-check.py`); termios identical before launch and after quit. Captures: `docs/shots/19-ascii-tier-drop-key.txt`, `docs/shots/19-ascii-tier-crisscross.txt`.
- [x] `graft <fixture> | cat` output contains no escape bytes (asserted). (`crates/graft/tests/it/piped_stdout.rs`) Implemented via a new headless path: `event_loop::run` checks `io::stdout().is_terminal()` and, when false, runs `run_headless()` — same `update`/`Command`/`JobRunner` machinery, prints one frame through `TestBackend` (never emits ANSI) instead of opening a real terminal.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)
All green, workspace-wide, at time of completion.

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes
This ticket ran across two prior compactions before code was written (per handoff notes from earlier sessions), then finished in this session without further compaction. Key choice: **F3** for the tier-drop key (not in the taken list: Space, 1, H, +, F, Enter, Esc, j/k/g/G, F2).
