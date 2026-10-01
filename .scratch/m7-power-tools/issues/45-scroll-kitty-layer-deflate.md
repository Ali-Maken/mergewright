# 45-scroll-kitty-layer-deflate — Kitty: a second layer above each strip, and zlib-compressed transmits

Status: ready
Build with: (blank: scout first — `graft-term::kitty` and `detect` are existing code)
Needs: none (additive; the wiring ticket places emphasis images)
Blocked by: none

## Read first
- `docs/adr/0053-selection-paint-is-its-own-layer.md` Decision 3–5.
- `42-scroll-held-key-gate.md` §Prior art (row "Compressed transmit").
- `crates/graft-term/MAP.md` `## kitty`, `## detect`, `## tmux`; `crates/graft-core/MAP.md` `Capabilities`.
- Kitty graphics protocol: https://sw.kovidgoyal.net/kitty/graphics-protocol/ — §"Compression" (`o=z`), §"z-index",
  §"Querying support" (`a=q`).
- `.scratch/m7-power-tools/debug-scroll-lag.md` cause 1 (`crates/graft-term/src/kitty/encode.rs:14-26`: `f=32`, no
  `o=z`).

## Why
A Pixel `j` sends whole RGBA strips as base64 (6.35 MB per frame). After ADR-0053 a `j` sends only emphasis images,
which are mostly transparent, and new strips at the viewport edge, also mostly transparent: both deflate well.
Emphasis must sit above its strip.

## Decided
1. **Layer.** `Place` gains `layer: Layer`. `Layer::Strip` keeps today's z; `Layer::Emphasis` is one z above
   `Strip`. Same cells, same crop rules; emphasis cell column = strip column + `first_column` (the caller computes).
2. **Deflate.** When on, every transmit carries `o=z` with the RGBA deflated (zlib stream, RFC 1950) at a fast level,
   then base64 and the existing chunking. `f=32`, `s`, `v` as today (the terminal knows the size from them). Encoded
   once per image, as today.
3. **On only when the probe says so.** `detect` adds one query to the existing Kitty graphics probe, in the same
   write and deadline: a 1×1 `a=q,f=32,o=z` image. An `OK` reply → `Capabilities::kitty_deflate = true`; anything else
   or silence → `false`. No extra round trip. Over tmux, through `Transport` as today.
4. **Not built here:** a runtime fallback. A terminal that answered `OK` to the deflated query takes deflated data.
5. **Capacity.** `KittyImages::new(capacity)` stays the caller's number; the wiring ticket doubles it for two layers.
6. **Terminal facts are verified, not assumed** (rule 7): the scout checks from source or docs, with links, that
   Ghostty and Kitty (and WezTerm if cheap) (a) accept `o=z` with `f=32`, (b) composite overlapping placements by
   alpha, (c) honour `z`. Any "no" → **stop** and report before building; ADR-0053 then needs another answer.

## Signatures
```rust
// graft-core
pub struct Capabilities { /* existing fields */ pub kitty_deflate: bool }   // false wherever not probed

// graft-term::kitty
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Layer { #[default] Strip, Emphasis }
pub struct Place<'a> { /* first_row, frame, rows, cells as today */ pub layer: Layer }
impl KittyImages {
    /// Builder: transmit with `o=z`. Off by default.
    pub fn deflating(self, on: bool) -> Self;
}
// KittyImages::frame unchanged in signature; it now orders and z-places by `layer`.
// stats the frames harness reads (scout names today's counter): transmits split by Layer, bytes before/after deflate.

// graft-term::detect — `detect` signature unchanged; it fills `kitty_deflate`.
```
If `Capabilities` or `Place` are built with struct literals outside graft-core/graft-term (grep), those sites gain the
field with `false` / `Layer::Strip` and nothing else — list them; they are this ticket's files.

## Acceptance (each box names its test; red first)
- [ ] Deflated transmit: `o=z` present, payload base64-decodes and inflates to the frame's exact RGBA.
      — `kitty::tests::deflated_transmit_round_trips`.
- [ ] Deflate off → byte-identical to today's output for the same frames (pin with today's bytes captured first).
      — `kitty::tests::deflate_off_is_unchanged`.
- [ ] A frame with a strip and its emphasis places the emphasis one z above, at its column; dropping the emphasis next
      frame deletes only its placement; the strip is not re-sent.
      — `kitty::tests::emphasis_sits_above_its_strip`, `kitty::tests::emphasis_goes_strip_stays`.
- [ ] Deflate on a mostly-transparent 187-column strip (synthetic frame: 6 thin lanes) is ≤ 10% of raw bytes and
      encodes in ≤ 2 ms (release, printed; recorded by `scripts/fix-scroll-deflate-cost.sh` →
      `docs/perf/45-scroll-kitty-layer-deflate.txt`; size asserted in both builds, time printed only).
      — `kitty::tests::deflate_shrinks_a_sparse_strip`.
- [ ] Probe: `OK` to the deflated query → `kitty_deflate`; an error reply or silence → not; Kitty graphics itself
      unaffected either way; no added deadline. — `detect::tests::{deflate_ok_turns_it_on,
      deflate_error_leaves_it_off, deflate_silence_leaves_it_off}` (scripted `TerminalHost`).
- [ ] Inside tmux: the new query and deflated chunks go through `Transport` wrapping.
      — `kitty::tests::deflated_chunks_wrap_in_tmux`.
- [ ] Terminal facts (Decided 6) recorded in NOTES with links.
- [ ] MAP.md (graft-term, graft-core): `Layer`, `Place::layer`, `deflating`, `kitty_deflate`.

## Files (exact)
- `crates/graft-term/src/kitty.rs`, `src/kitty/*.rs` (encode, frame; scout lists), `src/detect.rs` and
  `src/detect/*.rs` (probe write and reply parse; scout lists), their tests, `crates/graft-term/MAP.md`.
- `crates/graft-core/src/<capabilities file>` (scout names), `crates/graft-core/MAP.md`.
- Struct-literal sites outside those crates, field only (grep result in the report).
- `Cargo.toml` of graft-term only if a deflate crate is needed: prefer one already in `Cargo.lock` (the `png`
  encoder's); check ARCHITECTURE.md's dependency rules; say which.
- `scripts/fix-scroll-deflate-cost.sh`, `docs/perf/45-scroll-kitty-layer-deflate.txt`.
- **Not** `crates/graft/src/event_loop/graphics.rs` beyond literal sites (wiring ticket).

## Touches
- `KittyImages::answer` (`ENOENT` forgets an upload): an emphasis image forgotten retransmits like a strip; test or
  argument.
- `forget_uploads` after clear/resize, `free_all` on exit, `hide`: cover emphasis images; one test each or argued.
- iTerm2 path: untouched (it gets composites from the wiring ticket).
- First-paint budget (150 ms, ADR-0021): one more query in the same write; `heavy::budgets_hold` unchanged — say how
  checked (the probe deadline is unchanged; no new wait).
- Captures: none; Kitty bytes are not captured (raw streams never committed).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium (~50k).
