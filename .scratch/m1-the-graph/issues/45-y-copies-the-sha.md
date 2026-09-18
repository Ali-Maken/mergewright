# m1-the-graph — `y` copies the selected commit's SHA (OSC 52)

Status: done (user paste check pending)
Build with: **Opus** (high) — a terminal protocol through tmux passthrough (CLAUDE.md Models); small
Needs: none strictly; touches the same footer/hint/key code as 41 — sequence after it
Blocked by: 41
Leaves runnable: yes — `y` sends the full SHA to the terminal clipboard and says so.

Why: ADR-0023 removes the tooltip, which was the only place the full SHA appeared (and the only copy route besides Shift+drag on the short SHA). Decision 4: `y` copies via OSC 52; `.` lists it when the context menu exists (M2+).

## Read first
- `docs/design-notes/m1-the-graph.md`: decision 2 (item 4), "New UI: Copied notice", "Interfaces" graft-term and graft::update lines
- `docs/adr/0023-…` Decision 4
- docs/design/02-keymap.md §2.1 (rare verbs behind `.`), §2.2–2.3 (`y` is unassigned)
- `crates/graft-term/MAP.md` (tmux: `Transport`, `tmux_passthrough`), `crates/graft/MAP.md` (update: `Command`, `Status`, `AppState::status`; event_loop: `pump`, how Kitty bytes reach stdout); scouts for source

## Scope
In:
- `graft_term::clipboard::osc52_copy(text) -> Vec<u8>`: `ESC ] 52 ; c ; <base64(text)> ESC \`. Written through `Transport::write`, so inside tmux it is wrapped like an image command. base64: the `base64` crate in `graft-term` (leaf dependency; say so in the notes) or a 30-line local encoder — builder's call, tested either way.
- `Command::CopyToClipboard(String)`; `y` with a selected row emits it carrying the full 40-hex SHA; no rows / no selection → nothing. `pump` writes the bytes to stdout through the session's transport; a write error propagates like any other stdout failure (never swallowed).
- `Status::Copied { short: String }` → footer notice `848ec2e… → clipboard (osc 52)` until the next key clears it. Wording says what Graft sent; Graft cannot see whether the terminal honoured it.
- Hint bar: `y sha`, dropping under width pressure like the others (33).
- KNOWN_GAPS: "Copy SHA under `.` context actions: M2+ (menu not built)".
- MAP.md for every public change.
Out: `.` menu; copying anything but the SHA; reading the clipboard; `pbcopy`/`xclip` fallbacks.

## Signatures
```rust
// graft-term::clipboard (new module)
pub fn osc52_copy(text: &str) -> Vec<u8>;
// graft::update
pub enum Command { /* existing */ CopyToClipboard(String) }
pub enum Status  { /* existing */ Copied { short: String } }
```

## Acceptance
- [x] Test: `osc52_copy("abc")` is exactly `\x1b]52;c;YWJj\x1b\\`; through `Transport::TmuxPassthrough` it is DCS-wrapped with `ESC` doubled.
- [x] Update tests: `y` on a selected row → `CopyToClipboard(<40 hex>)` and `Status::Copied`; `y` with no rows → no command, status unchanged; the next key clears the notice.
- [x] pty test (release binary, as 17/24 did): after `y`, the captured byte stream contains `\x1b]52;c;` followed by the base64 of the selected SHA; footer shows the notice. Capture `docs/shots/<NN>-y-copies-sha.txt`. User judges by eye and pastes in Ghostty (by-eye box, unticked until tried).
- [ ] User by eye: paste in Ghostty / Zed / VS Code / tmux gives the 40-hex SHA.
- [x] TESTING.md row: "Select a commit, press `y`: the bottom line reads `<7 hex>… → clipboard (osc 52)`; paste in another window gives the 40-character SHA. Press `j`: the line is gone."
- [x] KNOWN_GAPS and MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens.

## Notes (builder, 2026-09-14)
- base64: local 20-line encoder in `graft-term::clipboard`, no new dependency; RFC 4648 vectors tested.
- `CopyToClipboard` is written by `event_loop::frames` through `Transport::for_tmux(session.capabilities.tmux)`, under `hold_output`; the write error propagates out of `pump` (test: `a_failed_clipboard_write_ends_the_loop_with_its_error`). `jobs` has an `unreachable!` arm: never dispatched.
- Choice: `y` while `Status::Failed` sends the SHA but keeps the failure (never dropped); no notice then. In KNOWN_GAPS.
- pty test is a script like 41's: `.scratch/m1-the-graph/pty-copy-check.py` (asserts OSC 52 bytes, notice, notice cleared). Captures `docs/shots/45-y-copies-sha.txt`, `docs/shots/45-notice-cleared.txt`.
- Open question for the user: inside tmux the request is DCS-wrapped (per spec); tmux's native `set-clipboard` path would need it unwrapped. In KNOWN_GAPS.
- Not compacted.
