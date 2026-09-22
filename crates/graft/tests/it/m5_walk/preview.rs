//! The preview itself: it opens with ghosts and the plan's commands, runs
//! nothing until `Confirm`, refuses when the picture went stale, and never
//! confirms from a screen too small to draw it.

use graft_core::event::KeyCode;
use graft_fixtures::Shape;

use super::fixture::{Fixture, forked};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::Marks;
use super::vocabulary::{
    box_says, ch, ghosts_drawn, key, nothing_open, plain, same_files, same_refs, status_containing,
    title_names,
};

pub(super) fn previewing() -> Vec<Scenario> {
    vec![
        preview_opens_with_ghosts_and_plan_commands(),
        esc_cancels_nothing_runs(),
        cancel_is_the_default_choice(),
        stale_preview_refused(),
        preview_too_small_never_confirms(),
    ]
}

/// `r` on `main`'s row from `feature`: the rebase preview.
fn r_on_main() -> Vec<Step> {
    vec![
        Step::WaitFor(Wait::Ready),
        Step::CursorTo("main"),
        key(ch('r')),
        Step::WaitFor(Wait::PreviewReady),
    ]
}

/// Nothing moved between setup and `mark`: refs and files alike.
fn nothing_ran(marks: &Marks, mark: &str) -> Result<(), String> {
    let after = marks.get(mark)?;
    same_refs("nothing ran", &marks.before, after)?;
    same_files("nothing ran", &marks.before, after)
}

fn preview_opens_with_ghosts_and_plan_commands() -> Scenario {
    Scenario {
        name: "preview-opens-with-ghosts-and-plan-commands",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('r')),
            Step::WaitFor(Wait::PreviewReady),
            Step::See("the box is a rebase preview", |state| {
                title_names(state, "rebase")
            }),
            Step::See("the box says what the rebase does", |state| {
                box_says(state, "2 commits replayed")?;
                box_says(state, "feature ")?;
                box_says(state, "→ 2 new commits")?;
                box_says(state, "no conflicts expected")?;
                box_says(state, "undo: u takes this back")
            }),
            Step::See("ghost rows are drawn", ghosts_drawn),
            Step::Claim,
            Step::Mark("open"),
        ],
        expect: |_, marks| {
            nothing_ran(marks, "open")?;
            let commands = &marks.claim()?.plan.plan.commands;
            crate::repo_truth::check(
                commands.len() == 1 && commands[0].starts_with("git rebase "),
                || format!("the plan's commands are {commands:?}"),
            )
        },
        capture: Some(Capture {
            stem: "m5-01-preview-rebase",
            headless: vec![],
            keys: plain(&[ch('j'), ch('j'), ch('r')]),
            on_screen: "Preview: rebase",
            when: When::Settled,
        }),
    }
}

fn esc_cancels_nothing_runs() -> Scenario {
    Scenario {
        name: "esc-cancels-nothing-runs",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('r')),
            Step::WaitFor(Wait::PreviewReady),
            key(KeyCode::Escape),
            Step::See("Esc closed the preview", nothing_open),
            Step::See("nothing is running", |state| {
                crate::repo_truth::check(state.operating.is_none(), || {
                    "an operation is running".to_owned()
                })
            }),
            Step::Mark("closed"),
        ],
        expect: |_, marks| nothing_ran(marks, "closed"),
        // The frame after `Esc`: the cursor on `main`, a branch that is not
        // checked out, with no box — the hint bar for a non-current branch
        // row. At 160 columns that bar is the one the conductor wants the
        // user to look at: `q quit` has degraded to a bare `q` since `b`
        // and `t` went live (m5-history-and-ghost-preview/11).
        capture: Some(Capture {
            stem: "m5-07-branch-row-hints",
            headless: vec![],
            keys: plain(&[ch('j'), ch('j'), ch('r'), KeyCode::Escape]),
            // `r rebase` is on the bar at every width; `b branch` is a bare
            // `b` at 80.
            on_screen: "r rebase",
            when: When::Settled,
        }),
    }
}

fn cancel_is_the_default_choice() -> Scenario {
    Scenario {
        name: "cancel-is-the-default-choice",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('r')),
            Step::WaitFor(Wait::PreviewReady),
            // `Enter` at once: the cursor is on `Cancel` (ADR-0038 §1).
            key(KeyCode::Enter),
            Step::See("Enter at once closed the preview", nothing_open),
            Step::See("nothing is running", |state| {
                crate::repo_truth::check(state.operating.is_none(), || {
                    "an operation is running".to_owned()
                })
            }),
            Step::Mark("closed"),
        ],
        expect: |_, marks| nothing_ran(marks, "closed"),
        capture: None,
    }
}

/// A teammate's push, an editor's `git`: `main` moves while the box is up.
fn main_moves_out_of_band(fixture: &Fixture) -> Result<(), String> {
    fixture
        .git(&["update-ref", "refs/heads/main", "main~1"])
        .map(drop)
}

fn stale_preview_refused() -> Scenario {
    Scenario {
        name: "stale-preview-refused",
        shape: Shape::Trivial,
        remote: false,
        size: (80, 24),
        setup: forked,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::CursorTo("main"),
            key(ch('r')),
            Step::WaitFor(Wait::PreviewReady),
            Step::Mark("drawn"),
            Step::Shell(main_moves_out_of_band),
            Step::Confirm,
            Step::Settle("the refused rebase"),
            Step::See("the status says main moved", |state| {
                status_containing(state, "main moved since the preview")?;
                status_containing(state, "r again")
            }),
            Step::Mark("refused"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            super::vocabulary::a_status_said(refused, "main moved since the preview")?;
            let moved = fixture.git(&["rev-parse", "main"])?;
            crate::repo_truth::check(
                super::vocabulary::branch(refused, "main")? == moved.trim(),
                || "main is not where the shell put it".to_owned(),
            )?;
            crate::repo_truth::check(
                super::vocabulary::branch(refused, "feature")?
                    == super::vocabulary::branch(&marks.before, "feature")?,
                || "the refused rebase moved feature".to_owned(),
            )?;
            same_files("the refused rebase", &marks.before, refused)?;
            crate::repo_truth::check(refused.journal.is_empty(), || {
                format!("the refusal journaled {} entries", refused.journal.len())
            })
        },
        // No capture: the refusal needs `main` to move *between* the plan
        // and `Enter`, and a pty run has no hook between two prelude keys
        // (handoff note in the ticket).
        capture: None,
    }
}

fn preview_too_small_never_confirms() -> Scenario {
    Scenario {
        name: "preview-too-small-never-confirms",
        shape: Shape::Trivial,
        remote: false,
        size: (38, 10),
        setup: forked,
        steps: {
            let mut steps = r_on_main();
            steps.extend([
                Step::See("the box does not fit", |state| {
                    crate::repo_truth::check(!graft::update::preview::fits(state), || {
                        "a 38x10 screen fits the box".to_owned()
                    })
                }),
                Step::Confirm,
                Step::Settle("Enter on the one-liner"),
                Step::See("the preview is still open, nothing ran", |state| {
                    crate::repo_truth::check(
                        state.preview.is_some() && state.operating.is_none(),
                        || format!("preview open: {}", state.preview.is_some()),
                    )
                }),
                Step::Mark("after-enter"),
            ]);
            steps
        },
        expect: |_, marks| {
            nothing_ran(marks, "after-enter")?;
            let after = marks.get("after-enter")?;
            crate::repo_truth::check(after.journal.is_empty(), || {
                "Enter on a too-small preview wrote a journal entry".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m5-05-preview-too-small",
            headless: vec![],
            keys: plain(&[ch('j'), ch('j'), ch('r')]),
            on_screen: "preview needs",
            when: When::Settled,
        }),
    }
}
