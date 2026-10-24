//! ease-pass/30: a pixel failure the tier cannot cure draws the graph as
//! text; one it cures keeps pixels.

use mergewright_core::capabilities::PixelProtocol;

use super::*;

mod ease_30_shots;

const UNCURED: &[u8] = b"\x1b_Gi=1000;EINVAL:bad\x1b\\";
const FALLBACK_NOTICE: &str = "renderer: unicode \u{b7} pixels failed: EINVAL:bad (F3 cycles)";

#[test]
fn an_enoent_reply_re_sends_the_strip_and_says_so() {
    let (frames, _tail) = pixel_run(
        vec![unbound_key(), quit_key()],
        vec![b"\x1b_Gi=1000;ENOENT:image not found\x1b\\".to_vec()],
    );

    assert_eq!(transmitted(&frames[0]).first(), Some(&1000));
    assert!(!transmitted(&frames[1]).is_empty(), "{:?}", frames[1]);
    assert!(
        frames[1].contains("gfx: ENOENT:image not found"),
        "{:?}",
        frames[1]
    );
    // Cured: pixels stay, nothing drops.
    assert!(!frames[1].contains("pixels failed"), "{:?}", frames[1]);
    assert!(!frames[1].contains('\u{25cf}'), "no glyph column");
    assert!(frames[1].contains("a=p,i="), "strips still placed");
}

#[test]
fn an_uncured_kitty_reply_draws_the_graph_as_text() {
    let (frames, tail) = pixel_run(
        vec![unbound_key(), unbound_key(), quit_key()],
        vec![UNCURED.to_vec()],
    );
    let sent = transmitted(&frames[0]);
    assert!(!sent.is_empty(), "premise: the first frame uploads");
    let after = &frames[1];
    assert!(after.contains('\u{25cf}'), "node glyphs: {after:?}");
    assert!(after.contains(FALLBACK_NOTICE), "{after:?}");
    assert!(!after.contains("gfx"), "no gfx in the header: {after:?}");
    for id in &sent {
        assert!(after.contains(&format!("a=d,d=I,i={id},")), "{id} freed");
    }
    for frame in &frames[2..] {
        assert!(transmitted(frame).is_empty(), "no later transmit");
        assert!(!frame.contains("a=p,i="), "no later place");
    }
    assert!(!tail.contains("pixels failed"));
}

/// [`pixel_run`] on a session and state the test shaped.
fn run_in(
    (mut session, state): (Session, AppState),
    wakes: Vec<Wake>,
    replies: Vec<Vec<u8>>,
) -> (Vec<String>, String) {
    let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let mut graphics = Graphics::new(ScriptedReplies(replies.into()), &session.capabilities);
    let exit = pump_with(
        Screen {
            terminal: &mut terminal,
            graphics: &mut graphics,
        },
        &mut ScriptedWakes::spaced(wakes),
        &Recorded::default(),
        &mut session,
        state,
        Outside::default(),
        &mut crate::trace::Tracer::off(),
    )
    .expect("writing to memory cannot fail");
    assert_eq!(exit, Exit::Quit);
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let mut frames: Vec<String> = recorder.written().split(&end).map(str::to_owned).collect();
    let tail = frames.pop().expect("split yields at least one piece");
    (frames, tail)
}

fn with_protocol(protocol: PixelProtocol) -> (Session, AppState) {
    let (mut session, state) = at_pixel_tier();
    session.capabilities.pixel_protocol = Some(protocol);
    (session, state)
}

#[test]
fn a_deflate_error_keeps_the_pixel_tier() {
    let (mut session, state) = at_pixel_tier();
    session.capabilities.kitty_deflate = true;
    let (frames, _tail) = run_in(
        (session, state),
        vec![unbound_key(), quit_key()],
        vec![UNCURED.to_vec()],
    );
    assert!(
        frames[0].contains("o=z"),
        "premise: deflated: {:?}",
        frames[0]
    );
    let after = &frames[1];
    assert!(!transmitted(after).is_empty(), "re-sent: {after:?}");
    assert!(!after.contains("o=z"), "plain this time: {after:?}");
    assert!(!after.contains("pixels failed"), "no drop: {after:?}");
    assert!(after.contains("a=p,i="), "strips still placed");
    assert!(after.contains("gfx: EINVAL:bad"), "{after:?}");
}

/// A cell height no strip's pixel buffer can be sized for: the rasteriser
/// says `TooLarge`, the way a terminal reporting an absurd cell would.
fn absurd_cell((mut session, state): (Session, AppState)) -> (Session, AppState) {
    session.rendering.cell_size.height = 3_000_000_000;
    (session, state)
}

#[test]
fn a_strip_that_will_not_rasterise_draws_the_graph_as_text() {
    for protocol in [PixelProtocol::Kitty, PixelProtocol::Iterm2] {
        let (frames, tail) = run_in(
            absurd_cell(with_protocol(protocol)),
            vec![unbound_key(), unbound_key(), quit_key()],
            vec![],
        );
        // Frame 0 is what the failure was found in; the next shows it.
        let after = &frames[1];
        assert!(
            after.contains('\u{25cf}'),
            "{protocol:?}: glyphs: {after:?}"
        );
        assert!(
            after.contains("pixels failed: a strip of"),
            "{protocol:?}: {after:?}"
        );
        assert!(!after.contains("gfx"), "{protocol:?}: {after:?}");
        for frame in &frames[1..] {
            assert!(!frame.contains("a=p,i="), "{protocol:?}: no Kitty place");
            assert!(!frame.contains("1337;File="), "{protocol:?}: no band");
        }
        assert!(!tail.contains("pixels failed"));
    }
}

#[test]
fn an_iterm2_band_error_draws_the_graph_as_text() {
    // A zero-width cell rasterises to an empty frame, which the PNG
    // encoder refuses: `ItermImages::frame` returns the error.
    let (mut session, state) = with_protocol(PixelProtocol::Iterm2);
    session.rendering.cell_size.width = 0;
    let (frames, _tail) = run_in((session, state), vec![unbound_key(), quit_key()], vec![]);
    let after = &frames[1];
    assert!(after.contains('\u{25cf}'), "glyphs: {after:?}");
    assert!(after.contains("pixels failed: "), "{after:?}");
    assert!(!after.contains("gfx"), "{after:?}");
    assert!(!after.contains("1337;File="), "no band: {after:?}");
}

#[test]
fn a_forced_pixel_renderer_falls_back_too() {
    let (mut session, state) = at_pixel_tier();
    session.rendering.tier_source = mergewright_ui::status_screen::TierSource::Forced;
    let (frames, _tail) = run_in(
        (session, state),
        vec![unbound_key(), unbound_key(), quit_key()],
        vec![UNCURED.to_vec()],
    );
    assert!(frames[1].contains('\u{25cf}'), "{:?}", frames[1]);
    assert!(frames[1].contains(FALLBACK_NOTICE), "{:?}", frames[1]);
}

#[test]
fn a_fallback_settles_without_waking_again() {
    let (frames, _tail) = run_in(
        at_pixel_tier(),
        vec![unbound_key(), Wake::Fence, Wake::Fence, quit_key()],
        vec![UNCURED.to_vec()],
    );
    // The first frame, the drop frame, and at most one for the status
    // settling when the next drain reads `Ok`; nothing after two fences.
    assert!(frames.len() <= 3, "{} frames: {frames:?}", frames.len());
    assert!(frames[1].contains(FALLBACK_NOTICE));
}
