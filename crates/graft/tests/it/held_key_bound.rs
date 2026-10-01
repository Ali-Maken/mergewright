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
