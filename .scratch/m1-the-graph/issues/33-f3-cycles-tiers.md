# m1-the-graph/33 — F3 cycles the graph renderer instead of only dropping

Status: done (by-eye box open)
Build with: **Opus** (high) — re-entering the pixel tier (uploads, strip padding)
Blocked by: none (run after 32)
Leaves runnable: yes — F3 steps pixels → Unicode → ASCII → back to the top tier.

User decision (2026-09-13, Ghostty check): F3 today is one-way (24d: Pixel→Unicode, else→Ascii; relaunch to recover). Make it a **cycle**.

## Read first
- Ticket 24d, "Handoff" notes on `drop_tier`, `AppState::settled_tier`, `strip_rows`
- Ticket 19, "Acceptance" (F3 chosen, pty tier-drop check)
- ADR-0016 (tier order)
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-term/MAP.md`. Scouts for source.

## Scope
In:
- F3 cycles: top → next lower → … → ASCII → top. **Top = the tier the terminal settled on** (detection or `graph.renderer` override), never higher. On a Unicode-only terminal F3 toggles Unicode ↔ ASCII; on ASCII-only it does nothing.
- Re-entering pixels works: uploads are sent again (don't trust old ids — free them on leaving, or `forget_uploads` and free), strips padded again (`strip_rows`), `gfx` status back.
- Footer names the current tier, e.g. `renderer: unicode (F3 cycles)`; nothing extra shown at the top tier.
- **Hint bar (PROMPT.md principle 4, line 198; docs/design/01-view-mockups.md §1.16):** every graph key that works now appears in 09's footer hints, F3 included. User saw no F3 hint in Ghostty. Check which graph keys are missing from the footer (Space, 1, H, +, F, F2, F3, Enter, /, n/N) and how it drops keys at narrow widths; add the missing ones in the same drop order.
- Rename `drop_tier` / `is_tier_drop` to match (update MAP.md).

Out:
- The `?` context help overlay (docs/design/01-view-mockups.md §1.16): M8 "keybinding help and remapping".

## Acceptance
- [x] Tests: Pixel→Unicode→Ascii→Pixel; Unicode-top cycle; Ascii-only no-op; re-entering Pixel transmits again and places, no leaked ids (every uploaded id freed by exit).
- [x] Footer hints list every working graph key (snapshot at 80, 120, 200 cols), F3 included.
- [x] pty check (19's `pty-tier-drop-check.py` pattern): three presses leaves the terminal sane; capture under `docs/shots/33-*`.
- [x] Real Ghostty (like 24d's `examples/kitty_check.rs`, or the app): cycle back to pixels gives 0 error replies — or say plainly it wasn't reachable.
- [x] **By eye (user, 2026-09-13: "f3 works, footer shows keys"):** F3 cycles in Ghostty and the pixel graph comes back clean. Unticked until the user looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; stop at a clean seam with a handoff note.

## Done (2026-09-13, builder-opus, no compaction)

Built:
- `update.rs`: `drop_tier`/`is_tier_drop` → `cycle_tier`/`is_tier_cycle`. Pixel→Unicode→Ascii→settled tier; override clears at the top (`None`), so nothing extra shows there. Top = `settled_tier`, else Unicode. `strip_rows` follows the tier both ways.
- `view.rs`: `tier_in_force(session, state)`; footer notice `renderer: <tier> (F3 cycles)` below the top only.
- `graphics.rs` `frame`: no pixels this frame and tier not Pixel → `free_all` (fresh ids on return); status screen at Pixel still only `hide`s.
- `graph_screen`: footer lists every graph key: `q F2 F3 [+] / j/k Enter Space 1 F H PgUp/PgDn g/G Ctrl+R`. Narrow widths drop **whole** hints from the right (before: `fit` cut mid-hint). `n/N` stay on the search line, where they work.
- `examples/kitty_check.rs`: new step 5, F3 away (free all) and back (transmit + place).

Verified:
- Tests: Unicode-top toggle (and no session), Ascii-only no-op, Pixel→Unicode→Ascii→Pixel with `strip_rows`; event loop: leaving pixels frees every frame-0 id, return transmits new ids + places + `gfx ok`, every id freed by exit. Footer: every hint at 200 cols, snapshots at 80/120/200, cut on hint boundaries. Other snapshot changes are footer lines only (checked by diff).
- pty (`.scratch/m1-the-graph/pty-tier-cycle-check.py`): 3 presses, Unicode↔ASCII (pty acks no image protocol), termios identical, exit 0. `docs/shots/33-pty-tier-cycle.txt`.
- **Real Ghostty 1.3.1** (kitty_check, synthetic strips through the app's encoder): F3 away + back, 0 error replies, control ENOENT heard. `docs/shots/33-kitty-check-report.txt`, `.esc-digest.txt` (the raw `.esc` removed in m8-polish-and-ship/22), `.esc.txt`.
- Not verified: the *app's* pixel graph cycling in a real terminal, or how anything looks.

## Ghostty command (user, by eye)

```
cd <home____________>/dev/sandbox/gitter && cargo build -p graft && open -na /Applications/Ghostty.app --args -e <home____________>/dev/sandbox/gitter/target/debug/graft <home____________>/dev/sandbox/gitter
```

Look for: footer shows `F3 renderer`; F3 → Unicode graph, footer `renderer: unicode (F3 cycles)`, no leftover images; F3 → ASCII; F3 → pixel graph back, header `gfx ok`, footer hints again, lanes aligned; `q` leaves no images.
