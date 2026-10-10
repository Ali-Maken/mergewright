//! m7-power-tools/30o-b R5: the review's minors in J — the bar's `b` from
//! the one function `b` reads, `b` on a `●` row, `⏎` on a step Graft does
//! not undo, and a lost commit whose subject could not be read.

use graft_ops::journal_list::OutsideStep;

use super::*;

fn frame(state: &AppState) -> Vec<String> {
    crate::view::tests::draw(state, 120, 30)
        .lines()
        .map(|line| line.trim_matches('"').trim_end().to_owned())
        .collect()
}

/// The bottom row: J's bar.
fn bar(frame: &[String]) -> &str {
    frame.last().map_or("", String::as_str)
}

/// J open on one `$` step that names no commit (a record with neither an
/// old nor a new value), built straight onto the list: git's reflog never
/// writes one that `steps` keeps, so this is the only way to reach it.
fn on_a_step_with_no_commit() -> AppState {
    let mut state = opened();
    state.journal.list = Some(Arc::new(JournalList {
        rows: vec![ListRow::Outside(OutsideStep {
            record: record("refs/heads/main", 0, 0, 1005, "update by push").record,
            words: "moved main".to_owned(),
            folded: 0,
        })],
        scan: LostScan::Done,
    }));
    state.journal.cursor = row_key(state.journal.list.as_ref().expect("set"), 0);
    state
}

/// Minor 2: `b new branch here` only where `b` has a commit to name
/// (`row_commit`), never from the row's kind alone.
#[test]
fn no_b_hint_on_a_step_that_names_no_commit() {
    let state = on_a_step_with_no_commit();
    let frame = frame(&state);
    let body = frame.join("\n");
    assert!(frame[0].starts_with("JOURNAL"), "J is up:\n{body}");
    assert!(
        !bar(&frame).contains("b new branch here"),
        "no b hint:\n{body}"
    );
    assert!(bar(&frame).ends_with("J/esc close  ? keys"), "{body}");
}

/// Minor 3: a `●` entry can move several refs or none, so `b` there says
/// where `b` works instead of "this step names no commit".
#[test]
fn b_on_a_graft_row_says_where_b_works() {
    let mut state = opened();
    press(&mut state, KeyCode::Char('b'));
    assert!(state.name_prompt.is_none());
    assert_eq!(
        state.status,
        Status::sticky("b makes a branch on a $ or ✗ row")
    );
}

/// The newest row a fetch: `[fetch, commit ← u, stage]`.
fn with_a_fetch_on_top() -> AppState {
    let mut state = opened_with(entry(3, OperationKind::Fetch, "fetch origin"));
    put_cursor(&mut state, 0);
    state
}

/// Minor 4: nothing to go back to on a fetch row — "before this" there is
/// "before" the next newer `●` row, which keeps its `⏎`.
#[test]
fn enter_on_a_fetch_row_says_graft_doesnt_undo_it() {
    let mut state = with_a_fetch_on_top();
    let (claimed, commands) = press(&mut state, KeyCode::Enter);
    assert!(claimed);
    assert!(state.preview.is_none(), "no preview");
    assert!(commands.is_empty(), "nothing planned: {commands:?}");
    assert!(state.journal.shown, "J stays");
    assert_eq!(
        state.status,
        Status::sticky("Tributary doesn't undo a fetch · nothing to go back to on this row")
    );
}

/// Minor 4: the bar offers no `⏎` there.
#[test]
fn no_enter_hint_on_a_fetch_row_in_the_bar() {
    let state = with_a_fetch_on_top();
    let frame = frame(&state);
    let body = frame.join("\n");
    assert!(!bar(&frame).contains('⏎'), "no ⏎ hint:\n{body}");
    assert!(
        bar(&frame).starts_with("/ find  space command log"),
        "{body}"
    );
}

/// Minor 4's other half: a `●` row Graft does undo keeps the time machine.
#[test]
fn enter_on_a_graft_row_is_the_revert_as_before() {
    let mut state = with_a_fetch_on_top();
    // [fetch, commit ← u, stage]: the stage is older than `u`'s.
    put_cursor(&mut state, 2);
    let (_, commands) = press(&mut state, KeyCode::Enter);
    let preview = state.preview.as_ref().expect("the time machine's preview");
    assert_eq!(
        preview.subject,
        crate::update::preview::PreviewSubject::RevertTo(older_entry())
    );
    assert!(
        matches!(commands.as_slice(), [Command::PlanPreview(_, _)]),
        "{commands:?}"
    );
}

/// Minor 5: a lost tip whose commit could not be read is drawn by its sha7
/// alone — no quoted placeholder standing where a subject would be.
#[test]
fn a_lost_commit_with_no_subject_draws_its_sha_alone() {
    let mut state = opened();
    outside_lands(&mut state, shell_reset());
    let mut unread = found(&[4]);
    unread.subjects.clear();
    lost_lands(&mut state, unread);
    let view = crate::view::journal_view(&state, 0);
    let row = view
        .rows
        .iter()
        .find(|row| row.kind == graft_ui::journal_screen::RowKind::Lost)
        .expect("the lost row");
    let words = row.description.as_str();
    assert!(words.starts_with("0404040 · "), "{words}");
    assert!(!words.contains('"'), "no quotes: {words}");
}
