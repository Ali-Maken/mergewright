//! Held `j` over resident strips since lanes draw by role (ease-pass 13):
//! a `j` that keeps the focused branch sends no strip; one that moves the
//! focus onto another branch re-keys the strips whose lanes' roles changed.

use mergewright_core::capabilities::PixelProtocol;
use mergewright_graph::raster::Backdrop;

use super::{Transmit, branchy, drive, js, planned, repainted, states, transmits};

#[test]
fn j_inside_resident_strips_sends_no_strip() {
    for deflate in [false, true] {
        let (session, state) = branchy(PixelProtocol::Kitty, deflate);
        let events = js(20);
        let plans: Vec<_> = states(state.clone(), &events)
            .iter()
            .map(|state| planned(state).0)
            .collect();
        let run = drive(&session, state, events);

        let first = transmits(&run.frames[0]);
        assert!(
            first.iter().any(|t| !t.is_emphasis()),
            "deflate {deflate}: the first frame uploads its strips"
        );
        let mut kept_focus = 0;
        let mut emphasis = 0;
        for (k, frame) in run.frames.iter().enumerate().skip(1) {
            let sent: Vec<Transmit> = transmits(frame);
            let strips = sent.iter().filter(|t| !t.is_emphasis()).count();
            emphasis += sent.len() - strips;
            if plans[k] == plans[k - 1] {
                kept_focus += 1;
                assert_eq!(
                    strips, 0,
                    "deflate {deflate}: j {k} kept the focus and re-sent a strip"
                );
            }
        }
        eprintln!("deflate {deflate}: {kept_focus} of 20 j kept the focus");
        assert!(kept_focus >= 10, "premise: most j stay on one branch");
        assert!(
            repainted(&run.paints) >= 1,
            "premise: the thread moved over 20 j"
        );
        assert!(emphasis >= 1, "deflate {deflate}: its emphasis went");
        assert!(
            run.backdrops.iter().all(|b| *b == Backdrop::OffThread),
            "{:?}",
            run.backdrops
        );
    }
}
