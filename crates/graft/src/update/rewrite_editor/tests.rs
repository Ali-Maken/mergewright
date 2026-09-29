//! The rewrite editor over scripted state (m7-power-tools/08-a): opening,
//! its keys, the AFTER plan's routing, `y`, and leaving. What the rewrite
//! *does* is graft-ops' and is proved against real git there and in the
//! walks (08-b); the open read is `jobs::rewrite_open::tests`'.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::diff::{DiffSpec, FileList};
use graft_git::repo::HeadState;
use graft_graph::Row;
use graft_ops::operation::{Operation, Refusal, RewriteKey, Tip};
use graft_ops::operation::{RebaseParams, Source};
use graft_ops::rewrite::{
    BranchLine, RewriteParams, RewriteRequest, TodoAction, TodoLine, TodoProblem,
};
use graft_ui::text_input::TextField;

use super::{After, AfterFacts, Field, MarkedBy, OpenedFrom, RewriteEditor, words};
use crate::history::{HistoryUpdate, Window};
use crate::jobs::operate::Operated;
use crate::jobs::rewrite_open::{BaseRow, OpenedRewrite, RewriteOpened};
use crate::update::detail::Loading;
use crate::update::preview::{Planned, PreviewFailure, PreviewSubject};
use crate::update::state::GraphOverlay;
use crate::update::{AppEvent, AppState, Command, Product, Progress, Size, Status, update};

fn oid(n: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[19] = n;
    Oid::from_sha1(bytes)
}

fn who() -> Identity {
    Identity {
        name: b"Ada".to_vec(),
        email: b"ada@example.com".to_vec(),
        when: Timestamp {
            seconds: 1_700_000_000,
            offset_seconds: 0,
        },
    }
}

fn row(id: u8, parents: &[u8]) -> Row {
    Row {
        commit: Commit {
            id: oid(id),
            parents: parents.iter().copied().map(oid).collect(),
            author: who(),
            committer: who(),
            message: format!("commit {id}").into_bytes(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    }
}

fn name(full: &[u8]) -> RefName {
    RefName::new(full.to_vec()).expect("a valid ref name")
}

/// `feature` at `HEAD` (commit 5), `main` at 1.
fn refs() -> crate::refs::RefBadges {
    crate::refs::badges(
        &[
            (name(b"refs/heads/feature"), oid(5)),
            (name(b"refs/heads/main"), oid(1)),
        ],
        &HeadState::Branch(name(b"refs/heads/feature")),
        &HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("badges")
}

/// A 187x50 screen, five rows (5 → 4 → 3 → 2 → 1), `feature` at 5, the
/// cursor on graph row `at` (0 is `HEAD`).
fn on_screen(at: usize) -> AppState {
    let (state, _) = crate::update::opening(
        std::path::PathBuf::from("/tmp/graft-rewrite-editor-tests"),
        Size {
            columns: 187,
            rows: 50,
        },
    );
    let id = state.awaiting.expect("opening mints the walk's id");
    let (state, _) = update(
        &state,
        AppEvent::Started(id, crate::update::Task::ReadHistory),
    );
    let asked = state.history.as_ref().expect("a walk").asked;
    let window = Window {
        request: asked,
        first: 0,
        rows: vec![
            row(5, &[4]),
            row(4, &[3]),
            row(3, &[2]),
            row(2, &[1]),
            row(1, &[]),
        ]
        .into(),
        laid_out: 5,
        done: true,
        promoted: false,
    };
    let (mut state, _) = update(
        &state,
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    state.refs = Some(Arc::new(refs()));
    for _ in 0..at {
        state = update(&state, key('j')).0;
    }
    state
}

fn press_key(state: &AppState, press: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(press)))
}

fn key(c: char) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(c))))
}

fn press(state: &AppState, c: char) -> (AppState, Vec<Command>) {
    press_key(state, KeyPress::plain(KeyCode::Char(c)))
}

fn plain(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    press_key(state, KeyPress::plain(code))
}

fn alt(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    press_key(
        state,
        KeyPress {
            code,
            modifiers: Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        },
    )
}

fn line(action: TodoAction, n: u8, subject: &str) -> TodoLine {
    TodoLine {
        action,
        commit: oid(n),
        subject: subject.to_owned(),
    }
}

/// `plan(Open)` over four plain commits since `main`, oldest first: 2, 3,
/// 4, 5. On screen (newest first): row 0 is 5, row 3 is 2, row 4 the base.
fn opened_with(todo: Vec<TodoLine>) -> OpenedRewrite {
    let whole = HashMap::from([
        (
            oid(2),
            "Add OAuth callback\n\nThe callback body.\n".to_owned(),
        ),
        (oid(3), "Handle expired tokens\n".to_owned()),
        (
            oid(4),
            "wip: tests\n\nCovers the expired path.\n".to_owned(),
        ),
        (oid(5), "wip\n".to_owned()),
    ]);
    OpenedRewrite {
        params: RewriteParams {
            head: Tip {
                name: "refs/heads/feature".to_owned(),
                oid: oid(5),
            },
            onto: oid(1),
            todo,
            line: BranchLine::default(),
            messages: BTreeMap::new(),
            made: Vec::new(),
            index_base: None,
        },
        whole,
        base: BaseRow {
            oid: oid(1),
            subject: "Bump version to 2.4".to_owned(),
            label: "main".to_owned(),
        },
    }
}

fn four_picks() -> Vec<TodoLine> {
    vec![
        line(TodoAction::Pick, 2, "Add OAuth callback"),
        line(TodoAction::Pick, 3, "Handle expired tokens"),
        line(TodoAction::Pick, 4, "wip: tests"),
        line(TodoAction::Pick, 5, "wip"),
    ]
}

fn open_job(commands: &[Command]) -> Option<(JobId, Oid)> {
    commands.iter().find_map(|command| match command {
        Command::OpenRewrite {
            id,
            req: RewriteRequest::Open { from },
        } => Some((*id, *from)),
        _ => None,
    })
}

/// The AFTER plan asked: its id and the params it plans.
fn plan_job(commands: &[Command]) -> Option<(JobId, RewriteParams)> {
    commands.iter().find_map(|command| match command {
        Command::PlanPreview(id, request) => match &request.subject {
            PreviewSubject::Operation { op, opened_by } => {
                assert_eq!(*opened_by, "R");
                match op.as_ref() {
                    Operation::Rewrite(params) => Some((*id, params.clone())),
                    other => panic!("the editor plans a rewrite, not {other:?}"),
                }
            }
            PreviewSubject::RevertTo(_) => None,
        },
        _ => None,
    })
}

fn cancels(commands: &[Command], job: JobId) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, Command::CancelJob(id) if *id == job))
}

fn operate_job(commands: &[Command]) -> Option<(JobId, &Operation)> {
    commands.iter().find_map(|command| match command {
        Command::Operate(id, op) => Some((*id, op.as_ref())),
        _ => None,
    })
}

fn answer(state: &AppState, id: JobId, product: Product) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Produced(id, product))
}

fn opened_answer(opened: OpenedRewrite) -> Product {
    Product::RewriteOpened(Ok(RewriteOpened::Editor(Box::new(opened))))
}

/// `R` on graph row `at`, answered with `todo`: the editor is up and its
/// first AFTER plan is out (its id returned).
fn editor_with(at: usize, todo: Vec<TodoLine>) -> (AppState, JobId) {
    let (state, commands) = press(&on_screen(at), 'R');
    let (id, _) = open_job(&commands).expect("R reads");
    let (state, commands) = answer(&state, id, opened_answer(opened_with(todo)));
    let (plan, _) = plan_job(&commands).expect("the first AFTER plan");
    (state, plan)
}

fn editor_up(at: usize) -> (AppState, JobId) {
    editor_with(at, four_picks())
}

fn a_planned(runs: Option<Operation>) -> Arc<Planned> {
    Arc::new(Planned {
        plan: crate::update::preview::tests::a_plan(),
        anchors: Vec::new(),
        runs: runs.map(Box::new),
    })
}

fn landed(state: &AppState, job: JobId) -> AppState {
    answer(state, job, Product::Preview(Ok(a_planned(None)))).0
}

fn ed(state: &AppState) -> &RewriteEditor {
    state.rewrite.editor.as_deref().expect("the editor is up")
}

fn actions(state: &AppState) -> Vec<TodoAction> {
    ed(state).todo.iter().map(|row| row.action).collect()
}

/// An editor over [`four_picks`], cursor on `HEAD`, with no job out: for
/// the sweeps elsewhere that only need one standing.
pub(crate) fn an_editor() -> Box<RewriteEditor> {
    Box::new(RewriteEditor::new(
        opened_with(four_picks()),
        OpenedFrom::Row(oid(5)),
    ))
}

/// Stand an editor on `state`.
pub(crate) fn open_an_editor(state: &mut AppState) {
    state.rewrite.editor = Some(an_editor());
}

/// Stand an editor on `state` whose AFTER plan landed (`y` starts), or
/// whose todo cannot run (`y` refuses): the rewrite editor bar's two
/// `can_start` forms.
pub(crate) fn open_an_editor_planned(state: &mut AppState, can_start: bool) {
    let mut editor = an_editor();
    if can_start {
        editor.after = After::Ready {
            planned: a_planned(None),
            asked: editor.opened.clone(),
        };
    }
    state.rewrite.editor = Some(editor);
}

/// Stand an editor on `state` with its `r` field open and empty.
pub(crate) fn open_an_editor_field(state: &mut AppState) {
    let mut editor = an_editor();
    editor.field = Some(Field {
        commit: oid(5),
        text: TextField::new(""),
        handover: None,
    });
    state.rewrite.editor = Some(editor);
}

/// What the editor's `r` field holds; `None` with no field.
pub(crate) fn field_text(state: &AppState) -> Option<String> {
    let field = state.rewrite.editor.as_ref()?.field.as_ref()?;
    Some(field.text.text().to_owned())
}

// ── opening ────────────────────────────────────────────────────────────

#[test]
fn capital_r_opens_on_the_cursor_row() {
    let (state, commands) = press(&on_screen(2), 'R');
    assert_eq!(open_job(&commands).map(|(_, from)| from), Some(oid(3)));
    assert_eq!(
        state.status,
        Status::Waiting("reading feature\u{2026}".to_owned())
    );
    assert_eq!(
        state.graph_overlay(),
        None,
        "no overlay until the read answers"
    );

    let (id, _) = open_job(&commands).expect("R reads");
    let (state, commands) = answer(&state, id, opened_answer(opened_with(four_picks())));
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Rewrite));
    assert_eq!(ed(&state).cursor, 2, "the cursor on commit 3's row");
    assert_eq!(state.status, Status::Ready);
    assert!(
        plan_job(&commands).is_some(),
        "the first AFTER plan is asked"
    );
}

#[test]
fn capital_r_on_the_uncommitted_row_opens_on_head() {
    let mut state = on_screen(2);
    state.on_working_tree = true;
    let (_, commands) = press(&state, 'R');
    assert_eq!(open_job(&commands).map(|(_, from)| from), Some(oid(5)));
}

#[test]
fn a_second_capital_r_while_reading_starts_nothing() {
    let (state, _) = press(&on_screen(0), 'R');
    let (again, commands) = press(&state, 'R');
    assert!(open_job(&commands).is_none(), "{commands:?}");
    assert_eq!(again.rewrite.opening, state.rewrite.opening);
}

#[test]
fn esc_while_reading_cancels_the_open() {
    let (state, commands) = press(&on_screen(0), 'R');
    let (id, _) = open_job(&commands).expect("R reads");
    let (state, commands) = plain(&state, KeyCode::Escape);
    assert!(cancels(&commands, id), "{commands:?}");
    assert_eq!(state.rewrite.opening, None);
    assert_eq!(state.status, Status::Ready, "the reading line is cleared");

    // Its answer, already on its way, opens nothing.
    let (state, _) = answer(&state, id, opened_answer(opened_with(four_picks())));
    assert_eq!(state.graph_overlay(), None);
}

#[test]
fn capital_r_during_an_operation_is_refused_in_words() {
    let mut state = on_screen(0);
    state.operating = Some(JobId::new(7));
    let (state, commands) = press(&state, 'R');
    assert!(open_job(&commands).is_none(), "{commands:?}");
    assert_eq!(state.status, Status::sticky("wait for the write to finish"));
}

#[test]
fn a_refused_open_shows_the_words_and_no_overlay() {
    let (state, commands) = press(&on_screen(0), 'R');
    let (id, _) = open_job(&commands).expect("R reads");
    let refusal = Refusal::ForkedDependents {
        a: name(b"refs/heads/feature-b"),
        b: name(b"refs/heads/feature-c"),
    };
    let (state, _) = answer(
        &state,
        id,
        Product::RewriteOpened(Ok(RewriteOpened::Refused(refusal))),
    );
    assert_eq!(
        state.status,
        Status::sticky(
            "feature-b and feature-c are both built on yours \u{b7} Graft moves one line"
        )
    );
    assert_eq!(state.graph_overlay(), None);
}

#[test]
fn capital_r_on_another_branch_is_refused_in_words() {
    let (state, commands) = press(&on_screen(0), 'R');
    let (id, _) = open_job(&commands).expect("R reads");
    let refusal = Refusal::NotOnYourBranch {
        commit: oid(9),
        by: RewriteKey::Open,
    };
    let words = refusal.explain();
    let (state, _) = answer(
        &state,
        id,
        Product::RewriteOpened(Ok(RewriteOpened::Refused(refusal))),
    );
    // `oid(9)`'s short sha is its first seven digits: all zeros.
    assert_eq!(words, "R rewrites your branch \u{b7} 0000000 isn't on it");
    assert_eq!(state.status, Status::sticky(words));
    assert_eq!(state.graph_overlay(), None);
}

#[test]
fn a_failed_open_read_says_so() {
    let (state, commands) = press(&on_screen(0), 'R');
    let (id, _) = open_job(&commands).expect("R reads");
    let (state, _) = answer(
        &state,
        id,
        Product::RewriteOpened(Err("couldn't read the refs".to_owned())),
    );
    assert_eq!(
        state.status,
        Status::Failed("couldn't open the rewrite editor: couldn't read the refs".to_owned())
    );
    assert_eq!(state.graph_overlay(), None);
}

// ── rows ───────────────────────────────────────────────────────────────

#[test]
fn s_marks_and_moves_one_older() {
    let (state, first) = editor_up(0);
    let (state, commands) = press(&state, 's');
    assert_eq!(
        actions(&state),
        [
            TodoAction::Pick,
            TodoAction::Pick,
            TodoAction::Pick,
            TodoAction::Squash
        ]
    );
    assert_eq!(ed(&state).cursor, 1, "one row older");
    assert!(cancels(&commands, first), "the old plan is cancelled");
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(params.todo[3].action, TodoAction::Squash);
}

#[test]
fn p_on_a_marked_row_unmarks_it() {
    let (state, _) = editor_up(0);
    let (state, _) = press(&state, 's');
    let (state, _) = press(&state, 'k');
    let (state, _) = press(&state, 'p');
    assert_eq!(actions(&state), [TodoAction::Pick; 4]);
}

#[test]
fn autosquash_rows_open_marked_by_their_name() {
    let todo = vec![
        line(TodoAction::Pick, 2, "Add OAuth callback"),
        line(TodoAction::Fixup, 4, "fixup! Add OAuth callback"),
        line(TodoAction::Pick, 3, "Handle expired tokens"),
        line(TodoAction::Pick, 5, "wip"),
    ];
    let (state, _) = editor_with(0, todo);
    let facts = ed(&state).view_facts();
    let marked: Vec<_> = facts
        .rows
        .iter()
        .map(|row| (row.commit, row.marked))
        .collect();
    assert_eq!(
        marked,
        [
            (oid(5), None),
            (oid(3), None),
            (oid(4), Some(MarkedBy::Name)),
            (oid(2), None)
        ]
    );
    assert_eq!(facts.rows[3].keeps, 2, "2 in one");

    // His own mark replaces the name's.
    let (state, _) = press(&state, 'j');
    let (state, _) = press(&state, 'j');
    let (state, _) = press(&state, 's');
    assert_eq!(ed(&state).view_facts().rows[2].marked, None);
}

#[test]
fn a_bottom_squash_is_refused_in_words() {
    let (state, _) = editor_up(3);
    assert_eq!(ed(&state).cursor, 3, "on the oldest row, commit 2");
    let (state, commands) = press(&state, 's');
    assert!(plan_job(&commands).is_none(), "nothing is planned");
    let facts = ed(&state).view_facts();
    assert_eq!(
        facts.rows[3].problem.as_deref(),
        Some("nothing below to meld into")
    );
    assert!(
        matches!(&facts.after, AfterFacts::Invalid(words) if words == "nothing below to meld into")
    );

    let (state, commands) = press(&state, 'y');
    assert!(operate_job(&commands).is_none());
    assert_eq!(
        ed(&state).notice.as_deref(),
        Some("nothing below to meld into")
    );
}

#[test]
fn every_commit_dropped_is_refused_in_words() {
    let (mut state, _) = editor_up(0);
    for _ in 0..4 {
        state = press(&state, 'd').0;
    }
    assert_eq!(actions(&state), [TodoAction::Drop; 4]);
    assert_eq!(
        ed(&state).after,
        super::After::Invalid(TodoProblem::EverythingDropped)
    );
    let (state, commands) = press(&state, 'y');
    assert!(operate_job(&commands).is_none());
    assert_eq!(
        ed(&state).notice.as_deref(),
        Some("every commit dropped \u{b7} the branch would be its base again")
    );
}

#[test]
fn the_base_row_refuses_actions() {
    let (state, _) = editor_up(0);
    let state = (0..4).fold(state, |state, _| press(&state, 'j').0);
    assert_eq!(ed(&state).cursor, 4, "on the base row");
    for letter in ['p', 's', 'f', 'd', 'e', 'r'] {
        let (after, commands) = press(&state, letter);
        assert_eq!(actions(&after), [TodoAction::Pick; 4], "{letter}");
        assert!(plan_job(&commands).is_none(), "{letter}");
        assert_eq!(
            ed(&after).notice.as_deref(),
            Some(words::BASE_STAYS),
            "{letter}"
        );
    }
    let (after, _) = alt(&state, KeyCode::Up);
    assert_eq!(ed(&after).notice.as_deref(), Some(words::BASE_STAYS));
    assert_eq!(ed(&after).todo, ed(&state).todo);
}

#[test]
fn alt_up_moves_the_row_and_marks_it() {
    let (state, first) = editor_up(1);
    let (state, commands) = alt(&state, KeyCode::Up);
    let order: Vec<_> = ed(&state).todo.iter().map(|row| row.commit).collect();
    assert_eq!(
        order,
        [oid(2), oid(3), oid(5), oid(4)],
        "4 is now the newest"
    );
    assert_eq!(ed(&state).cursor, 0, "the cursor follows the row");
    let facts = ed(&state).view_facts();
    assert!(facts.rows[0].moved, "↕ moved");
    assert!(!facts.rows[1].moved);
    assert!(cancels(&commands, first));
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(params.todo[3].commit, oid(4));
}

// ── the field and messages ─────────────────────────────────────────────

#[test]
fn r_opens_a_field_that_owns_every_key() {
    let (state, _) = editor_up(0);
    let (state, _) = press(&state, 'r');
    assert!(state.text_field_open());
    assert_eq!(field_text(&state).as_deref(), Some("wip"));
    let mut state = state;
    for letter in ['u', '/', 'q', 'y'] {
        let (next, commands) = press(&state, letter);
        assert!(commands.is_empty(), "{letter}: {commands:?}");
        state = next;
    }
    assert_eq!(field_text(&state).as_deref(), Some("wipu/qy"));
    assert!(state.search_prompt.is_none());
    assert!(state.quit.is_none());
}

#[test]
fn esc_in_the_field_drops_only_the_edit() {
    let (state, _) = editor_up(0);
    let (state, _) = press(&state, 'r');
    let (state, _) = press(&state, 'x');
    let (state, commands) = plain(&state, KeyCode::Escape);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        state.graph_overlay(),
        Some(GraphOverlay::Rewrite),
        "still up"
    );
    assert!(ed(&state).field.is_none());
    assert!(ed(&state).edits.is_empty());
    assert_eq!(actions(&state), [TodoAction::Pick; 4]);
}

#[test]
fn enter_in_the_field_rewords_and_replans() {
    let (state, first) = editor_up(0);
    let (mut state, _) = press(&state, 'r');
    for _ in 0..3 {
        state = plain(&state, KeyCode::Backspace).0;
    }
    for letter in "Done".chars() {
        state = press(&state, letter).0;
    }
    let (state, commands) = plain(&state, KeyCode::Enter);
    assert!(ed(&state).field.is_none());
    assert_eq!(ed(&state).todo[3].action, TodoAction::Reword);
    assert!(cancels(&commands, first));
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(params.messages.get(&3).map(String::as_str), Some("Done\n"));
    assert_eq!(ed(&state).view_facts().rows[0].subject, "Done");
}

#[test]
fn the_squash_message_is_default_message_until_edited() {
    // `s` on commit 4: it melds into 3.
    let (state, _) = editor_up(1);
    let (state, commands) = press(&state, 's');
    let default = "Handle expired tokens\n\nwip: tests\n\nCovers the expired path.\n";
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(params.messages.get(&1).map(String::as_str), Some(default));
    let messages = ed(&state).view_facts().messages;
    assert_eq!(messages.len(), 1);
    assert!(messages[0].squashed);
    assert_eq!(messages[0].kept, oid(3));
    assert_eq!(messages[0].text, default);

    // `r` on the kept row edits the group's message; the action stays.
    assert_eq!(ed(&state).cursor, 2, "on commit 3");
    let (mut state, _) = press(&state, 'r');
    assert_eq!(field_text(&state).as_deref(), Some("Handle expired tokens"));
    for _ in 0.."Handle ".len() {
        state = plain(&state, KeyCode::Home).0;
        state = plain(&state, KeyCode::Delete).0;
    }
    let (state, commands) = plain(&state, KeyCode::Enter);
    let edited = "expired tokens\n\nwip: tests\n\nCovers the expired path.\n";
    assert_eq!(ed(&state).todo[1].action, TodoAction::Pick);
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(params.messages.get(&1).map(String::as_str), Some(edited));
}

#[test]
fn r_on_a_melded_row_says_whose_message_it_is() {
    let (state, _) = editor_up(1);
    let (state, _) = press(&state, 's');
    let (state, _) = press(&state, 'k');
    let (state, _) = press(&state, 'r');
    assert!(ed(&state).field.is_none());
    assert_eq!(
        ed(&state).notice.as_deref(),
        Some("this row melds into \"Handle expired tokens\" \u{b7} r there changes the message")
    );
}

#[test]
fn ctrl_e_hands_the_message_over_and_back() {
    let (state, _) = editor_up(1);
    let (state, _) = press(&state, 'r');
    let (state, commands) = press_key(&state, KeyPress::control(KeyCode::Char('e')));
    let (id, message) = commands
        .iter()
        .find_map(|command| match command {
            Command::EditCommitMessage { id, message } => Some((*id, message.clone())),
            _ => None,
        })
        .expect("the handover");
    assert_eq!(message, "wip: tests\n\nCovers the expired path.\n");
    assert!(ed(&state).view_facts().handing_over);

    // The terminal is away: nothing typed here acts.
    let (away, commands) = press(&state, 'y');
    assert!(commands.is_empty());
    assert_eq!(field_text(&away).as_deref(), Some("wip: tests"));

    let (state, commands) = answer(
        &state,
        id,
        Product::ExternalEditor(Ok("Test the expired path\n\nAll of it.\n\n".to_owned())),
    );
    assert!(ed(&state).field.is_none());
    assert_eq!(ed(&state).todo[2].action, TodoAction::Reword);
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(
        params.messages.get(&2).map(String::as_str),
        Some("Test the expired path\n\nAll of it.\n")
    );
    assert!(
        state.commit_editor.is_none(),
        "the commit editor never saw it"
    );
}

#[test]
fn a_commit_editor_handover_answer_is_not_the_editors() {
    let (state, _) = editor_up(0);
    let (state, _) = press(&state, 'r');
    let (state, _) = press_key(&state, KeyPress::control(KeyCode::Char('e')));
    let before = ed(&state).clone();
    let (state, _) = answer(
        &state,
        JobId::new(4_242),
        Product::ExternalEditor(Ok("from the commit editor".to_owned())),
    );
    assert_eq!(ed(&state), &before, "an answer it never asked for");
}

// ── the right side ─────────────────────────────────────────────────────

#[test]
fn enter_shows_the_rows_changes() {
    let (state, _) = editor_up(0);
    let (state, commands) = plain(&state, KeyCode::Enter);
    let (id, spec) = commands
        .iter()
        .find_map(|command| match command {
            Command::ListFiles(id, spec, _) => Some((*id, *spec)),
            _ => None,
        })
        .expect("the row's files are read");
    assert_eq!(
        spec,
        DiffSpec::Commit {
            commit: oid(5),
            against: None
        }
    );
    let list = Arc::new(FileList::default());
    let (state, _) = answer(
        &state,
        id,
        Product::Files(Ok((Arc::default(), Arc::clone(&list)))),
    );
    assert_eq!(
        ed(&state).view_facts().changes,
        Some((oid(5), &Loading::Ready(list)))
    );

    // `⏎` again: back to AFTER.
    let (state, _) = plain(&state, KeyCode::Enter);
    assert!(ed(&state).view_facts().changes.is_none());
}

#[test]
fn tab_swaps_sides_below_120_only() {
    let (state, _) = editor_up(0);
    let (wide, _) = plain(&state, KeyCode::Tab);
    assert!(!ed(&wide).narrow_shows_right, "≥120: swallowed");
    let mut narrow = state;
    narrow.size.columns = 100;
    let (narrow, _) = plain(&narrow, KeyCode::Tab);
    assert!(ed(&narrow).narrow_shows_right);
    let (narrow, _) = plain(&narrow, KeyCode::Tab);
    assert!(!ed(&narrow).narrow_shows_right);
}

#[test]
fn esc_writes_nothing() {
    let (state, _) = editor_up(0);
    let (state, _) = press(&state, 's');
    let plan = ed(&state).jobs().next().expect("a plan out");
    let (state, commands) = plain(&state, KeyCode::Escape);
    assert_eq!(state.graph_overlay(), None);
    assert!(operate_job(&commands).is_none());
    assert!(cancels(&commands, plan), "{commands:?}");
    assert_eq!(state.rewrite.launched, None);

    // The cancelled plan's answer, already on its way, lands nowhere.
    let (after, _) = answer(&state, plan, Product::Preview(Ok(a_planned(None))));
    assert!(after.preview.is_none());
    assert!(after.rewrite.editor.is_none());
}

// ── planning and `y` ───────────────────────────────────────────────────

#[test]
fn y_before_the_plan_lands_does_nothing() {
    let (state, _) = editor_up(0);
    let (state, commands) = press(&state, 'y');
    assert!(operate_job(&commands).is_none());
    assert!(matches!(
        ed(&state).view_facts().after,
        AfterFacts::Checking
    ));
}

#[test]
fn y_launches_the_planned_rewrite() {
    let (state, plan) = editor_up(0);
    let (state, _) = press(&state, 's');
    let job = ed(&state).jobs().next().expect("a plan out");
    assert_ne!(job, plan);
    let asked = ed(&state).params().expect("valid");
    // The op run is the op pictured: the plan's `runs`.
    let runs = Operation::Rewrite(RewriteParams {
        made: vec![oid(77)],
        ..asked.clone()
    });
    let (state, _) = answer(
        &state,
        job,
        Product::Preview(Ok(a_planned(Some(runs.clone())))),
    );
    assert!(matches!(
        ed(&state).view_facts().after,
        AfterFacts::Ready(_)
    ));
    let (mut state, commands) = press(&state, 'y');
    let (id, op) = operate_job(&commands).expect("y launches");
    assert_eq!(op, &runs);
    assert_eq!(state.graph_overlay(), None);
    assert!(state.rewrite.launched.is_some());

    let commands =
        crate::update::operate::operated(&mut state, id, Operated::Done(applied()), &mut || {
            JobId::new(9_000)
        });
    assert!(operate_job(&commands).is_none());
    assert!(state.rewrite.launched.is_none(), "done ends it");
    assert!(state.rewrite.editor.is_none());
}

#[test]
fn an_edit_replans_and_the_last_wins() {
    let (state, first) = editor_up(0);
    let (state, commands) = press(&state, 's');
    let (second, _) = plan_job(&commands).expect("replanned");
    assert!(cancels(&commands, first));
    let (state, commands) = press(&state, 'f');
    let (third, asked) = plan_job(&commands).expect("replanned");
    assert!(cancels(&commands, second));
    assert_eq!(
        asked.todo[2].action,
        TodoAction::Fixup,
        "the last edit's params"
    );

    let state = landed(&state, third);
    assert!(matches!(
        ed(&state).view_facts().after,
        AfterFacts::Ready(_)
    ));
    assert!(state.preview.is_none(), "the editor never sets the preview");
}

#[test]
fn a_superseded_after_answer_is_dropped() {
    let (state, first) = editor_up(0);
    let (state, commands) = press(&state, 's');
    let (second, _) = plan_job(&commands).expect("replanned");
    // The first plan's answer was already on its way when it was cancelled.
    let state = landed(&state, first);
    assert!(
        matches!(&ed(&state).after, super::After::Pending { job, .. } if *job == second),
        "still waiting on the last edit's plan"
    );
    assert!(state.preview.is_none(), "not the preview's either");
    assert!(!super::owns(&state, first), "dropped once");
}

#[test]
fn a_failed_after_plan_says_so_and_y_refuses() {
    let (state, plan) = editor_up(0);
    let (state, _) = answer(
        &state,
        plan,
        Product::Preview(Err(PreviewFailure::Failed(
            "couldn't read 0000003".to_owned(),
        ))),
    );
    assert!(matches!(
        ed(&state).view_facts().after,
        AfterFacts::Failed("couldn't read 0000003")
    ));
    let (state, commands) = press(&state, 'y');
    assert!(operate_job(&commands).is_none());
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Rewrite));
}

#[test]
fn a_moved_branch_refuses_y_in_words() {
    let (state, plan) = editor_up(0);
    let moved = Refusal::Moved {
        name: "refs/heads/feature".to_owned(),
    };
    let words = moved.explain();
    let (state, _) = answer(
        &state,
        plan,
        Product::Preview(Err(PreviewFailure::Refused(moved))),
    );
    assert!(matches!(ed(&state).view_facts().after, AfterFacts::Refused(said) if said == words));
    let (state, commands) = press(&state, 'y');
    assert!(operate_job(&commands).is_none());
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Rewrite));
}

#[test]
fn a_refused_write_restores_the_editor_with_the_words() {
    let (state, plan) = editor_up(0);
    let state = landed(&state, plan);
    let (mut state, commands) = press(&state, 'y');
    let (id, _) = operate_job(&commands).expect("y launches");
    let moved = Refusal::Moved {
        name: "refs/heads/feature".to_owned(),
    };
    let words = moved.explain();
    crate::update::operate::operated(
        &mut state,
        id,
        Operated::Failed {
            error: "the words nobody should see".to_owned(),
            refresh: None,
            remote: None,
            refused: Some(moved),
            edits: None,
        },
        &mut || JobId::new(9_000),
    );
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Rewrite), "back");
    assert_eq!(ed(&state).notice.as_deref(), Some(words.as_str()));
    assert!(state.rewrite.launched.is_none());
}

// ── modality ───────────────────────────────────────────────────────────

#[test]
fn the_overlay_is_rewrite_while_the_editor_is_up() {
    let (state, _) = editor_up(0);
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Rewrite));
    assert!(state.modal_owns_keys());
    assert!(!state.text_field_open(), "no field until r");
    let (state, _) = plain(&state, KeyCode::Escape);
    assert_eq!(state.graph_overlay(), None);
    assert!(!state.modal_owns_keys());
}

#[test]
fn u_while_the_editor_is_up_does_nothing() {
    let (state, _) = editor_up(0);
    for letter in ['u', 'U', '/', 'q', 'R'] {
        let (after, commands) = press(&state, letter);
        assert!(commands.is_empty(), "{letter}: {commands:?}");
        assert_eq!(ed(&after), ed(&state), "{letter}");
        assert!(after.search_prompt.is_none(), "{letter}");
        assert!(after.quit.is_none(), "{letter}");
    }
}

#[test]
fn ctrl_c_still_quits_under_the_editor() {
    let (state, _) = editor_up(0);
    let (after, _) = press_key(&state, KeyPress::control(KeyCode::Char('c')));
    assert!(after.quit.is_some());
}

fn applied() -> graft_ops::apply::Applied {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    graft_ops::apply::Applied {
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Stage,
            description: "rewrite feature".to_owned(),
            status: EntryStatus::Committed,
            touches: Touches::default(),
            head_before: HeadRef::Branch("refs/heads/feature".to_owned()),
            head_after: None,
            refs_before: BTreeMap::new(),
            refs_after: BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: BTreeMap::new(),
            config_after: BTreeMap::new(),
            put_back: None,
        },
        refresh: graft_ops::apply::Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

// ── what 08-b's view reads ─────────────────────────────────────────────

/// `view_facts` is the whole screen in the screen's order: title facts,
/// rows newest first with the base under them, the field on its row, the
/// message blocks and the AFTER side.
#[test]
fn view_facts_are_the_screen_in_order() {
    let (state, _) = editor_up(1);
    let (state, _) = press(&state, 's');
    let (state, _) = press(&state, 'r');
    let facts = ed(&state).view_facts();
    assert_eq!(facts.branch, "feature");
    assert_eq!(facts.since, 4);
    assert_eq!(facts.base.label, "main");
    assert_eq!(facts.cursor, 2);
    assert_eq!(
        facts.rows.iter().map(|row| row.action).collect::<Vec<_>>(),
        [
            TodoAction::Pick,
            TodoAction::Squash,
            TodoAction::Pick,
            TodoAction::Pick
        ]
    );
    assert_eq!(facts.rows[2].keeps, 2, "2 in one");
    assert_eq!(facts.field, Some((2, "Handle expired tokens", 21)));
    assert_eq!(facts.messages[0].kept_subject, "Handle expired tokens");
}

/// The quiet facts are quiet, and a landed plan is the one shown.
#[test]
fn view_facts_show_the_plan_that_landed() {
    let (state, plan) = editor_up(1);
    let (state, _) = press(&state, 's');
    let facts = ed(&state).view_facts();
    assert_eq!(facts.notice, None);
    assert!(!facts.narrow_shows_right);
    assert!(!facts.handing_over);
    assert!(matches!(facts.after, AfterFacts::Checking));

    // The first plan was superseded by `s`; the second one lands.
    let job = ed(&state).jobs().next().expect("a plan out");
    assert_ne!(job, plan);
    let planned = a_planned(None);
    let (state, _) = answer(&state, job, Product::Preview(Ok(Arc::clone(&planned))));
    assert!(
        matches!(ed(&state).view_facts().after, AfterFacts::Ready(shown) if *shown == *planned)
    );
}

// ── the mouse ──────────────────────────────────────────────────────────

/// The screen line `subject` is drawn on, found in the drawn editor itself
/// (not through `rewrite_row_at`, which is what the click goes through): the
/// first line whose rows' side holds the subject.
fn drawn_line_of(state: &AppState, subject: &str) -> u16 {
    let area = crate::view::rewrite_editor::editor_area(state);
    let view = crate::view::rewrite_editor::rewrite_view(state, false).expect("the editor is up");
    let theme = graft_ui::theme::Theme::new(
        graft_ui::theme::ThemeKind::Dark,
        &crate::session::for_tests::capabilities(),
    );
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    ratatui::widgets::Widget::render(
        graft_ui::rewrite_editor::RewriteEditorWidget {
            view: &view,
            theme: &theme,
        },
        area,
        &mut buffer,
    );
    (area.y..area.bottom())
        .find(|&y| {
            let left: String = (area.x..graft_ui::rewrite_editor::LEFT_WIDTH)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            left.contains(subject)
        })
        .unwrap_or_else(|| panic!("{subject:?} is drawn on the rows' side"))
}

fn mouse(state: &AppState, kind: MouseKind, row: u16) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind,
            column: 20,
            row,
            modifiers: Modifiers::default(),
        })),
    )
}

#[test]
fn a_click_selects_the_row() {
    let (state, _) = editor_up(0);
    assert_eq!(ed(&state).cursor, 0);
    let line = drawn_line_of(&state, "Handle expired tokens");
    let (after, commands) = mouse(&state, MouseKind::Down(MouseButton::Left), line);
    // Newest first: 5, 4, 3, 2 — "Handle expired tokens" (3) is row 2.
    assert_eq!(ed(&after).cursor, 2, "the clicked row is the cursor");
    assert_eq!(ed(&after).todo, ed(&state).todo, "a click moves nothing");
    assert!(
        plan_job(&commands).is_none(),
        "nothing changed, nothing replanned"
    );
    assert_eq!(
        after.graph_overlay(),
        Some(GraphOverlay::Rewrite),
        "the editor stays up"
    );
}

#[test]
fn a_drag_moves_the_row() {
    let (state, first) = editor_up(0);
    let from = drawn_line_of(&state, "wip: tests");
    let to = drawn_line_of(&state, "Add OAuth callback");
    let (state, _) = mouse(&state, MouseKind::Down(MouseButton::Left), from);
    assert_eq!(ed(&state).cursor, 1, "the press selects 4");
    let (state, commands) = mouse(&state, MouseKind::Drag(MouseButton::Left), to);
    let (state, _) = mouse(&state, MouseKind::Up(MouseButton::Left), to);
    let order: Vec<_> = ed(&state).todo.iter().map(|row| row.commit).collect();
    assert_eq!(
        order,
        [oid(4), oid(2), oid(3), oid(5)],
        "4 dragged down to the oldest place (oldest first)"
    );
    assert_eq!(ed(&state).cursor, 3, "the cursor follows the row");
    assert!(ed(&state).view_facts().rows[3].moved, "↕ moved");
    assert!(cancels(&commands, first), "the old plan is cancelled");
    let plans = commands
        .iter()
        .filter(|command| matches!(command, Command::PlanPreview(..)))
        .count();
    assert_eq!(plans, 1, "one AFTER plan for the whole drag");
    let (_, params) = plan_job(&commands).expect("replanned");
    assert_eq!(params.todo[0].commit, oid(4));
    // After the release, a drag moves nothing.
    let (after, _) = mouse(&state, MouseKind::Drag(MouseButton::Left), from);
    assert_eq!(ed(&after).todo, ed(&state).todo);
}

// ── the graph bar ──────────────────────────────────────────────────────

/// Whether the graph bar, at 187 columns, draws `R rewrite` for `state`.
fn bar_offers_r(state: &AppState) -> bool {
    let view = crate::view::rows::graph_view(state, 0);
    graft_ui::graph_screen::hints_for(
        &view.hint_context,
        graft_ui::graph_screen::Divider::Hidden,
        187,
    )
    .iter()
    .any(|hint| hint.key == "R")
}

/// `main`'s first-parent chain is commit 1 alone.
fn with_context(mut state: AppState) -> AppState {
    state.radar.context = Loading::Ready(Arc::new(crate::jobs::radar::RadarContext::of_ids(
        [oid(1)],
        Vec::new(),
    )));
    state
}

#[test]
fn the_bar_offers_capital_r_on_his_commit_rows() {
    for at in 0..4 {
        assert!(
            bar_offers_r(&with_context(on_screen(at))),
            "row {at} is his"
        );
    }
    // No context yet: offered on every row and `plan` decides (ruling 10).
    for at in 0..5 {
        assert!(bar_offers_r(&on_screen(at)), "row {at}, no context");
    }
}

#[test]
fn the_bar_hides_capital_r_on_default_branch_rows() {
    assert!(
        !bar_offers_r(&with_context(on_screen(4))),
        "1 is on main's chain"
    );
    // Detached HEAD: no row is his branch's.
    let mut detached = with_context(on_screen(0));
    let refs = crate::refs::badges(
        &[(name(b"refs/heads/main"), oid(1))],
        &HeadState::Detached(oid(5)),
        &HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("badges");
    detached.refs = Some(Arc::new(refs));
    assert!(!bar_offers_r(&detached), "HEAD on no branch");
}

// ── opened from a rebase preview (rebase-preview-keeps-drops) ────────────

/// The rebase of `feature` (5) onto `main` (1), commits 2, 3 and 4
/// suggested as drops.
fn a_rebase_with_drops() -> RebaseParams {
    let mut params = RebaseParams::new(
        Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(5),
        },
        Source::Branch("refs/heads/main".to_owned()),
        oid(1),
    );
    params.drops = vec![oid(2), oid(3), oid(4)];
    params
}

/// `FromRebase`'s rows for [`a_rebase_with_drops`]: oldest first, the
/// three drops then 5 kept.
fn rebase_todo() -> Vec<TodoLine> {
    vec![
        line(TodoAction::Drop, 2, "Add OAuth callback"),
        line(TodoAction::Drop, 3, "Handle expired tokens"),
        line(TodoAction::Drop, 4, "wip: tests"),
        line(TodoAction::Pick, 5, "wip"),
    ]
}

/// `R` in a rebase preview, answered: the editor is up.
fn editor_from_rebase() -> AppState {
    let mut state = on_screen(0);
    let mut commands = Vec::new();
    let mut next = {
        let mut n = 1000;
        move || {
            n += 1;
            JobId::new(n)
        }
    };
    super::open_rewrite_from_rebase(&mut state, &mut commands, &mut next, a_rebase_with_drops());
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenRewrite {
                id,
                req: RewriteRequest::FromRebase(params),
            } => {
                assert_eq!(params, &a_rebase_with_drops(), "the params as given");
                Some(*id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("a FromRebase read: {commands:?}"));
    assert_eq!(
        state.status,
        Status::Waiting("reading feature\u{2026}".to_owned())
    );
    answer(&state, id, opened_answer(opened_with(rebase_todo()))).0
}

/// The notes the view draws on his rows, newest first.
fn notes(state: &AppState) -> Vec<Option<String>> {
    let view = crate::view::rewrite_editor::rewrite_view(state, false).expect("the editor's view");
    view.rows
        .iter()
        .take(view.rows.len() - 1)
        .map(|row| row.note.as_ref().map(|note| note.as_str().to_owned()))
        .collect()
}

#[test]
fn seeded_drops_read_already_in_and_p_clears_it() {
    let state = editor_from_rebase();
    let marked: Vec<_> = ed(&state)
        .view_facts()
        .rows
        .iter()
        .map(|row| (row.commit, row.action, row.marked))
        .collect();
    assert_eq!(
        marked,
        [
            (oid(5), TodoAction::Pick, None),
            (oid(4), TodoAction::Drop, Some(MarkedBy::AlreadyIn)),
            (oid(3), TodoAction::Drop, Some(MarkedBy::AlreadyIn)),
            (oid(2), TodoAction::Drop, Some(MarkedBy::AlreadyIn)),
        ]
    );
    let seeded = Some("left out \u{b7} already in main \u{b7} p keeps it".to_owned());
    assert_eq!(
        notes(&state),
        [None, seeded.clone(), seeded.clone(), seeded.clone()]
    );

    // `p` on the newest drop (the cursor's): a plain pick, the words go.
    let (state, _) = press(&state, 'p');
    assert_eq!(
        actions(&state),
        [
            TodoAction::Drop,
            TodoAction::Drop,
            TodoAction::Pick,
            TodoAction::Pick
        ]
    );
    assert_eq!(notes(&state), [None, None, seeded.clone(), seeded]);

    // `d` pressed again is his own mark: no words.
    let (state, _) = press(&state, 'k');
    let (state, _) = press(&state, 'd');
    assert_eq!(ed(&state).view_facts().rows[1].marked, None);
    assert_eq!(notes(&state)[1].as_deref(), Some("left out"));
}

#[test]
fn cursor_opens_on_the_newest_drop() {
    let state = editor_from_rebase();
    let facts = ed(&state).view_facts();
    assert_eq!(facts.cursor, 1, "row 1 is commit 4, the newest drop");
    assert_eq!(facts.rows[facts.cursor].commit, oid(4));
}

#[test]
fn autosquash_rows_still_read_marked_by_its_name() {
    let todo = vec![
        line(TodoAction::Pick, 2, "Add OAuth callback"),
        line(TodoAction::Fixup, 4, "fixup! Add OAuth callback"),
        line(TodoAction::Pick, 3, "Handle expired tokens"),
        line(TodoAction::Pick, 5, "wip"),
    ];
    let (state, _) = editor_with(0, todo);
    let drawn = notes(&state);
    assert_eq!(
        drawn[2].as_deref(),
        Some("melds into the row below \u{b7} marked by its name"),
        "{drawn:?}"
    );
    assert!(
        drawn
            .iter()
            .flatten()
            .all(|note| !note.contains("already in")),
        "{drawn:?}"
    );
    assert_eq!(ed(&state).view_facts().cursor, 0, "on the row R was on");
}
