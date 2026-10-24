# fix round: pixels fail, graph goes blank — fall back to the Unicode column

Status: ready
Blocked by: none
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the decision is made below and reuses `F3`'s path whole (`renderer_override`), so the new code is one field, one event, one take and one notice; each part is judged by a byte-stream or state test, not by the diff or an eye. No Fable test met: no new interface across crates, nothing structural.
Needs: none.

From KNOWN_GAPS `ease/29-gfx-failed-blank-graph` (broken, by code), scout `.scratch/ease-pass/scouts/29r-gfx-failed.md`.

## Need
Not hit by him yet; found by code reading (gap row: "Not run in a terminal"). He uses the pixel tier in Ghostty. If the terminal refuses an image, or a strip will not rasterise, the header reads `gfx: <msg>` and the graph column is empty: no lanes, no nodes, no hint that `F3` exists. The shape of history (M1's promise) is gone and his only way back is a key nothing on screen names. PROMPT.md §7.1 asks for "a correct pixel graph *or* a clean fallback, never a broken screen".

## Evidence (rule 7; read at the lines this pass)
- Failure set at `event_loop/graphics.rs:150` (any Kitty error reply), `:197` (iTerm2 `bands.frame` Err), `:252` (strip `layers` Err in `drawn`), `:289` (prefetch).
- `drain` starts every batch at `Ok` (`graphics.rs:146`) and stores it (`:154`), so the Kitty status self-clears on the next drain with no error, and a drain runs on every wake (`running/turn.rs:244`) and every fence answer (`:270`).
- **Local failures never reach the header, by code.** The view reads the status (`running/frame.rs:102`) *before* `graphics.frame` sets `:197`/`:252` (`running/frame.rs:106`); the next wake's drain resets it to `Ok` before the next view. So a strip that will not rasterise draws blank under `gfx ok`. Not run; the raster box below is red-first and will show it.
- `draws_pixels` (`view/frame.rs:129-131`) reads session and state only; `view_with_graphics` takes the status (`frame.rs:175`) but uses it only for the header (`:269`). The column skips text at the pixel tier (`mergewright-ui/src/graph_screen/render.rs:151`; the gap row says :149, the condition is :151).
- Cured replies exist: `KittyImages::answer` re-sends a missing image and turns deflate off after an error on a deflated upload (term MAP § kitty; `GraphicsReply::is_missing_image`, `KittyImages::is_deflating`). The ENOENT path re-sends and recovers (`event_loop/tests.rs:532-545`).
- `F3` already does the whole drop: `renderer_override` (`update/state.rs:143-147`) feeds `tier_in_force` (`view/frame.rs:86-88`), which hides the `gfx` field (`:112-119`), draws the column as text, frees Kitty uploads (`graphics.rs:171-181`), and puts `renderer: unicode (F3 cycles)` in the footer (`view/frame/notices.rs:83-85`).
- Design: `docs/design/04-terminal-capability-detection.md:77` covers a forced renderer (config wins, one-key escape); silent on any failure after the tier is chosen. §4.3 (`:107-108`): the tier is "decided once at startup, not a per-frame guess".

## Decision
**D1. A failure the pixel tier cannot cure drops the graph to Unicode for the rest of the run, as `F3` would.** Same `renderer_override = Some(Unicode)`; nothing new decides the tier, so the column, header, hit-tests, uploads and boxes all follow the path `F3` already tests.
- **Uncured** (drops): a Kitty error reply that is neither `is_missing_image()` nor the one that turned deflating off (`is_deflating()` true before `answer`, false after); an iTerm2 `bands.frame` Err; a strip that fails to rasterise in `drawn`.
- **Cured** (keeps pixels; header as today, `gfx: <msg>` until the next error-free drain): ENOENT re-sent, deflate turned off. If the cure itself fails, the next reply is uncured and drops.
- **Prefetch failure (`:289`) drops nothing and shows nothing.** The strip stays uncached; `drawn` rasterises it again before it is ever on screen, and that failure drops. Dropping on an off-screen strip would change the graph's look while he is idle, with nothing pressed. Replace the assignment with a comment saying exactly that (rule 3).
- **First failure wins** within one take: later ones in the same frame are not shown; comment why (they are its consequences or the same kind, and the tier is gone either way).

**D2. Detected and forced alike.** `graph.renderer = "pixel"` chooses what is *attempted*; it does not ask for a blank graph. The forced-but-silent case (no reply at all) still needs `F3` — nothing reported, nothing to act on.

**D3. No automatic return to pixels.** He goes back with `F3` (Unicode → ASCII → pixels, the existing cycle). A fresh failure drops again; each retry is his key, so nothing loops.
- Rejected: **follow the status** (text while `Failed`, pixels when it clears). Text draws no images, so nothing replies, the next drain reads `Ok`, pixels return, fail, drop: the column swaps every frame at rest, and every swap owes a frame (ADR-0065's at-rest rule as the conductor states it: no self-sustaining wake chain). On the cured ENOENT path it would flash text for one frame each time.
- Rejected: **a timed retry**: a timer wake at rest, against the same rule.
- Rejected: **text only for the failed strip's rows**: two pictures in one column, the reason ADR-0060 drops the whole column under a preview.
- Rejected: **a count of failures before dropping**: the blank column stays up while it counts, for errors that do not cure themselves.

**D4. Words: the footer says why; the header says nothing.** Once dropped, the `gfx` field is gone (tier in force is not pixel), so no error stays in the header for a tier that is not drawing. The renderer notice gains the reason, exactly:
`renderer: unicode · pixels failed: <msg> (F3 cycles)` (and `ascii` after one more `F3`).
Fitted to the footer's text width as the interrupted notice is (`columns − 1 − RIGHT_MARGIN`, `notices.rs:70-72`): `<msg>` is cut with `…`; `renderer: <tier> · pixels failed: ` and ` (F3 cycles)` stay whole; under ~40 cells the reason goes and the line is today's. A notice, not a `⊗` failure: it must not clobber a running write's `Busy` line, and the graph *is* drawn correctly now. The reason is cleared when `F3` cycles back to the top, with the override.

**D5. Alignment holds.** The fallback column is the Unicode column ticket 29 D5 already aligned with the pixel nodes, so nodes do not move sideways on the drop. Asserted below.

**Doc edit (part of this ticket).** `docs/design/04-terminal-capability-detection.md`:
- Row `:77`, replace the cell with: "Whatever it says — always wins over detection. Forcing `pixel` on a terminal that didn't ack the query still attempts it, using the reachable protocol above. `F3` drops a tier if the output looks garbled; a failure the terminal or the rasteriser reports drops it on its own (below)."
- New paragraph after the tier table, before "iTerm2 and Sixel move above Unicode…": "**When pixels fail.** The tier is chosen once, but a pixel tier can fail while it runs: the terminal answers an image command with an error, or a strip will not rasterise or encode. Then the graph drops to the Unicode tier for the rest of the run, as `F3` would, whether the pixel tier was detected or forced — config picks what is attempted, not a blank graph. The footer says so: `renderer: unicode · pixels failed: <reason> (F3 cycles)`; `F3` cycles back to pixels, and a fresh failure drops again. Not a failure: an error Mergewright cures itself (a missing image sent again, a deflated upload sent plain), which keeps pixels and shows in the header's `gfx` field until the next error-free reply. No timed retry: nothing wakes the loop at rest to try pixels again, and a column that swapped back and forth would read as the graph changing."

## Journeys
- Pixels fail mid-session. Today: header `gfx: <msg>`, column blank; he must already know `F3` (1 step if he does, else he leaves the app). After: **0 steps**; the next frame draws the graph as text, the footer says why and names `F3`.
- Try pixels again: `F3`, `F3` = **2 steps** (existing cycle). Lands on pixels; if they fail again, back on text with the same footer.

## Scope
In: crate `mergewright` only: `event_loop/graphics.rs` (failure field, `drain` classification, `:197`/`:252` record, `:289` comment), `event_loop/running/turn.rs` (+ any other `drain`/`draw` caller: grep `\.drain()` and `draw_frame` across `event_loop/`), `update` (event, field, handler beside `F3`'s, `F3` clearing the reason), `view/frame/notices.rs`, tests, shot test. The doc edit above. MAP lines.
Out: `mergewright-ui` (`GraphicsStatus`, `render.rs` unchanged), `mergewright-term` (no signature change; `answer`, `is_deflating`, `is_missing_image` are enough), the cured-reply header words (Question below), `F3`'s cycle, Sixel, the 3-column no-gutter rule (ticket 29 D5.6).

## Signatures
```rust
// crates/mergewright/src/event_loop/graphics.rs
pub struct Graphics<R> { /* … */ failure: Option<SanitizedText>, }
impl<R: GraphicsReplies> Graphics<R> {
    /// The first failure since the last take that the pixel tier cannot cure
    /// itself (an uncured Kitty reply, an iTerm2 band error, a strip that
    /// would not rasterise on screen); `None` when there is none. The loop
    /// hands it to `update` as `AppEvent::PixelsFailed` (ticket 30 D1).
    pub fn take_failure(&mut self) -> Option<SanitizedText>;
}

// crates/mergewright/src/update/events.rs
pub enum AppEvent { /* … */
    /// The pixel tier failed in a way it cannot cure: draw the graph as text.
    PixelsFailed(SanitizedText),
}

// crates/mergewright/src/update/state.rs (AppState)
/// Why the pixel tier gave way to text, while it has: set with
/// `renderer_override` by `AppEvent::PixelsFailed`, cleared when `F3`
/// cycles back to the top (ticket 30 D4).
pub pixels_failed: Option<SanitizedText>,

// beside F3's tier-drop handler (both writers of `renderer_override` in one file)
/// At the pixel tier in force: override to Unicode and keep `why`. Off it
/// (a late reply after `F3`): nothing to drop from; no change, commented.
fn fall_back_from_pixels(state: &mut AppState, why: SanitizedText);
```
The loop: after every `graphics.drain()` and every drawn frame, `take_failure()`; `Some` → `update(AppEvent::PixelsFailed(why))` and a frame owed. If `AppState`'s derives refuse `SanitizedText`, carry `String` and sanitise in the view; say which in NOTES.

## Acceptance (tick each with its test, inline; red first where marked)
app, fast tier:
- [ ] **Red first: an uncured Kitty reply draws the graph as text.** `pixel_run` with reply `\x1b_Gi=1000;EINVAL:bad\x1b\\`: the frame after the reply has Unicode node glyphs in the graph column, `renderer: unicode · pixels failed: EINVAL:bad (F3 cycles)` on the status row, no `gfx` in the header, `a=d` deletes for every upload sent; no later frame transmits or places. Red on main: header `gfx: EINVAL:bad`, column blank. <- `event_loop::tests::an_uncured_kitty_reply_draws_the_graph_as_text`
- [ ] **Cured replies keep pixels.** ENOENT: today's assertions plus no `pixels failed`, no glyph column, strips still placed. An error while deflating: deflate off, strips re-sent plain, no drop. <- `event_loop::tests::an_enoent_reply_re_sends_the_strip_and_says_so` (extended), `a_deflate_error_keeps_the_pixel_tier`
- [ ] **Red first: a strip that will not rasterise drops the column** (Kitty and iTerm2), and the drop frame follows at once. Red on main: blank column under `gfx ok` (Evidence, third bullet). Seam: a real input that makes the raster or PNG encode fail if one exists (look in `raster`'s error cases); else a `#[cfg(test)]` fault on the strip cache / `ItermImages`, named in MAP. No fault switch in release code. <- `event_loop::tests::a_strip_that_will_not_rasterise_draws_the_graph_as_text`
- [ ] **An iTerm2 band error drops the column.** <- `event_loop::tests::an_iterm2_band_error_draws_the_graph_as_text`
- [ ] **A prefetch failure changes nothing on screen**: no drop, no status, no frame owed; the strip is not cached. <- `event_loop::graphics` unit test `a_prefetch_failure_drops_nothing`
- [ ] **Forced config falls back too.** Session with `graph.renderer = "pixel"`, uncured reply → same frame as box 1. <- `event_loop::tests::a_forced_pixel_renderer_falls_back_too`
- [ ] **No chain at rest.** After the drop frame (and at most one more for the status settling), no further frame is drawn with no input, through two fence answers. <- `event_loop::tests::a_fallback_settles_without_waking_again`
- [ ] **State rule.** `PixelsFailed` at the pixel tier sets override `Unicode` and the reason, no commands; a second one changes nothing; at Unicode or ASCII (after `F3`) it changes nothing; `F3` from the fallback goes to ASCII keeping the reason, `F3` again clears both. <- `update::tests::pixels_failed_drops_to_unicode_once` (where F3's tests live)
- [ ] **Words.** Exact line at 100 cols; at 80 cols with a 120-char message the message is cut with `…` and `(F3 cycles)` is whole; at 30 cols today's `renderer: unicode (F3 cycles)`; a `Busy` status still wins the row. <- `view::frame::notices` tests `the_renderer_line_says_why_pixels_failed`, `…_fits_the_row`
- [ ] **Nodes do not move on the drop.** For the 100×30 fixture of ticket 29, the pixel node centre before the failure equals the Unicode glyph cell centre after it, every cell width there. <- `view::tests::pixel_preview::lead::fallback_nodes_sit_where_the_pixel_nodes_were`
- [ ] **iTerm2: the drop frame's text survives.** On iTerm2 the `None` frame calls `bands.hide` (`graphics.rs:180`), which writes `ECH` over the cells the last frame's bands covered (`mergewright-term/src/iterm2.rs:114-120`) *after* the text (`running/frame.rs:102` then `:106`). Assert the graph column's glyphs are on the terminal after the drop frame and the next (a screen model fed the bytes, or the byte order). **If red, stop and report** with the lines: it is `F3`'s path too and changes the frame's byte order (DESIGN §4.7), so the conductor files it apart. <- `event_loop::tests::an_iterm2_drop_keeps_the_text_column`

walk tier: `scripts/walks.sh` green; no existing `docs/shots/*.txt` changes bytes.

- [ ] **For his eye:** `docs/shots/ease-pass/30-gfx-failed-{before,after}.txt` (≤ 200 KB each), 100×30, Kitty, uncured reply. "Before" taken on main by the same test: header `gfx: EINVAL:bad`, graph column empty. "After": the graph drawn as text from the first column (lanes, nodes, badges as at the Unicode tier), no `gfx` field in the header, status row `renderer: unicode · pixels failed: EINVAL:bad (F3 cycles)`. Field check, unverified until he does it: none practical (a real terminal rarely refuses); stays unverified. Colour unverified (rule 5). <- `ease_30_shots::capture_gfx_failed_fallback`

## Read first
- This ticket; scout `.scratch/ease-pass/scouts/29r-gfx-failed.md`.
- `crates/mergewright/MAP.md` § event_loop, event_loop::graphics, event_loop::running, update::state, view::frame; `crates/mergewright-term/MAP.md` § kitty, iterm2.
- `docs/design/04-terminal-capability-detection.md:73-87`.
- Source: `event_loop/graphics.rs:100-293`, `event_loop/running/turn.rs:233-274` and `Run::draw`, `event_loop/running/frame.rs`, `view/frame.rs:79-131`, `view/frame/notices.rs`, `F3`'s handler (grep `renderer_override` under `update/`), `event_loop/tests.rs:505-560` (`pixel_run`, ENOENT, `f3_round_trip`).

## Touches (check each, say how)
- **Every drain and draw caller** (`pump`, test and pixel forms, headless): each takes the failure, or NOTES says why it cannot have one (grep, file:line).
- **`renderer_override`'s readers**: hit-tests, `row_columns`, `max_lanes`/row padding (`settled_tier`), prefetch (must do nothing off the pixel tier: file:line), the status screen's tier line (what it says after a drop; report, no change unless it claims pixel).
- **Kitty uploads after the drop**: freed by the `None` frame (`graphics.rs:178`); none placed later. `free_all` on exit still runs with nothing held.
- **Header `gfx` field**: gone after the drop; `gfx ok`/`gfx: <msg>` unchanged otherwise (CLAUDE status rule). Tests at `graph_screen/tests.rs:897-899`, `tests/problem_line.rs:59-70` still pass.
- **Ticket 29's box** `preview_nodes_sit_where_the_pixel_nodes_were` says "a `Failed` status draws no text column at all": update that NOTE's wording in the test comment if it repeats it.
- **Keymap**: no new key; `F3` as in `docs/design/02-keymap.md`. Journal/undo: none. Other crates: none (`git diff --stat`).
- **Captures and sheet**: the two new files; the ease-pass contact sheet (ADR-0037) gains them, conductor assembles.
- **Docs**: the 04 edit above; app MAP `event_loop::graphics` line gains "a failure it cannot cure is taken by the loop (`take_failure`) and drops the graph to Unicode as `F3` would (ticket 30)"; `update::state` line for `pixels_failed`.

## Question (NOTES only, not to fix)
A cured reply still shows `gfx: ENOENT:…` until the next error-free drain, though nothing is wrong (CLAUDE status rule). Suppressing it needs `drain` to owe the re-send frame some other way, or the re-send waits for the next wake; and a terminal that evicts every frame would then loop at rest. Record what you see; the conductor decides.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`; then `scripts/walks.sh` (this changes what the app draws).

## Size
~45k tokens: one field and take, one classification in `drain`, three record sites, one event and handler, one notice, eleven tests, two captures, one doc paragraph.

## Planner's note to the conductor
rg was down: Glob and Grep failed. Read by hand at the lines cited in Evidence. **Not read:** ADR-0065 (file name unknown without a listing); its rule is cited as your dispatch states it ("at-rest wake rule"), and app MAP's ADR-0065 lines ("one answer, no chain") agree. **Not verified:** that local raster/iTerm2 failures never show in the header (by code order only; box 3 is red-first to show it); the iTerm2 erase-after-text blank (box 11 decides, stop if red); whether a real failing raster input exists (box 3's seam).
