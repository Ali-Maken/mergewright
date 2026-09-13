# m1-the-graph/33 — F3 cycles the graph renderer instead of only dropping

Status: ready
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
- Rename `drop_tier` / `is_tier_drop` to match (update MAP.md).

Out:
- A general key-hint line or `?` help (checkpoint question).

## Acceptance
- [ ] Tests: Pixel→Unicode→Ascii→Pixel; Unicode-top cycle; Ascii-only no-op; re-entering Pixel transmits again and places, no leaked ids (every uploaded id freed by exit).
- [ ] pty check (19's `pty-tier-drop-check.py` pattern): three presses leaves the terminal sane; capture under `docs/shots/33-*`.
- [ ] Real Ghostty (like 24d's `examples/kitty_check.rs`, or the app): cycle back to pixels gives 0 error replies — or say plainly it wasn't reachable.
- [ ] **By eye (user):** F3 cycles in Ghostty and the pixel graph comes back clean. Unticked until the user looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; stop at a clean seam with a handoff note.
