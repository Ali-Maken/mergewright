//! The pinned uncommitted row is an image of its own (ease-pass/20): scrolling
//! the viewport re-sends no strip, and the row is uploaded once and placed by
//! id after. 46c's bound, with a dirty working tree and `HEAD` scrolled off.

use std::sync::Arc;

use mergewright_core::capabilities::PixelProtocol;

use super::*;
use crate::view::tests::{counts, oid, refs_with_head};

/// `history_of(100)` (one branch: the lanes' roles never change) with a
/// dirty tree and `HEAD` on row 0, the viewport scrolled to `top`.
fn scrolled_dirty(state: &AppState, top: usize) -> AppState {
    let mut state = state.clone();
    state.refs = Some(Arc::new(refs_with_head(Some(oid(100)))));
    state.working_tree = Some(counts(1, 0, 0));
    let history = state.history.as_mut().expect("history");
    history.top = top;
    history.cursor = top;
    state
}

/// Placements (`a=p`) in one frame's bytes.
fn placements(frame: &str) -> usize {
    frame.split("\x1b_G").filter(|c| c.starts_with("a=p")).count()
}

#[test]
fn a_pinned_uncommitted_row_resends_no_strip_while_scrolling() {
    for deflate in [false, true] {
        let (mut session, state) = at_pixel_tier();
        session.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);
        session.capabilities.kitty_deflate = deflate;
        // Viewport steps 18..=26 on the 18-row pane: strips 0..32 and 32..64
        // stay on screen while it moves a row at a time under the pinned row.
        let first = scrolled_dirty(&state, 18);
        assert!(first.uncommitted_row_pinned() && !first.uncommitted_row_attached());
        let steps: Vec<AppState> = (19..=26).map(|top| scrolled_dirty(&state, top)).collect();
        let (mut terminal, recorder) = terminal(first.size.columns, first.size.rows);
        let mut graphics = Graphics::new(NoReplies, &session.capabilities);
        draw(&mut terminal, &session, &first, &mut graphics).expect("memory");
        for state in &steps {
            draw(&mut terminal, &session, state, &mut graphics).expect("memory");
        }
        let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
        let mut run: Vec<String> = recorder.written().split(&end).map(str::to_owned).collect();
        run.pop();

        let strips = |frame: &String| -> usize {
            transmits(frame).iter().filter(|t: &&Transmit| !t.is_emphasis()).count()
        };
        // The first frame sends every strip on screen and the pinned row.
        assert!(strips(&run[0]) >= 2, "deflate {deflate}: first frame {}", strips(&run[0]));
        for (k, frame) in run.iter().enumerate().skip(1) {
            assert_eq!(
                strips(frame),
                0,
                "deflate {deflate}: scroll step {k} re-sent a strip under a pinned row"
            );
            assert!(
                placements(frame) >= strips(&run[0]),
                "deflate {deflate}: step {k} placed fewer images than the first frame: the pinned row is placed by id"
            );
        }
    }
}
