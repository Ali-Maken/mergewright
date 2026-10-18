//! The whole-merge rows that end at the review's `y` (ease-pass/05b), moved
//! out of `conflict_journeys.rs` to keep that file under its size row.
//! Fixtures and step helpers come from the parent.

use super::{
    KeyCode, Scenario, Step, Wait, both_files_done, ch, conflicting, done_by, end, journey, key,
    launched, m_m, merged_landing, on_feature, review_then_commit, route, two_files,
    two_files_in_the_shell,
};

// ───────────────────────────── whole rows ─────────────────────────────

pub(super) fn a_whole_one_file() -> Scenario {
    journey(
        "a-whole-one-file",
        conflicting,
        vec![
            on_feature(),
            vec![route()],
            m_m(),
            done_by('O', "shared.txt"),
            review_then_commit(),
            vec![end()],
            merged_landing(),
        ],
    )
}

/// J13 (amendment 2026-09-24): `gone.txt` done, the cursor on `notes.txt`'s
/// last open block — `o` solves the last clash, `y` opens the review and
/// its `y` commits. No `⏎`: a merge stages at the review's `y`
/// (ease-pass/05b; the stage and the summary opening on its own were
/// m6-conflicts/60's).
pub(super) fn last_block_to_finished() -> Scenario {
    journey(
        "last-block-to-finished",
        two_files,
        vec![
            on_feature(),
            m_m(),
            done_by('o', "gone.txt"),
            // `notes.txt`'s first block taken: the cursor moves on to the
            // last one (m6-conflicts/15).
            vec![key(ch('o')), Step::WaitFor(Wait::ConflictOpened)],
            vec![
                route(),
                key(ch('o')),
                Step::WaitFor(Wait::ConflictOpened),
                Step::WaitFor(Wait::ResolvedPath("notes.txt")),
            ],
            review_then_commit(),
            vec![end()],
            merged_landing(),
        ],
    )
}

pub(super) fn a_whole_two_files() -> Scenario {
    journey(
        "a-whole-two-files",
        two_files,
        vec![
            on_feature(),
            vec![route()],
            m_m(),
            both_files_done(),
            review_then_commit(),
            vec![end()],
            merged_landing(),
        ],
    )
}

pub(super) fn b_whole_two_files() -> Scenario {
    journey(
        "b-whole-two-files",
        two_files_in_the_shell,
        vec![
            launched(),
            vec![
                route(),
                key(KeyCode::Enter),
                Step::WaitFor(Wait::ConflictOpened),
            ],
            both_files_done(),
            review_then_commit(),
            vec![end()],
            merged_landing(),
        ],
    )
}
