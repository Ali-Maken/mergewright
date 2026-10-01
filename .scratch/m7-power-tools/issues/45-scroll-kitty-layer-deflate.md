# 45-scroll-kitty-layer-deflate — Kitty: a second layer above each strip, and zlib-compressed transmits

Status: ready
Build with: Sonnet — additive encoder and probe work in mapped files (`.scratch/m7-power-tools/scouts/45.md`); the one
judgement call (terminal behaviour) is a gate below, not a build decision.
Needs: none (additive; the wiring ticket places emphasis images)
Blocked by: none for the deflate half. The `Layer` half is gated on Step 0 (below).
File set:
- graft-term: `src/kitty/{mod.rs, encode.rs, tests.rs}`, `src/detect/{probe.rs, reply.rs, mod.rs, tests.rs}`,
  `src/iterm2.rs` (test literal `:250` only), `Cargo.toml`, `MAP.md`.
- graft-core: `src/capabilities.rs`, `MAP.md`.
- Literal sites, new field only: `crates/graft/src/event_loop/graphics.rs:168` (`Place`),
  `crates/graft/examples/kitty_check/driver.rs:165` (`Place`), `crates/graft/src/session.rs:245` (`Capabilities`),
  and whichever of `crates/graft-ui/{tests/support/mod.rs, examples/graph_column.rs, benches/row_render.rs,
  src/test_support.rs, src/graph_screen/tests.rs}`, `crates/graft/tests/it/{captures.rs, frames/journal_find.rs,
  frames/run.rs}` are full literals (compile tells; list them).
- `scripts/fix-scroll-deflate-cost.sh`, `docs/perf/45-scroll-kitty-layer-deflate.txt`.
Overlap and order:
- 43 (crate `graft`): disjoint files; may run concurrently (two builders in `graft`, ADR-0051).
- 42 (crate `graft`, new `tests/it/frames/{drain_sink,held_key}.rs`): disjoint; whichever merges second re-runs the
  gates — a full `Capabilities`/`Place` literal in 42's new files gains the field then.
- 46 holds `event_loop/graphics.rs` after this ticket: the `:168` edit here is the field only. 46 Blocked by 45.
- 41, 44: none.

## Read first
- `.scratch/m7-power-tools/scouts/45.md` (whole; 1 page).
- `docs/adr/0053-selection-paint-is-its-own-layer.md` Decision 3–5.
- `42-scroll-held-key-gate.md` §Prior art (row "Compressed transmit").
- `crates/graft-term/MAP.md` `## kitty` (`:45-54`), `## detect` (`:15-29`), `## tmux`; `crates/graft-core/MAP.md`
  `Capabilities`.
- Kitty graphics protocol, https://sw.kovidgoyal.net/kitty/graphics-protocol/ — §"Compression" (`o=z`), §"Querying
  support and available transmission mediums" (`a=q`), §"z-index" (placement `z=`), and the sections on overlapping
  images and alpha.

## Why
A Pixel `j` sends whole RGBA strips as base64 (6.35 MB per frame). After ADR-0053 a `j` sends only emphasis images,
which are mostly transparent, and new strips at the viewport edge, also mostly transparent: both deflate well.
Emphasis must sit above its strip.

## Terminal facts: what is and is not confirmed (rule 7)
This spec pass had no web access; nothing below was read from a page in this pass. Status per fact:

| Fact | Kitty | Ghostty | How Graft is safe if it is wrong |
|---|---|---|---|
| (a) accepts `o=z` with `f=32` | protocol §Compression defines it (unquoted here) | **unconfirmed** | **probe + runtime fallback** (Decided 3, 4) |
| (a′) `a=q` really decodes the payload | protocol §Querying says the terminal tries to load the image (unquoted here) | **unconfirmed** | runtime fallback (Decided 4) covers a terminal that answers `OK` without decoding |
| (b) overlapping placements blend by alpha | not quoted | **unconfirmed** | **no probe exists** → Step 0 gate |
| (c) higher `z` drawn above | protocol §z-index (unquoted here) | **unconfirmed** | **order belt** (Decided 1): emphasis also has the higher id and is placed after its strip |

Argument for (b), not proof: today's strips are already RGBA, mostly transparent, and show the cell background through
(the Pixel tier he has seen), so the image pass blends; whether one image blends over another image is the open part.

**Step 0 (before writing `Layer`):** NOTES quote, with anchors/links, Kitty's spec text for (a), (a′), (b), (c) and
Ghostty source at a pinned commit (`github.com/ghostty-org/ghostty`, `src/terminal/kitty/graphics_*.zig`, file +
line) for the same four. WezTerm only if cheap. Builder without web access → report `blocked: terminal facts` and the
conductor dispatches `researcher` for exactly this table. **(b) "no" or unconfirmable in Kitty or Ghostty → stop**:
ADR-0053 then needs another answer (e.g. Kitty also takes `composited`, one image per strip, deflated). The deflate
half does not wait on Step 0: the probe and fallback make it safe either way.

## Decided
1. **Layer.** `Place` gains `layer: Layer`. `Layer::Strip` sends **no** `z=` key, exactly as today (none is sent,
   `encode.rs:52-83`; protocol default 0) — so deflate-off output stays byte-identical. `Layer::Emphasis` sends `z=1`.
   Same cells, same crop rules; emphasis cell column = strip column + `first_column` (the caller computes).
   **Order belt** for terminals that ignore `z` but stack by id or by placement order: emphasis image ids have bit 31
   set (`EMPHASIS_ID_BIT`), strip ids never do (assert at allocation); within a frame every strip placement is
   emitted before any emphasis placement, and a frame that (re)places a strip re-places its emphasis after it (a
   placement costs no transmit). The id bit also lets the frames harness tell the layers apart from bytes alone.
2. **Deflate.** When on, every transmit carries `o=z` with the RGBA deflated (zlib stream, RFC 1950) at a fast level,
   then base64 and the existing chunking (`CHUNK`, `encode.rs:11`). `f=32`, `s`, `v` as today. Encoded once per
   image, as today.
3. **Probe.** `detect` adds one query to the existing Kitty graphics probe (`probe.rs:20`, `full_round` `:60`, tmux
   `graphics_round` `:76`), in the same write, before the DA1 fence, same deadline (`detect/mod.rs:145`): a 1×1
   `a=q,t=d,f=32,s=1,v=1,o=z` image with **its own id** (31 is taken by the graphics query; pick an unused one) and a
   4-byte RGBA payload, deflated then base64 (a const; a test inflates it back to 4 bytes). Replies are matched **by
   id**: `reply::kitty_graphics` (`reply.rs:259`) today accepts any `i=`; it must accept only the graphics query's
   id, and a new parse reads the deflate query's. `OK` → `Capabilities::kitty_deflate = true`; error or silence →
   `false`. Over tmux, through `Transport` as today.
4. **Runtime fallback.** `KittyImages::answer` today forgets an upload on `ENOENT` (`kitty/mod.rs:191`). It also: any
   **other** error reply for an image whose last transmit was deflated → deflating off for the rest of this
   `KittyImages`'s life, and that upload forgotten so it retransmits raw next frame. One-way, no retry loop: raw is
   today's known-good encoding. Cost: one wasted transmit per image in flight when it trips.
5. **Capacity.** `KittyImages::new(capacity)` stays the caller's number; the wiring ticket doubles it for two layers.
6. **No stats in graft-term.** There is no transmit counter here today (scout 45 item 2); the frames harness counts
   from bytes (`crates/graft/tests/it/frames/sink.rs:95`, `readings.rs:99`) and splits layers by `EMPHASIS_ID_BIT`.

## Signatures
```rust
// graft-core
pub struct Capabilities { /* existing fields */ pub kitty_deflate: bool }   // false wherever not probed (`unprobed`)

// graft-term::kitty
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Layer { #[default] Strip, Emphasis }
/// Bit set in every `Layer::Emphasis` image id and in no strip id.
pub const EMPHASIS_ID_BIT: u32 = 1 << 31;
pub struct Place<'a> { /* first_row, frame, rows, cells as today */ pub layer: Layer }
impl KittyImages {
    /// Builder: transmit with `o=z`. Off by default.
    pub fn deflating(self, on: bool) -> Self;
    /// Still deflating: false once an error reply turned it off (Decided 4).
    pub fn is_deflating(&self) -> bool;
}
// KittyImages::frame unchanged in signature; it now orders, ids and z-places by `layer`.

// graft-term::detect — `detect` signature unchanged; it fills `kitty_deflate`.
```

## Acceptance (each box names its test; red first)
- [ ] Deflated transmit: `o=z` present, payload base64-decodes and inflates to the frame's exact RGBA.
      — `kitty::tests::deflated_transmit_round_trips`.
- [ ] Deflate off, all `Layer::Strip` → byte-identical to today's output for the same frames (today's bytes pinned
      before any change). — `kitty::tests::deflate_off_is_unchanged`.
- [ ] A strip and its emphasis: emphasis placed with `z=1`, id has `EMPHASIS_ID_BIT`, emitted after its strip, at its
      column; dropping the emphasis next frame deletes only its placement; the strip is not re-sent; a frame that
      re-places the strip re-places the emphasis after it.
      — `kitty::tests::{emphasis_sits_above_its_strip, emphasis_goes_strip_stays, emphasis_follows_a_moved_strip}`.
- [ ] Error reply (not `ENOENT`) to a deflated transmit → `is_deflating()` false, that image retransmitted raw next
      frame, no `o=z` after; `ENOENT` behaves as today. — `kitty::tests::a_deflate_error_falls_back_to_raw`.
- [ ] Deflate on a mostly-transparent 187-column strip (synthetic frame: 6 thin lanes) is ≤ 10% of raw bytes and
      encodes in ≤ 2 ms (release, printed; recorded by `scripts/fix-scroll-deflate-cost.sh` →
      `docs/perf/45-scroll-kitty-layer-deflate.txt`; size asserted in both builds, time printed only).
      — `kitty::tests::deflate_shrinks_a_sparse_strip`.
- [ ] Probe: `OK` to the deflate query → `kitty_deflate`; error or silence → not; graphics `OK` alone does not set
      deflate, deflate `OK` alone does not set graphics; no added deadline. — `detect::tests::{deflate_ok_turns_it_on,
      deflate_error_leaves_it_off, deflate_silence_leaves_it_off, replies_are_matched_by_id}` (scripted
      `TerminalHost`); the probe const inflates to 4 bytes — `detect::tests::deflate_query_payload_inflates`.
- [ ] Inside tmux: the new query and deflated chunks go through `Transport` wrapping.
      — `kitty::tests::deflated_chunks_wrap_in_tmux`, `detect::tests::deflate_query_wraps_in_tmux`.
- [ ] `answer`, `forget_uploads`, `free_all`, `hide` cover emphasis images. — `kitty::tests::
      {emphasis_enoent_retransmits, forget_covers_emphasis, free_all_frees_emphasis, hide_hides_emphasis}`.
- [ ] Step 0 table in NOTES with quotes and links; any cell still unconfirmed named as such.
- [ ] MAP.md (graft-term, graft-core): `Layer`, `EMPHASIS_ID_BIT`, `Place::layer`, `deflating`, `is_deflating`,
      `kitty_deflate`.

## Dependency
No deflate in std. Use a crate already in `Cargo.lock` (via `png`: `flate2`, `miniz_oxide`, `fdeflate`;
`Cargo.lock:2126-2136`, `:688-695`). Acceptance: the `Cargo.lock` diff adds only graft-term's edge to it, no new
package; say which crate and why. ARCHITECTURE.md (`:214-234`) rules internal edges only.

## Touches (report each on the **Touches:** line)
- Every `Place {` and `Capabilities {` literal (list above; compile-checked).
- `Capabilities::unprobed` (`detect/mod.rs:228`) and the probed build (`:190`): `kitty_deflate: false` / probed.
- iTerm2 path: untouched (it gets composites from the wiring ticket).
- First-paint budget (150 ms, ADR-0021): one more query in the same write; `heavy::budgets_hold` unchanged — say how
  checked (the probe deadline is unchanged; no new wait).
- Captures: none; Kitty bytes are not captured (raw streams never committed).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium (~55k).
