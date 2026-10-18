//! The time machine: `Enter` on an older journal row previews the way
//! back, `Confirm` goes there in one step per entry, `Esc` goes back to
//! the browser.

use mergewright_core::journal::OperationKind;
use mergewright_fixtures::Shape;

use super::fixture::{Fixture, Remote, STAGED, three_to_stage};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{box_says, ch, chain, key, plain, redo, title_names};
use crate::repo_truth::check;

pub(super) fn time_travelling() -> Vec<Scenario> {
    vec![
        time_machine_three_steps(),
        time_machine_esc_returns_to_journal(),
    ]
}

/// Three stages, `J`, down to the oldest row, `Enter`: the preview. Shared
/// with `regressions::esc_cancels_undo_and_rolls_back`, which cancels the
/// three-step undo this box offers.
pub(super) fn preview_of_the_oldest() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        Step::Stage(STAGED[0]),
        Step::Stage(STAGED[1]),
        Step::Stage(STAGED[2]),
        Step::WaitFor(Wait::JournalHas(OperationKind::Stage)),
        Step::Mark("staged"),
        key(ch('J')),
        Step::WaitFor(Wait::Journal),
        key(ch('j')),
        key(ch('j')),
        key(mergewright_core::event::KeyCode::Enter),
        Step::WaitFor(Wait::PreviewReady),
        Step::See("the box is the time machine's", |state| {
            title_names(state, "revert to before \"stage one.txt\" (3 steps)")?;
            box_says(state, "restored from backup")
        }),
    ]
}

fn time_machine_three_steps() -> Scenario {
    Scenario {
        name: "time-machine-three-steps",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: three_to_stage,
        steps: chain(vec![
            preview_of_the_oldest(),
            vec![
                Step::Confirm,
                Step::Write("the three-step revert"),
                Step::WaitFor(Wait::Ready),
                Step::Mark("back"),
            ],
            redo(),
            redo(),
            redo(),
            vec![Step::Mark("forward")],
        ]),
        expect: three_steps_hold,
        capture: Some(Capture {
            stem: "m5-04-time-machine",
            headless: vec![
                Step::WaitFor(Wait::Ready),
                Step::Stage(STAGED[0]),
                Step::Stage(STAGED[1]),
                Step::Stage(STAGED[2]),
            ],
            keys: plain(&[ch('J'), ch('j'), ch('j'), mergewright_core::event::KeyCode::Enter]),
            on_screen: "3 steps",
            when: When::Settled,
        }),
    }
}

fn time_machine_esc_returns_to_journal() -> Scenario {
    Scenario {
        name: "time-machine-esc-returns-to-journal",
        shape: Shape::Trivial,
        remote: Remote::None,
        size: (80, 24),
        setup: three_to_stage,
        steps: chain(vec![
            preview_of_the_oldest(),
            vec![
                key(mergewright_core::event::KeyCode::Escape),
                Step::See("Esc is back in the journal browser", |state| {
                    check(state.preview.is_none() && state.journal.shown, || {
                        format!(
                            "preview open {}, journal shown {}",
                            state.preview.is_some(),
                            state.journal.shown
                        )
                    })
                }),
                Step::Mark("closed"),
            ],
        ]),
        expect: |_, marks| {
            let staged = marks.get("staged")?;
            let closed = marks.get("closed")?;
            check(
                closed.index_tree.is_some() && closed.index_tree == staged.index_tree,
                || "Esc changed the index".to_owned(),
            )?;
            check(closed.journal.len() == 3, || {
                format!("{} entries after Esc", closed.journal.len())
            })
        },
        capture: None,
    }
}

/// Three stages, three steps back: the index tree is the pre-stage one,
/// three `Undo` entries, and three `U` bring it forward again.
fn three_steps_hold(_fixture: &Fixture, marks: &Marks) -> Result<(), String> {
    let staged = marks.get("staged")?;
    check(staged.journal.len() == 3, || {
        format!("{} entries after three stages", staged.journal.len())
    })?;
    let back = marks.get("back")?;
    check(
        back.index_tree.is_some() && back.index_tree == marks.before.index_tree,
        || {
            format!(
                "the index is {:?}, not the pre-stage {:?}",
                back.index_tree, marks.before.index_tree
            )
        },
    )?;
    check(back.repo.files == marks.before.repo.files, || {
        "the time machine changed the files".to_owned()
    })?;
    let undos = back
        .journal
        .iter()
        .filter(|entry| entry.kind == OperationKind::Undo)
        .count();
    check(undos == 3, || format!("{undos} Undo entries, not three"))?;
    let forward = marks.get("forward")?;
    check(
        forward.index_tree.is_some() && forward.index_tree == staged.index_tree,
        || "three U did not bring the index back".to_owned(),
    )
}
