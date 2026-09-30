//! `a` on an older commit, absorb, their landings and the row tags, over
//! scripted state (m7-power-tools/09). What the plan and the rewrite *do*
//! is graft-ops' and is proved against real git there and in
//! `tests/it/m7_walk/amend_into.rs`.

use std::collections::BTreeMap;
use std::sync::Arc;

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_graph::Row;
use graft_ops::operation::{Operation, Refusal, RewriteKey, Tip};
use graft_ops::preview::{
    ConflictForecast, PreviewPlan, RefMove, Target, UndoNote, WorktreeEffect,
};
use graft_ops::rewrite::{BranchLine, RewriteParams, RewriteRequest, TodoAction, TodoLine};
use graft_ui::context_menu::ContextAction;
use graft_ui::graph_screen::RowTag;

use crate::history::{HistoryUpdate, Window};
use crate::jobs::rewrite::{
    BoxWords, DoneWords, PUT_IT_IN, PUT_THEM_IN, PlannedPutIn, RewritePlanned,
};
use crate::update::preview::{Planned, preview_box, preview_produced};
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

/// `feature` at `HEAD` (commit 3); `others` are more local branches.
fn refs(others: &[(&[u8], u8)]) -> crate::refs::RefBadges {
    let mut all = vec![(name(b"refs/heads/feature"), oid(3))];
    all.extend(others.iter().map(|(full, at)| (name(full), oid(*at))));
    crate::refs::badges(
        &all,
        &HeadState::Branch(name(b"refs/heads/feature")),
        &std::collections::HashMap::new(),
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

fn staged(paths: &[&str]) -> Arc<WorkingTreeStatus> {
    Arc::new(WorkingTreeStatus {
        entries: paths
            .iter()
            .map(|path| StatusEntry {
                path: (*path).into(),
                original_path: None,
                staged: Some(Change::Modified),
                unstaged: None,
                untracked: false,
                unmerged: false,
                unmerged_how: None,
                submodule: None,
            })
            .collect(),
    })
}

/// A 187x50 screen, three rows (3 → 2 → 1), `feature` at 3, `src/auth.rs`
/// staged, the cursor on `row` (0 is `HEAD`).
fn on_screen(at: usize, others: &[(&[u8], u8)]) -> AppState {
    let (state, _) = crate::update::opening(
        std::path::PathBuf::from("/tmp/graft-amend-into-tests"),
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
        rows: vec![row(3, &[2]), row(2, &[1]), row(1, &[0])].into(),
        laid_out: 3,
        done: true,
        promoted: false,
    };
    let (mut state, _) = update(
        &state,
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    state.refs = Some(Arc::new(refs(others)));
    state.working_tree = Some(staged(&["src/auth.rs"]));
    for _ in 0..at {
        state = update(&state, key('j')).0;
    }
    state
}

fn key(c: char) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(c))))
}

fn ids() -> impl FnMut() -> JobId {
    let mut next = 1000;
    move || {
        next += 1;
        JobId::new(next)
    }
}

/// The plan job `commands` start, if any.
fn plan_job(commands: &[Command]) -> Option<(JobId, &RewriteRequest, Option<&Tip>)> {
    commands.iter().find_map(|command| match command {
        Command::PlanRewrite { id, request, head } => Some((*id, request, head.as_ref())),
        _ => None,
    })
}

fn reads_head_message(commands: &[Command]) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, Command::ReadHeadMessage(_)))
}

/// Fixup of commit 2 made as commit 50; commit 3 replayed on top.
fn params() -> RewriteParams {
    let line = |action, n: u8| TodoLine {
        action,
        commit: oid(n),
        subject: format!("commit {n}"),
    };
    RewriteParams {
        head: Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        },
        onto: oid(1),
        todo: vec![
            line(TodoAction::Pick, 2),
            line(TodoAction::Fixup, 50),
            line(TodoAction::Pick, 3),
        ],
        line: BranchLine::default(),
        messages: BTreeMap::new(),
        made: vec![oid(50)],
        index_base: None,
    }
}

fn put_in(offsets: Vec<u32>, confirm: &'static str) -> RewritePlanned {
    RewritePlanned::Plan(Box::new(PlannedPutIn {
        params: params(),
        offsets,
        words: DoneWords {
            what: "put into \"commit 2\"".to_owned(),
            also: None,
        },
        shown: BoxWords {
            title: "put your staged changes into 0000000 \"commit 2\"".to_owned(),
            confirm,
            rows: vec!["0000000  commit 2  gains  src/auth.rs +4 \u{2212}1".to_owned()],
        },
    }))
}

fn a_plan() -> PreviewPlan {
    PreviewPlan {
        files: None,
        description: "rewrite feature".to_owned(),
        commands: vec!["git rebase --interactive 0000001".to_owned()],
        ghosts: Vec::new(),
        ref_moves: vec![RefMove {
            name: "refs/heads/feature".to_owned(),
            from: Some(oid(3)),
            to: Some(Target::Real(oid(60))),
        }],
        head: None,
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
        already_in: None,
        published: Vec::new(),
        messages: Vec::new(),
    }
}

fn answer(
    state: &AppState,
    id: JobId,
    planned: Result<RewritePlanned, String>,
) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Produced(id, Product::RewritePlanned(planned)),
    )
}

/// `a` on the older commit (row 1), answered with a plan, the preview's
/// own job answered too.
fn previewed() -> AppState {
    previewed_from(&on_screen(1, &[]))
}

/// `previewed`, from `state`.
fn previewed_from(state: &AppState) -> AppState {
    let (state, commands) = update(state, key('a'));
    let (id, _, _) = plan_job(&commands).expect("a plan job");
    let (mut state, _) = answer(&state, id, Ok(put_in(vec![1], PUT_IT_IN)));
    let job = state.preview.as_ref().expect("the preview opened").job;
    preview_produced(
        &mut state,
        job,
        Ok(Arc::new(Planned {
            plan: a_plan(),
            anchors: Vec::new(),
            runs: None,
        })),
    );
    state
}

fn operate_job(commands: &[Command]) -> Option<(JobId, &Operation)> {
    commands.iter().find_map(|command| match command {
        Command::Operate(id, op) => Some((*id, op.as_ref())),
        _ => None,
    })
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
            worktree_raw_before: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: std::collections::BTreeMap::new(),
            config_after: std::collections::BTreeMap::new(),
            put_back: None,
        },
        refresh: graft_ops::apply::Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

/// `previewed`, `y`, and the write's `Done`.
fn landed() -> (AppState, Vec<Command>) {
    let (state, commands) = update(&previewed(), key('y'));
    let (id, _) = operate_job(&commands).expect("y launches");
    let mut state = state;
    let commands = crate::update::operate::operated(
        &mut state,
        id,
        crate::jobs::operate::Operated::Done(applied()),
        &mut ids(),
    );
    (state, commands)
}

fn tag_job(commands: &[Command]) -> Option<(JobId, &[u32])> {
    commands.iter().find_map(|command| match command {
        Command::ResolveRowTags(id, offsets) => Some((*id, offsets.as_slice())),
        _ => None,
    })
}

fn sticky(state: &AppState) -> &str {
    match &state.status {
        Status::Sticky { reason, .. } => reason,
        other => panic!("not a sticky line: {other:?}"),
    }
}

// ── `a`'s routes ───────────────────────────────────────────────────────

/// Ruling 1's red: `a` on an older commit used to read `HEAD`'s message
/// and open the amend editor while the cursor marked another commit.
#[test]
fn a_on_an_older_commit_opens_the_put_it_in_preview() {
    let state = on_screen(1, &[]);
    let (state, commands) = update(&state, key('a'));
    assert!(
        !reads_head_message(&commands),
        "not today's amend: {commands:?}"
    );
    let (id, request, head) = plan_job(&commands).expect("a plan job for the cursor's commit");
    assert_eq!(request, &RewriteRequest::AmendInto { target: oid(2) });
    assert_eq!(head, None);
    assert_eq!(
        state.status,
        Status::Waiting("checking where your staged changes go\u{2026}".to_owned())
    );

    let (state, _) = answer(&state, id, Ok(put_in(vec![1], PUT_IT_IN)));
    let preview = state.preview.as_ref().expect("the preview opened");
    assert_eq!(
        preview.subject.operation(),
        Some(&Operation::Rewrite(params()))
    );
    let view = preview_box(&state).expect("a box");
    assert_eq!(
        view.title.as_str(),
        "Preview: put your staged changes into 0000000 \"commit 2\""
    );
}

/// No other local branch: nothing can be built on his, so `a` on `HEAD`
/// is exactly today's amend — no job, `HEAD`'s message read at once.
#[test]
fn a_on_head_without_dependents_is_todays_amend() {
    let (_, commands) = update(&on_screen(0, &[]), key('a'));
    assert!(reads_head_message(&commands), "{commands:?}");
    assert!(plan_job(&commands).is_none());

    // With another branch, a check that finds nothing built on his: the
    // same editor, once the answer is in.
    let (state, commands) = update(&on_screen(0, &[(b"refs/heads/main", 1)]), key('a'));
    let (id, _, _) = plan_job(&commands).expect("the dependents check");
    let (_, commands) = answer(&state, id, Ok(RewritePlanned::NothingBuiltOnHead));
    assert!(reads_head_message(&commands), "{commands:?}");
}

/// Refs not read yet: `HEAD`'s row cannot be told from an older one, so
/// `a` on the top row is today's amend, not a put-in (review Minor 3).
#[test]
fn a_on_head_with_refs_unread_is_todays_amend() {
    let mut state = on_screen(0, &[]);
    state.refs = None;
    let (_, commands) = update(&state, key('a'));
    assert!(plan_job(&commands).is_none(), "{commands:?}");
    assert!(reads_head_message(&commands), "{commands:?}");
}

#[test]
fn a_on_head_with_a_dependent_previews() {
    let state = on_screen(0, &[(b"refs/heads/feature-b", 3)]);
    let (state, commands) = update(&state, key('a'));
    let (id, request, head) = plan_job(&commands).expect("the dependents check");
    assert_eq!(request, &RewriteRequest::AmendInto { target: oid(3) });
    assert_eq!(
        head,
        Some(&Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        })
    );
    assert_eq!(
        state.status,
        Status::Waiting("checking what's built on feature\u{2026}".to_owned())
    );
    let (state, _) = answer(&state, id, Ok(put_in(Vec::new(), PUT_IT_IN)));
    assert!(
        state.preview.is_some(),
        "the preview carries feature-b along"
    );
}

#[test]
fn a_second_a_while_checking_starts_nothing() {
    let (state, _) = update(&on_screen(1, &[]), key('a'));
    let (state, commands) = update(&state, key('a'));
    assert!(plan_job(&commands).is_none(), "{commands:?}");
    assert!(!reads_head_message(&commands));
    assert!(state.amend_into.checking());
}

#[test]
fn a_off_the_branch_is_refused_in_words() {
    let (state, commands) = update(&on_screen(1, &[]), key('a'));
    let (id, _, _) = plan_job(&commands).expect("a plan job");
    let refusal = Refusal::NotOnYourBranch {
        commit: oid(2),
        by: RewriteKey::AmendInto,
    };
    let (state, _) = answer(&state, id, Ok(RewritePlanned::Refused(refusal.clone())));
    assert_eq!(sticky(&state), refusal.explain());
    assert!(
        sticky(&state).starts_with("a puts staged changes into a commit on your branch"),
        "{}",
        sticky(&state)
    );
    assert!(state.preview.is_none());
}

#[test]
fn a_with_nothing_staged_is_refused_in_words() {
    let mut state = on_screen(1, &[]);
    state.working_tree = Some(staged(&[]));
    let (state, commands) = update(&state, key('a'));
    assert!(plan_job(&commands).is_none(), "no job for nothing");
    assert_eq!(
        sticky(&state),
        "nothing staged to put into 0000000 \u{b7} space stages"
    );
}

/// 06e's carry: the planner refuses a pushed target; the words reach the
/// status line whole.
#[test]
fn a_on_a_pushed_older_commit_is_refused_in_words() {
    let (state, commands) = update(&on_screen(1, &[]), key('a'));
    let (id, _, _) = plan_job(&commands).expect("a plan job");
    let refusal = Refusal::AlreadyPushed { commit: oid(2) };
    let (state, _) = answer(&state, id, Ok(RewritePlanned::Refused(refusal)));
    assert_eq!(
        sticky(&state),
        "0000000 is already pushed \u{b7} a only changes older commits you haven't pushed"
    );
}

#[test]
fn a_during_an_operation_keeps_p14() {
    for at in [0, 1] {
        let mut state = on_screen(at, &[(b"refs/heads/feature-b", 3)]);
        state.in_progress = Some(InProgressDetail {
            kind: InProgress::Merge,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        });
        let expected = {
            let mut probe = state.clone();
            let live = crate::update::commit_editor::live_operation(&probe).expect("live");
            let tabs = crate::update::commit_editor::refusal_tabs(&probe);
            probe.status = crate::update::commit_editor::operation_refusal(live, tabs).status();
            probe.status
        };
        let (state, commands) = update(&state, key('a'));
        assert!(plan_job(&commands).is_none());
        assert!(!reads_head_message(&commands));
        assert_eq!(state.status, expected, "row {at}");
    }
}

// ── absorb ─────────────────────────────────────────────────────────────

fn absorb_asked() -> (AppState, JobId) {
    let mut state = on_screen(0, &[]);
    let commands = crate::update::context_menu::apply_context_action(
        &mut state,
        ContextAction::Absorb,
        0,
        None,
        &[],
        &mut ids(),
    );
    let (id, request, _) = plan_job(&commands).expect("absorb plans");
    assert_eq!(request, &RewriteRequest::Absorb);
    (state, id)
}

/// Absorb during a live operation is refused up front, in `a`'s words
/// (P14), before any job (review Minor 4).
#[test]
fn absorb_during_an_operation_keeps_p14() {
    let mut state = on_screen(0, &[]);
    state.in_progress = Some(InProgressDetail {
        kind: InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    let (after_a, _) = update(&state, key('a'));
    let commands = crate::update::context_menu::apply_context_action(
        &mut state,
        ContextAction::Absorb,
        0,
        None,
        &[],
        &mut ids(),
    );
    assert!(plan_job(&commands).is_none(), "{commands:?}");
    assert!(!state.amend_into.checking());
    assert_eq!(state.status, after_a.status);
}

#[test]
fn absorb_opens_its_preview() {
    let (state, id) = absorb_asked();
    let (state, _) = answer(&state, id, Ok(put_in(vec![1], PUT_THEM_IN)));
    let preview = state.preview.as_ref().expect("the absorb preview");
    assert!(matches!(
        preview.subject,
        crate::update::preview::PreviewSubject::Operation { opened_by: ".", .. }
    ));
}

#[test]
fn absorb_with_nothing_placeable_is_refused_in_words() {
    let (state, id) = absorb_asked();
    let (state, _) = answer(
        &state,
        id,
        Ok(RewritePlanned::Refused(Refusal::NothingToAbsorb)),
    );
    assert_eq!(
        sticky(&state),
        "none of your staged lines belong to a commit on your branch \u{b7} c commits them"
    );
    assert!(state.preview.is_none());
}

// ── the box, `y`, the landing ──────────────────────────────────────────

/// The put-in's rows lead the box, and its footer names the confirm by
/// its verb; `y` runs the very rewrite that was planned.
#[test]
fn y_launches() {
    let state = previewed();
    let view = preview_box(&state).expect("a box");
    assert_eq!(
        view.lines.first().map(graft_term::SanitizedText::as_str),
        Some("0000000  commit 2  gains  src/auth.rs +4 \u{2212}1")
    );
    assert_eq!(
        view.again
            .as_ref()
            .map(|(key, verb)| (key.as_str(), verb.as_str())),
        Some(("y", "put it in"))
    );
    let (state, commands) = update(&state, key('y'));
    let (_, op) = operate_job(&commands).expect("y launches");
    assert_eq!(op, &Operation::Rewrite(params()));
    assert!(state.preview.is_none());
}

#[test]
fn landing_words_are_done_line_shaped() {
    let (state, _) = landed();
    let Status::Done { what, also, keys } = &state.status else {
        panic!("not done: {:?}", state.status);
    };
    assert_eq!(
        crate::view::status::done_line(what, also.as_deref(), keys.as_deref()),
        "put into \"commit 2\" \u{2713} \u{b7} u undoes"
    );
}

/// `asked` is for the planned rewrite only: another rewrite landing (the
/// `R` editor's) keeps its own words and tags nothing.
#[test]
fn an_r_editor_rewrite_does_not_take_amend_into_words() {
    let mut state = previewed();
    let mut other = params();
    other.todo.truncate(1);
    let mut commands = Vec::new();
    crate::update::amend_into::landed(
        &mut state,
        Some(&Operation::Rewrite(other)),
        &mut commands,
        &mut ids(),
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert!(!matches!(state.status, Status::Done { .. }));
    assert!(
        state
            .amend_into
            .shown_for(&Operation::Rewrite(params()))
            .is_some()
    );
}

// ── the row tags ───────────────────────────────────────────────────────

#[test]
fn the_target_row_is_tagged_changed_after_landing() {
    let (state, commands) = landed();
    let (id, offsets) = tag_job(&commands).expect("the tag read");
    assert_eq!(offsets, &[1]);
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::RowTagsResolved(Ok(vec![oid(2)]))),
    );
    assert_eq!(state.row_tags.on(oid(2)), Some(&RowTag::Changed));
    assert_eq!(state.row_tags.on(oid(3)), None, "the target's row only");
}

#[test]
fn absorb_tags_every_target() {
    let (state, id) = absorb_asked();
    let (state, _) = answer(&state, id, Ok(put_in(vec![2, 0], PUT_THEM_IN)));
    let mut state = state;
    let job = state.preview.as_ref().expect("preview").job;
    preview_produced(
        &mut state,
        job,
        Ok(Arc::new(Planned {
            plan: a_plan(),
            anchors: Vec::new(),
            runs: None,
        })),
    );
    let (mut state, commands) = update(&state, key('y'));
    let (op_id, _) = operate_job(&commands).expect("y launches");
    let commands = crate::update::operate::operated(
        &mut state,
        op_id,
        crate::jobs::operate::Operated::Done(applied()),
        &mut ids(),
    );
    let (id, offsets) = tag_job(&commands).expect("the tag read");
    assert_eq!(offsets, &[2, 0]);
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::RowTagsResolved(Ok(vec![oid(1), oid(3)]))),
    );
    assert_eq!(state.row_tags.on(oid(1)), Some(&RowTag::Changed));
    assert_eq!(state.row_tags.on(oid(3)), Some(&RowTag::Changed));
}

#[test]
fn the_tags_clear_when_the_next_write_is_claimed() {
    let (state, commands) = landed();
    let (id, _) = tag_job(&commands).expect("the tag read");
    let (tagged, _) = update(
        &state,
        AppEvent::Produced(id, Product::RowTagsResolved(Ok(vec![oid(2)]))),
    );
    assert!(tagged.row_tags.on(oid(2)).is_some());

    let mut next = tagged.clone();
    let claimed = crate::update::operate::claim_write(&mut next, JobId::new(9_999));
    assert!(claimed.is_ok());
    assert_eq!(next.row_tags.on(oid(2)), None, "any write clears them");

    // Undo included: `u` claims its write the same way.
    let (undone, commands) = update(&tagged, key('u'));
    assert!(
        undone.operating.is_some(),
        "u launched: {commands:?} {:?}",
        undone.status
    );
    assert_eq!(undone.row_tags.on(oid(2)), None);
}

#[test]
fn a_failed_tag_read_says_so_in_the_status() {
    let (state, commands) = landed();
    let (id, _) = tag_job(&commands).expect("the tag read");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::RowTagsResolved(Err("rev-list failed".to_owned())),
        ),
    );
    let Status::Done { what, also, keys } = &state.status else {
        panic!("the landing stands: {:?}", state.status);
    };
    assert_eq!(
        crate::view::status::done_line(what, also.as_deref(), keys.as_deref()),
        "put into \"commit 2\" \u{2713} \u{b7} couldn't mark the changed commit: rev-list failed \u{b7} u undoes"
    );
    assert_eq!(state.row_tags.on(oid(2)), None);
}

// ── 09b Q3: the landing composes with the put-back ─────────────────────

/// `state`'s working tree plus `src/tokens.rs` edited and not staged.
fn with_unstaged(mut state: AppState) -> AppState {
    let mut tree = state
        .working_tree
        .as_deref()
        .cloned()
        .expect("a working tree on screen");
    tree.entries.push(StatusEntry {
        path: "src/tokens.rs".into(),
        original_path: None,
        staged: None,
        unstaged: Some(Change::Modified),
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    });
    state.working_tree = Some(Arc::new(tree));
    state
}

/// `state` (a put-in's preview on screen), `y`, and the write's `Done`;
/// the status line as drawn.
fn landing_line(state: &AppState) -> String {
    let (mut state, commands) = update(state, key('y'));
    let (id, _) = operate_job(&commands).expect("y launches");
    crate::update::operate::operated(
        &mut state,
        id,
        crate::jobs::operate::Operated::Done(applied()),
        &mut ids(),
    );
    let Status::Done { what, also, keys } = &state.status else {
        panic!("not done: {:?}", state.status);
    };
    crate::view::status::done_line(what, also.as_deref(), keys.as_deref())
}

#[test]
fn a_put_in_over_unstaged_edits_says_they_are_back() {
    assert_eq!(
        landing_line(&previewed_from(&with_unstaged(on_screen(1, &[])))),
        "put into \"commit 2\" \u{2713} \u{b7} your unstaged edits are back \u{b7} u undoes"
    );
}

#[test]
fn a_put_in_with_nothing_unstaged_says_nothing_about_edits() {
    assert_eq!(
        landing_line(&previewed()),
        "put into \"commit 2\" \u{2713} \u{b7} u undoes"
    );
}

#[test]
fn absorb_over_unstaged_edits_says_they_are_back() {
    let (state, id) = absorb_asked();
    let planned = RewritePlanned::Plan(Box::new(PlannedPutIn {
        params: params(),
        offsets: Vec::new(),
        words: DoneWords {
            what: "put 2 hunks into 2 commits".to_owned(),
            also: Some("1 stays staged".to_owned()),
        },
        shown: BoxWords {
            title: "put staged changes into the commits they belong to".to_owned(),
            confirm: PUT_THEM_IN,
            rows: Vec::new(),
        },
    }));
    let (mut state, _) = answer(&with_unstaged(state), id, Ok(planned));
    let job = state.preview.as_ref().expect("preview").job;
    preview_produced(
        &mut state,
        job,
        Ok(Arc::new(Planned {
            plan: a_plan(),
            anchors: Vec::new(),
            runs: None,
        })),
    );
    assert_eq!(
        landing_line(&state),
        "put 2 hunks into 2 commits \u{2713} \u{b7} 1 stays staged \u{b7} your unstaged edits are back \u{b7} u undoes"
    );
}

// ── landings that are not `Done` (review m7-09-to-21c, Minor 1) ─────────

/// `previewed`, `y`, and the write's answer `outcome`, with the op `y` ran.
fn answered(outcome: crate::jobs::operate::Operated) -> (AppState, Vec<Command>, Operation) {
    let (state, commands) = update(&previewed(), key('y'));
    let (id, op) = operate_job(&commands).expect("y launches");
    let op = op.clone();
    let mut state = state;
    let commands = crate::update::operate::operated(&mut state, id, outcome, &mut ids());
    (state, commands, op)
}

/// The put-in ran and was read back; only the staging did not come back.
/// It is a landing like `Done`: the changed commit is tagged and the plan's
/// words are spent — and the staging failure stays on the line.
#[test]
fn staging_not_kept_after_a_put_in_tags_and_spends_its_words() {
    let (state, commands, op) = answered(crate::jobs::operate::Operated::StagingNotKept {
        applied: applied(),
        paths: vec!["src/auth.rs".into()],
    });
    assert!(tag_job(&commands).is_some(), "the changed commit is tagged");
    assert_eq!(state.amend_into.shown_for(&op), None, "its words are spent");
    assert!(sticky(&state).contains("src/auth.rs"), "{:?}", state.status);
}

/// It ran, but its after-state could not be read: the plan's words are
/// forgotten, not left for a later write to pick up.
#[test]
fn incomplete_after_a_put_in_forgets_its_words() {
    let (state, _, op) = answered(crate::jobs::operate::Operated::Incomplete {
        applied: applied(),
        error: "the refs did not read".to_owned(),
    });
    assert_eq!(
        state.amend_into.shown_for(&op),
        None,
        "its words are forgotten"
    );
    assert!(
        sticky(&state).contains("could not confirm"),
        "{:?}",
        state.status
    );
}

/// Review m7-09-to-21c, Minor 2: a write that starts while a put-in's tag
/// read is out ends that read — its answer, coming back after, names rows
/// of the history before the write and must tag nothing.
#[test]
fn a_write_claimed_while_the_tags_are_read_drops_their_answer() {
    let mut state = AppState::default();
    let tagging = JobId::new(40);
    state.amend_into.tagging = Some(tagging);
    crate::update::operate::claim_write(&mut state, JobId::new(41))
        .unwrap_or_else(|busy| panic!("nothing else is running: {busy:?}"));
    super::tags_resolved(&mut state, tagging, Ok(vec![Oid::from_sha1([2; 20])]));
    assert_eq!(state.row_tags.on(Oid::from_sha1([2; 20])), None);
}
