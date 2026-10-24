use super::done_line;
use crate::update::{AppState, GraphOverlay, Screen, Status};
use mergewright_ui::graph_screen::ProblemKind;

/// ease-pass 26, the invariant: whichever screen owns the bottom row,
/// `Status::Failed` with words only draws `⊗ <words>` once and no `✗`.
/// The screens are every reader of `shown_status` that draws a bar row:
/// the graph footer and the tab layout (`draw_status_or_hints`), the
/// resolver, the commit editor, J (`journal_status_row`), the status
/// screen. `false` rows draw no `⊗` today: Map and finish draw no status
/// words; S draws its notice as a body line, which part B marks.
#[test]
fn every_failed_status_draws_the_mark_once() {
    let words = "x went wrong";
    let shows_a_bar: [(&str, Option<GraphOverlay>, bool); 9] = [
        ("graph", None, true),
        ("resolver", Some(GraphOverlay::Resolver), true),
        ("commit editor", Some(GraphOverlay::CommitEditor), true),
        ("journal", Some(GraphOverlay::Journal), true),
        // The finish summary's bottom row is `view::finish::finish_bar`
        // (finish.rs:157-167): ` committing…` or empty, never the
        // status, so no words and no mark. Product question, not a defect
        // of the mark.
        ("finish", Some(GraphOverlay::Finish), false),
        ("rewrite", Some(GraphOverlay::Rewrite), false),
        ("place", Some(GraphOverlay::Place), false),
        ("map", Some(GraphOverlay::Map), false),
        ("stash", Some(GraphOverlay::Stash), true),
    ];
    let mut wrong = Vec::new();
    for (name, overlay, expected) in shows_a_bar {
        for (columns, rows) in [(120, 40), (187, 50)] {
            let mut state = crate::view::frame::tests::standing_on(overlay);
            state.status = Status::Failed(words.to_owned());
            let drawn = crate::view::tests::draw(&state, columns, rows);
            let marks = drawn.matches(&format!("⊗ {words}")).count();
            let lost = drawn.matches('✗').count();
            if (expected && marks != 1) || marks > 1 || lost != 0 {
                wrong.push(format!("{name} {columns}x{rows}: {marks} marks, {lost} ✗"));
            }
        }
    }
    let mut status = AppState {
        screen: Screen::Status,
        status: Status::Failed(words.to_owned()),
        ..AppState::default()
    };
    status.repo_path = std::path::PathBuf::from("/code/api");
    let drawn = crate::view::tests::draw(&status, 120, 40);
    if drawn.matches(&format!("⊗ {words}")).count() != 1 || drawn.contains('✗') {
        wrong.push(format!("status screen:\n{drawn}"));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The commit mock's landing line (`mergewright-commit-screen.html:178`):
/// the tick first, then what finished, two spaces, then the keys.
#[test]
fn the_done_line_leads_with_its_tick() {
    assert_eq!(
        done_line("Committed 3 files to main.", None, Some("u undo · P push")),
        "✓ Committed 3 files to main.  u undo · P push"
    );
    assert_eq!(done_line("x", None, None), "✓ x");
    assert_eq!(
        done_line("x", None, Some("c finishes the merge")),
        "✓ x  c finishes the merge"
    );
}

/// Review m6-final-fixes-74 M4: a fact that is not a key (`your edits
/// are back`) rides in `also`, drawn after `what` with a `·` and before
/// any key — never in `keys`, where the key-driven paths replace it.
#[test]
fn done_line_places_also_before_keys() {
    assert_eq!(
        done_line("rebase", Some("your edits are back"), None),
        "✓ rebase · your edits are back"
    );
    assert_eq!(
        done_line("rebase", Some("your edits are back"), Some("u undoes")),
        "✓ rebase · your edits are back  u undoes"
    );
}

/// ease-pass 28 A: a sticky line carries its kind. `Failure` draws `⊗ <reason>`
/// once with its keys whole after it; `Notice` draws no mark. Checked on the
/// graph footer; the modal bar only ever shows a refusal (a notice).
#[test]
fn sticky_draws_the_mark_by_its_kind() {
    let mut state = crate::view::frame::tests::standing_on(None);
    state.status = Status::sticky_failure("x went wrong");
    let drawn = crate::view::tests::draw(&state, 120, 40);
    assert!(drawn.contains("⊗ x went wrong"), "{drawn}");
    assert_eq!(drawn.matches('⊗').count(), 1, "{drawn}");

    state.status = Status::Sticky {
        reason: "push rejected: origin/main has commits you don't".to_owned(),
        keys: Some("p pull · P force".to_owned()),
        kind: ProblemKind::Failure,
    };
    let drawn = crate::view::tests::draw(&state, 120, 40);
    assert!(
        drawn.contains("⊗ push rejected") && drawn.contains("p pull · P force"),
        "{drawn}"
    );
    assert_eq!(drawn.matches('⊗').count(), 1, "{drawn}");

    state.status = Status::sticky("x went wrong");
    let drawn = crate::view::tests::draw(&state, 120, 40);
    assert!(drawn.contains(" x went wrong"), "{drawn}");
    assert!(!drawn.contains('⊗') && !drawn.contains("(x)"), "{drawn}");
}

/// ease-pass 28 A: the refusals stay plain: a sticky notice, the name prompt's
/// refusal and the part-way undo cancel (the journal's recover refusal is
/// pinned as a notice by `failure_marks::sticky_sites_are_pinned_by_kind`).
#[test]
fn sticky_notices_carry_no_mark() {
    let mut state = crate::view::frame::tests::standing_on(None);
    state.status = Status::sticky("x went wrong");
    let drawn = crate::view::tests::draw(&state, 120, 40);
    assert!(!drawn.contains('⊗') && !drawn.contains("(x)"), "{drawn}");

    // The part-way cancel is built where it happens: not a failure.
    let mut state = crate::view::frame::tests::standing_on(None);
    state.operating = Some(mergewright_core::job::JobId::new(2));
    crate::update::undo::undone(
        &mut state,
        mergewright_core::job::JobId::new(2),
        crate::jobs::undo::Undone::Cancelled {
            clean: false,
            why: "undo stopped part-way".to_owned(),
            refresh: mergewright_ops::apply::Refresh::WorkingTree,
        },
        &mut || mergewright_core::job::JobId::new(9),
    );
    assert!(matches!(
        state.status,
        Status::Sticky {
            kind: ProblemKind::Notice,
            ..
        }
    ));
    let drawn = crate::view::tests::draw(&state, 120, 40);
    assert!(drawn.contains("undo stopped part-way"), "{drawn}");
    assert!(!drawn.contains('⊗') && !drawn.contains("(x)"), "{drawn}");

    let at = mergewright_core::oid::Oid::from_bytes(&[7; mergewright_core::oid::Oid::SHA1_BYTES])
        .expect("20 bytes");
    let mut state = crate::view::frame::tests::standing_on(None);
    assert!(crate::update::name_prompt::open_name_prompt(
        &mut state,
        crate::update::name_prompt::NameKind::Tag { at },
    ));
    state
        .name_prompt
        .as_mut()
        .expect("the prompt is open")
        .refusal = Some(crate::update::name_prompt::PromptRefusal::EmptyName);
    let drawn = crate::view::tests::draw(&state, 120, 40);
    assert!(drawn.contains("a tag needs a name"), "{drawn}");
    assert!(!drawn.contains('⊗') && !drawn.contains("(x)"), "{drawn}");
}

/// A wait is not a problem: neither the status screen nor the graph
/// footer's problem line shows it (review 47-49, m1).
#[test]
fn a_wait_is_never_drawn_as_a_problem() {
    let wait = super::Status::Waiting("still reading the refs — try again".to_owned());
    assert_eq!(super::problem(&wait), None);
    assert!(super::graph_problem(&wait, mergewright_ui::keymap::Keymap::shipped()).is_none());
}
