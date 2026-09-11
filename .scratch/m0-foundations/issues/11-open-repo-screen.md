# m0-foundations/11 — Wire it together: open a repo, show it, quit clean

Status: todo
Build with: Sonnet
Blocked by: 04, 05, 06, 07, 08, 09, 10

Why: this ticket is the M0 checkpoint itself — "Opens a repo; quitting leaves the terminal clean."

## Read first
- The Comments sections of tickets 03–10 for anything their builders left you
- ARCHITECTURE.md (SanitizedText boundary; `ui` renders view models, never repo handles)
- DESIGN.md §4.2 (config override precedence over detected tier)

## Goal
`cargo run -- <path>` (default: cwd) opens the repo and shows a status screen; quitting restores the terminal exactly.

## Scope
In:
- `main()` order (decide the exact sequence; suggested): load config (07) → on error print message, exit 1, before touching the terminal → enter `TerminalSession` (03, kitty-keyboard off) → run `Capabilities::detect` (04) → push kitty keyboard if supported → start loop (09) → submit "open repo" (08) through the job runner (10) → result event updates `AppState` → status screen.
- Status screen (a `ui` widget fed a plain view model `app` assembles): repo workdir path, HEAD state (branch / detached / unborn), rendering tier + provenance (mark assumed cell size / unknown theme hint as such), theme in effect, git-version warning if < 2.38, config warnings.
- Every repo-derived string crosses into `ui` only as `term::SanitizedText` (05) — the view model fields are typed that way.
- Tier shown = detection (04) combined with `graph.renderer` override per DESIGN.md §4.2 (override wins; forced pixel is display-only honesty at M0 — no graph exists yet).
- Theme = `Theme::select` (06) from `ui.theme` + OSC 11 hint; `NO_COLOR` honoured.
- Not a repo: leave the terminal session (or never enter it — your call, but the terminal must be clean), one clear stderr line, exit 1.
- Quit key exits; terminal restored (session from 03 does the work).

Out:
- Any graph, log, panes, keybindings beyond quit. First-run Init/Clone screen (later milestone).
- Auto-refresh, file watching.

## Acceptance
- [ ] `update`-level integration tests: open-repo success event → state shows repo facts; failure event → error state.
- [ ] Status-screen snapshot via `TestBackend` + `insta` with a fixed fake view model (incl. a branch name containing an ANSI escape — renders inert).
- [ ] Manual, in Ghostty or iTerm2 (note results in Comments): `cargo run -- .` shows repo + tier; `q` exits with no stray characters; `cargo run -- /tmp` prints the not-a-repo line and exits 1 with a clean terminal; kill -INT mid-run leaves the terminal clean.
- [ ] Fast-tier tests stay laptop-safe; no PTY end-to-end test (manual covers it).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
