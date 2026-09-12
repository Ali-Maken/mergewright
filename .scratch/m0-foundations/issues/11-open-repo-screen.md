# m0-foundations/11 — Wire it together: open a repo, show it, quit clean

Status: done
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
- [x] `update`-level integration tests: open-repo success event → state shows repo facts; failure event → error state.
- [x] Status-screen snapshot via `TestBackend` + `insta` with a fixed fake view model (incl. a branch name containing an ANSI escape — renders inert).
- [x] Manual, in Ghostty or iTerm2 (note results in Comments): `cargo run -- .` shows repo + tier; `q` exits with no stray characters; `cargo run -- /tmp` prints the not-a-repo line and exits 1 with a clean terminal; kill -INT mid-run leaves the terminal clean.
- [x] Fast-tier tests stay laptop-safe; no PTY end-to-end test (manual covers it).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Comments

**Built 2026-09-12** (picked up a previous builder's uncommitted work; kept the
`Product`/`AppEvent` job-result design, replaced the half-done view and loop).

Startup order as suggested, with one change: **kitty keyboard is pushed after
the probe, not at session open.** The probe needs raw mode, so the session has
to exist before detection can say whether the terminal speaks the protocol.
`TerminalSession::enable_kitty_keyboard()` applies the step late so teardown
still pops it in order. Detection also runs *before* `input::spawn_reader` —
two readers on stdin would eat the replies.

Not-a-repo path: the session is entered, the failed open-repo job becomes
`Quit::Fatal`, the loop returns `Exit::Fatal(reason)` and `main` prints one
stderr line once the terminal is the user's again. Cost: a brief alternate-screen
flash (KNOWN_GAPS).

`repository::facts` now makes the workdir absolute (`std::path::absolute`, not
`canonicalize` — symlinks stay as the user knows them), because `graft .` would
otherwise put `.` on screen as the repository path.

### Manual results

Driven through a real pty (`pty.fork`, 100x30, `TERM=xterm-256color`) — not a
GUI terminal, so **the M0 checkpoint still needs the user's eyes** (KNOWN_GAPS).

| Check | Result |
|---|---|
| `graft .` | `Graft` / `Repository  /Users/…/gitter` / `HEAD  main` / `Rendering  unicode, detected, cell 8x16 (assumed)` / `Theme  dark, terminal background unknown, truecolor` / `q  quit` |
| `q` | exit 0, alternate screen left, every mode popped, cursor shown |
| `graft /tmp` | `graft: /tmp is not inside a git repository`, exit 1, terminal restored first |
| `SIGINT` mid-run | dies of the signal (exit code -2), terminal fully restored |
| `NO_COLOR=1` | no SGR colour emitted at all; theme line reads `no colour (NO_COLOR)` |

Nothing answered the probe in the pty, so the tier shown was the silent-terminal
fallback. A real emulator may pick a different tier — that is the checkpoint's job.
