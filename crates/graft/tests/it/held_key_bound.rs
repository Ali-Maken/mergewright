// The after-release bound of `held_key` (m7-power-tools/42 §Planner
// correction). Pure, so it compiles in the fast tier: `held_key.rs` is
// heavy-gated and `include!`s this file; `main.rs` mounts it alone as
// `held_key` otherwise. Test path is `held_key::…` either way.

/// Where `tail_ms` ends.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TailEnd {
    /// The drain passing this frame's last byte.
    Frame(usize),
    /// Scenario B with no frame counted: `J`'s frame's first byte.
    JsFirstByte,
    /// Scenario A: the last frame's last byte.
    LastByte,
}

/// Frames counted after release, and where the tail ends.
///
/// `drawn[k]` is frame `k`'s draw time (`None`: not a frame). A counts every
/// frame drawn after `last_j`. B (`js_frame` is `J`'s frame index) counts only
/// those before `J`'s frame: its frame is judged by `j_ms`, not here.
pub(crate) fn after_release<T: Ord + Copy>(
    drawn: &[Option<T>],
    last_j: T,
    js_frame: Option<usize>,
) -> (usize, TailEnd) {
    let bound = js_frame.unwrap_or(drawn.len());
    let counted: Vec<usize> = (0..bound.min(drawn.len()))
        .filter(|&k| drawn[k].is_some_and(|at| at > last_j))
        .collect();
    let end = match (js_frame, counted.last()) {
        (None, _) => TailEnd::LastByte,
        (Some(_), Some(&k)) => TailEnd::Frame(k),
        (Some(_), None) => TailEnd::JsFirstByte,
    };
    (counted.len(), end)
}

#[test]
fn b_bound_stops_at_js_frame() {
    // Held frames at 1, 2; last j at 2; two j frames after; J's frame at 4.
    let drawn = [Some(1), Some(2), None, Some(3), Some(4), Some(5), Some(6)];
    assert_eq!(after_release(&drawn, 2, Some(5)), (2, TailEnd::Frame(4)));
    // None after release before J: the tail ends at J's first byte.
    let drawn = [Some(1), Some(2), Some(5)];
    assert_eq!(after_release(&drawn, 2, Some(2)), (0, TailEnd::JsFirstByte));
    // A: unchanged, every frame after the last j, to the last byte.
    let drawn = [Some(1), Some(2), Some(3), Some(4)];
    assert_eq!(after_release(&drawn, 2, None), (2, TailEnd::LastByte));
}

/// Where the loop's own rest begins after release (G-settle): the index of
/// the first `Rest` handed after `last_j`. `handed[k]` is (is the loop's
/// `Rest`, when it was handed).
///
/// Found by when it was handed, not by a draw stamp: the rest's commands
/// start the pane's reads, their `Started` answers join the rest's turn,
/// and the turn's one frame is stamped on its last wake. Looking for a
/// drawn `Rest` missed it there and counted the pane's catch-up as the
/// graph's tail.
pub(crate) fn rest_index<T: Ord + Copy>(handed: &[(bool, T)], last_j: T) -> Option<usize> {
    handed.iter().position(|&(rest, at)| rest && at > last_j)
}

#[test]
fn the_rest_is_found_when_job_answers_join_its_frame() {
    // Measured (pane shown, Pixel deflate, release): last j at 0; its frame
    // drawn at 28. The loop rests at 60, its commands start the pane's
    // reads (`Started::ListFiles` at 61, `Started::ContainedIn` at 61), and
    // both join the rest's turn: one frame, drawn at 62, stamped on the
    // turn's last wake. The rest begins at index 2, not nowhere.
    let handed = [
        (false, -10),
        (false, 10),
        (true, 60),
        (false, 61),
        (false, 61),
    ];
    let drawn = [Some(10), Some(28), None, None, Some(62)];
    let rest = rest_index(&handed, 0);
    assert_eq!(rest, Some(2));
    // Graph frames end where the rest begins: two after release, not three.
    assert_eq!(
        after_release(&drawn[..rest.unwrap_or(drawn.len())], 0, None).0,
        2
    );
    // A rest drawn alone is found as before.
    assert_eq!(rest_index(&[(false, 10), (true, 60)], 0), Some(1));
    // A rest handed before the last j is not the rest after release.
    assert_eq!(rest_index(&[(true, -5), (false, 0)], 0), None);
}
