//! The streaming layout replays a rename at a branch tip (ADR-0064).

use super::tests::{VecSource, advance_all};
use super::*;
use crate::layout::layout;

/// ADR-0064: the rename at a branch tip is part of the replayed state, so a
/// checkpoint cut between the upper tip's rows and the lower tip changes
/// nothing.
#[test]
fn a_rename_at_a_tip_replays_the_same_across_a_checkpoint() {
    use crate::layout::tests::stacked::{stack_commits, upper_lower_seeds};
    let commits = stack_commits();
    let whole = layout(&commits, upper_lower_seeds());
    let config = StreamConfig {
        chunk_rows: 2,
        checkpoint_every: 2,
        max_checkpoints: 8,
        resident_chunks: 1,
        buffer_rows: 0,
    };
    let mut streaming = StreamingLayout::new(
        VecSource::new(commits.to_vec()),
        upper_lower_seeds(),
        config,
    );
    streaming.set_viewport(0..5);
    advance_all(&mut streaming);
    // Look at the tail first so the head is evicted, then replay it.
    streaming.set_viewport(3..5);
    let _ = streaming.rows(3..5).expect("an in-memory source");
    streaming.set_viewport(0..5);
    let rows = streaming.rows(0..5).expect("an in-memory source");
    assert_eq!(rows, whole);
}
