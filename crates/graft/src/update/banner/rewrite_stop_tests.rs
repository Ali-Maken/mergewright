//! A rewrite's stop (m7-power-tools/07b), over plain data: the banner's
//! words, who gets the keyboard at each of the three places a stop lands,
//! `a` at an `edit` stop, and a progress read that failed.
//!
//! The same over a real repository — a relaunch mid-rewrite, `a` then `c`,
//! `Esc` — is `tests/it/rewrite_stop.rs`.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::journal::EntryId;
use graft_core::ref_name::RefName;
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{StatusEntry, WorkingTreeStatus};
use graft_ops::operation::Operation;
use graft_ops::rewrite::{EditStop, Follower, RewriteProgress, TodoAction, TodoLine};
use graft_ui::graph_screen::banner_line;

use super::*;
use crate::jobs::operate::Operated;
use crate::update::{AppEvent, LiveOrigin, Product, Stop, update};
use crate::view::tests::{history_of, oid, refs_with_head};

const SUBJECT: &str = "wip: tests";

fn rebase() -> InProgressDetail {
    InProgressDetail {
        kind: InProgress::Rebase,
        step: Some((2, 5)),
        head_name: None,
        onto: None,
        theirs: Ok(None),
    }
}

fn unmerged(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: true,
        unmerged_how: None,
        submodule: None,
    }
}

/// What the job thread read at a stop on row `action` of `oid(3)`, step 2
/// of 5; `chain` names the branch of the line, `edit` adds the edit-stop
/// read (07a).
fn read(action: TodoAction, chain: Option<(&str, usize, usize)>, edit: bool) -> RewriteStop {
    RewriteStop {
        progress: RewriteProgress {
            step: (2, 5),
            current: Some(TodoLine {
                action,
                commit: oid(3),
                subject: SUBJECT.to_owned(),
            }),
            pending: Vec::new(),
            branch: chain.map(|(name, index, of)| {
                (
                    RefName::new(format!("refs/heads/{name}").into_bytes()).expect("a ref name"),
                    index,
                    of,
                )
            }),
            begun_in_graft: chain.is_some(),
            followers: Vec::new(),
            fork: None,
        },
        edit: edit.then(|| EditStop {
            stopped: oid(3),
            head_on_it: true,
        }),
    }
}

/// A rebase Graft began (one live entry), stopped with `rewrite` read and
/// `paths` conflicted. `HEAD` is `oid(3)`, the cursor's row; the graph has
/// the keyboard.
fn live(rewrite: Option<Result<RewriteStop, String>>, paths: &[&str]) -> AppState {
    AppState {
        history: Some(history_of(3, 0)),
        refs: Some(Arc::new(refs_with_head(Some(oid(3))))),
        in_progress: Some(rebase()),
        live_entries: vec![EntryId::new(1, [0; 10])],
        working_tree: Some(Arc::new(WorkingTreeStatus {
            entries: paths.iter().map(|path| unmerged(path)).collect(),
        })),
        stopped: Some(Stop {
            origin: LiveOrigin::ThisRun,
            again: false,
            kind: InProgress::Rebase,
            paths_at_stop: paths.len(),
            rewrite,
        }),
        ..AppState::default()
    }
}

pub(in crate::update) fn edit_stop() -> AppState {
    live(Some(Ok(read(TodoAction::Edit, None, true))), &[])
}

fn words(state: &AppState) -> String {
    banner_line(
        &banner_view(state).expect("a rebase is live"),
        u16::MAX,
        &state.keymap,
    )
}

fn key(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Key(KeyPress {
            code,
            modifiers: Modifiers::NONE,
        })),
    )
}

// ── The banner's words ─────────────────────────────────────────────────

#[test]
fn a_chain_stop_names_the_branch_in_the_banner() {
    let state = live(
        Some(Ok(read(TodoAction::Pick, Some(("feature-b", 2, 3)), false))),
        &["src/auth.rs"],
    );
    assert_eq!(
        words(&state),
        "REBASING  step 2 of 5 \u{b7} pick 0303030 wip: tests \u{b7} feature-b, branch 2 of 3"
    );
}

/// A stop inside a fork's section (ADR-0059 §3, m8/14c) names the fork
/// where the line form names the branch, on a line of one branch or more.
#[test]
fn a_stop_inside_a_fork_names_the_fork() {
    for chain in [Some(("feature-b", 2, 3)), None] {
        let mut stop = read(TodoAction::Pick, chain, false);
        stop.progress.fork = Some(Box::new(graft_ops::rewrite::ForkStop {
            name: "feature-x".to_owned(),
            at: oid(0xa1),
        }));
        let state = live(Some(Ok(stop)), &["src/auth.rs"]);
        assert_eq!(
            words(&state),
            "REBASING  step 2 of 5 \u{b7} pick 0303030 wip: tests \u{b7} feature-x, forked at a1a1a1a",
            "{chain:?}"
        );
    }
}

#[test]
fn a_single_branch_stop_has_no_chain_words() {
    let state = live(Some(Ok(read(TodoAction::Pick, None, false))), &["a.rs"]);
    assert_eq!(
        words(&state),
        "REBASING  step 2 of 5 \u{b7} pick 0303030 wip: tests"
    );
}

/// The step counts are the todo's (his rows), not git's `msgnum` from
/// `in_progress`: the two differ once `update-ref` lines are in the todo.
#[test]
fn the_step_is_the_rewrite_s_own_count() {
    let mut state = live(Some(Ok(read(TodoAction::Pick, None, false))), &["a.rs"]);
    state.in_progress = Some(InProgressDetail {
        step: Some((4, 9)),
        ..rebase()
    });
    assert!(words(&state).starts_with("REBASING  step 2 of 5 "));
}

/// A progress read that failed: today's banner, and the status line says
/// why (rule 3 — the error is shown, not swallowed).
#[test]
fn a_failed_progress_read_lands_with_todays_banner_and_says_so() {
    let mut state = AppState {
        in_progress: Some(rebase()),
        live_entries: vec![EntryId::new(1, [0; 10])],
        ..AppState::default()
    };
    let id = crate::update::JobId::new(7);
    state.operating = Some(id);
    crate::update::operate::operated(
        &mut state,
        id,
        Operated::Conflicted {
            rewrite: Some(Err("git-rebase-todo: unreadable".to_owned())),
            description: "rewrite feature".to_owned(),
            kind: InProgress::Rebase,
            paths: vec!["a.rs".into()],
            stalled: None,
            entry: EntryId::new(1, [0; 10]),
        },
        &mut || crate::update::JobId::new(8),
    );
    assert_eq!(words(&state), "REBASING step 2 of 5", "today's banner");
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "couldn't read the rewrite's progress: git-rebase-todo: unreadable".to_owned(),
            keys: None,
        }
    );
    assert!(state.banner_focus, "a stop it can't read focuses as today");
}

// ── Who has the keyboard ───────────────────────────────────────────────

/// The three places a stop lands: the write that stopped, a resume that
/// stopped again, a relaunch's working-tree read. `rewrite` is what each
/// read at the stop; the answer is whether the banner has the keyboard.
fn focus_at_each_site(rewrite: &RewriteStop, paths: &[&str]) -> [bool; 3] {
    let conflicted = |rewrite: &RewriteStop| Operated::Conflicted {
        rewrite: Some(Ok(rewrite.clone())),
        description: "rewrite feature".to_owned(),
        kind: InProgress::Rebase,
        paths: paths.iter().map(Into::into).collect(),
        stalled: None,
        entry: EntryId::new(1, [0; 10]),
    };
    let id = crate::update::JobId::new(7);

    // The first stop: the rewrite's own write answers.
    let mut first = AppState {
        in_progress: Some(rebase()),
        operating: Some(id),
        ..AppState::default()
    };
    crate::update::operate::operated(&mut first, id, conflicted(rewrite), &mut || {
        crate::update::JobId::new(8)
    });
    assert!(first.stopped.is_some(), "the write stopped");

    // Stopped again: `c` resumed and git stopped on a later row.
    let mut again = live(Some(Ok(rewrite.clone())), paths);
    again.banner_focus = true;
    again.operating = Some(id);
    again.resuming = Some((id, Resume::Continue));
    resumed(&mut again, id, conflicted(rewrite), &mut || {
        crate::update::JobId::new(8)
    });
    assert!(again.stopped.as_ref().is_some_and(|stop| stop.again));

    // A relaunch: the first working-tree read finds the rebase.
    let launched = AppState {
        history: Some(history_of(3, 0)),
        live_entries: vec![EntryId::new(1, [0; 10])],
        ..AppState::default()
    };
    let (relaunched, _) = update(
        &launched,
        AppEvent::Produced(
            crate::update::JobId::new(90),
            Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
                rewrite: Some(Ok(rewrite.clone())),
                status: Arc::new(WorkingTreeStatus {
                    entries: paths.iter().map(|path| unmerged(path)).collect(),
                }),
                in_progress: Some(rebase()),
                mergetool: None,
            }))),
        ),
    );
    assert!(relaunched.stopped.is_some(), "the relaunch landed on it");
    assert_eq!(
        relaunched
            .stopped
            .as_ref()
            .and_then(|stop| stop.rewrite.clone()),
        Some(Ok(rewrite.clone())),
        "the relaunch keeps what the read found"
    );

    [
        first.banner_focus,
        again.banner_focus,
        relaunched.banner_focus,
    ]
}

#[test]
fn an_edit_stop_gives_the_keyboard_to_the_graph() {
    assert_eq!(
        focus_at_each_site(&read(TodoAction::Edit, None, true), &[]),
        [false; 3],
        "first stop, stopped again, relaunch"
    );
}

#[test]
fn a_conflict_stop_still_focuses_as_today() {
    assert_eq!(
        focus_at_each_site(&read(TodoAction::Pick, None, false), &["a.rs"]),
        [true; 3],
        "first stop, stopped again, relaunch"
    );
}

#[test]
fn tab_focuses_the_banner_at_an_edit_stop() {
    let state = edit_stop();
    assert!(!state.banner_focus);
    let (tabbed, _) = key(&state, KeyCode::Tab);
    assert!(tabbed.banner_focus, "one ⇥ from the graph");
}

// ── `a` at an `edit` stop ──────────────────────────────────────────────

/// `a` on `HEAD`'s row: `HEAD`'s message is read, and its answer launches
/// the amend with it — no editor. Returns the state after the launch and
/// the amend's job id.
pub(in crate::update) fn a_amends(
    state: &AppState,
    message: &str,
) -> (AppState, crate::update::JobId) {
    let (asked, commands) = key(state, KeyCode::Char('a'));
    let [Command::ReadHeadMessage(read_id)] = commands.as_slice() else {
        panic!("expected HEAD's message to be read: {commands:?}");
    };
    assert!(asked.commit_editor.is_none(), "no editor opens");
    let (launched, commands) = update(
        &asked,
        AppEvent::Produced(*read_id, Product::HeadMessage(Ok(message.to_owned()))),
    );
    assert!(launched.commit_editor.is_none(), "no editor opens");
    let amend = commands.iter().find_map(|command| match command {
        Command::Operate(id, op, ..) => match op.as_ref() {
            Operation::Amend(params) => Some((*id, params.message.clone())),
            _ => None,
        },
        _ => None,
    });
    let (id, kept) = amend.unwrap_or_else(|| panic!("expected an amend: {commands:?}"));
    assert_eq!(kept, message, "the message is kept");
    (launched, id)
}

/// The amend's answer: done, still at the stop.
pub(in crate::update) fn amended(state: &AppState, id: crate::update::JobId) -> AppState {
    let (landed, _) = update(
        state,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(Operated::Done(super::tests::done_applied()))),
        ),
    );
    landed
}

#[test]
fn a_at_an_edit_stop_amends_the_stopped_commit() {
    let (launched, id) = a_amends(&edit_stop(), "wip: tests\n\nThe body stays.\n");
    let landed = amended(&launched, id);
    assert_eq!(
        landed.status,
        Status::Done {
            what: "amended the stopped commit".to_owned(),
            also: None,
            keys: Some("c continues".to_owned()),
        }
    );
    assert!(landed.stopped.is_some(), "the rewrite is still stopped");
}

#[test]
fn a_twice_at_an_edit_stop_amends_twice() {
    let (launched, id) = a_amends(&edit_stop(), SUBJECT);
    let landed = amended(&launched, id);
    let (again, second) = a_amends(&landed, SUBJECT);
    assert_ne!(id, second, "a second amend, its own job");
    let landed = amended(&again, second);
    assert!(matches!(landed.status, Status::Done { .. }));
}

/// App side: after the amend the rewrite's entry is still the live one and
/// `c` resumes it. (The entry `resume_target` finds on disk is graft-ops'
/// `an_amend_inside_graft_s_rewrite_folds_and_c_still_continues_it`.)
#[test]
fn c_after_an_amend_at_an_edit_stop_continues_the_rewrite() {
    let (launched, id) = a_amends(&edit_stop(), SUBJECT);
    let landed = amended(&launched, id);
    assert_eq!(landed.live_entries, vec![EntryId::new(1, [0; 10])]);
    // The landing re-reads the files; `c` waits for that answer (its
    // `checking the files`), which comes back still at the stop.
    let read_id = landed.status_read.expect("the landing re-reads the files");
    let (reread, _) = update(
        &landed,
        AppEvent::Produced(
            read_id,
            Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
                rewrite: Some(Ok(read(TodoAction::Edit, None, true))),
                status: Arc::new(WorkingTreeStatus {
                    entries: Vec::new(),
                }),
                in_progress: Some(rebase()),
                mergetool: None,
            }))),
        ),
    );
    let (_, commands) = key(&reread, KeyCode::Char('c'));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Resume(_, Resume::Continue, ..))),
        "{commands:?}"
    );
}

/// A conflict stop is not an `edit` stop: `a` is refused in words, as
/// every live step is (P14), and nothing is read or run. The ops half is
/// 07a's `edit_stop_is_none_at_a_conflict_stop`.
#[test]
fn a_at_a_conflict_stop_is_still_refused() {
    let state = live(Some(Ok(read(TodoAction::Pick, None, false))), &["a.rs"]);
    let (refused, commands) = key(&state, KeyCode::Char('a'));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadHeadMessage(_) | Command::Operate(..))),
        "{commands:?}"
    );
    let Status::Sticky { reason, .. } = &refused.status else {
        panic!("expected a refusal in words: {:?}", refused.status);
    };
    assert!(reason.contains("rebase"), "{reason}");
}

/// Off `HEAD`'s row `a` is not the stop's amend.
#[test]
fn a_off_head_at_an_edit_stop_is_not_the_stop_s_amend() {
    let mut state = edit_stop();
    state.history = Some(history_of(3, 1));
    let (_, commands) = key(&state, KeyCode::Char('a'));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(_, op, ..) if matches!(op.as_ref(), Operation::Amend(_)))),
        "{commands:?}"
    );
}

/// Decided "Edit stop": the cursor on `HEAD`. At a relaunch with the refs
/// and rows in, at once; at a write's stop, owed to its refresh.
#[test]
fn an_edit_stop_lands_the_cursor_on_head() {
    let launched = AppState {
        history: Some(history_of(3, 2)),
        refs: Some(Arc::new(refs_with_head(Some(oid(3))))),
        live_entries: vec![EntryId::new(1, [0; 10])],
        ..AppState::default()
    };
    let working_tree = |rewrite: RewriteStop, paths: &[&str]| {
        AppEvent::Produced(
            crate::update::JobId::new(90),
            Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
                rewrite: Some(Ok(rewrite)),
                status: Arc::new(WorkingTreeStatus {
                    entries: paths.iter().map(|path| unmerged(path)).collect(),
                }),
                in_progress: Some(rebase()),
                mergetool: None,
            }))),
        )
    };
    let (landed, _) = update(
        &launched,
        working_tree(read(TodoAction::Edit, None, true), &[]),
    );
    let history = landed.history.as_ref().expect("rows");
    assert_eq!(history.cursor, 0, "HEAD's row");
    assert!(!landed.on_working_tree);
    assert_eq!(
        landed.detail.focus,
        graft_ui::detail::DetailFocus::Graph,
        "the graph has the keyboard"
    );
    // A conflict stop lands on the work, as it did.
    let (conflict, _) = update(
        &launched,
        working_tree(read(TodoAction::Pick, None, false), &["a.rs"]),
    );
    assert!(conflict.on_working_tree);

    // The write that stopped: owed to its refresh's refs and walk.
    let id = crate::update::JobId::new(7);
    let mut first = AppState {
        in_progress: Some(rebase()),
        operating: Some(id),
        ..AppState::default()
    };
    crate::update::operate::operated(
        &mut first,
        id,
        Operated::Conflicted {
            rewrite: Some(Ok(read(TodoAction::Edit, None, true))),
            description: "rewrite feature".to_owned(),
            kind: InProgress::Rebase,
            paths: Vec::new(),
            stalled: None,
            entry: EntryId::new(1, [0; 10]),
        },
        &mut || crate::update::JobId::new(8),
    );
    assert!(first.land_on_head, "owed to the refresh");
}

// ── The still-to-apply block (07d D2) ──────────────────────────────────

/// A pick row of `oid(n)` with subject `subject`.
fn pick(n: u8, subject: &str) -> TodoLine {
    TodoLine {
        action: TodoAction::Pick,
        commit: oid(n),
        subject: subject.to_owned(),
    }
}

#[test]
fn pending_view_lists_this_branchs_rows_and_summarises_followers() {
    let mut rewrite = read(TodoAction::Edit, Some(("feature-a", 1, 3)), true);
    // His a3 a4, then Sara's b1 b2, then Lina's c1: the line runs bottom
    // up, so the followers' rows are `pending`'s last.
    rewrite.progress.pending = vec![
        pick(10, "a3"),
        pick(11, "a4"),
        pick(20, "b1"),
        pick(21, "b2"),
        pick(30, "c1"),
    ];
    rewrite.progress.followers = vec![
        Follower {
            branch: "feature-b".to_owned(),
            tip_author: "Sara".to_owned(),
            commits: 2,
        },
        Follower {
            branch: "feature-c".to_owned(),
            tip_author: "Lina".to_owned(),
            commits: 1,
        },
    ];
    let state = live(Some(Ok(rewrite)), &[]);

    let view = pending_view(&state).expect("the block");

    let subjects: Vec<&str> = view.rows.iter().map(|row| row.subject.as_str()).collect();
    assert_eq!(subjects, ["a4", "a3"], "his rows only, newest first");
    let theirs: Vec<String> = [20, 21, 30]
        .iter()
        .map(|n| oid(*n).to_string().chars().take(7).collect())
        .collect();
    assert!(
        view.rows.iter().all(|row| !theirs.contains(&row.sha7)),
        "no follower's sha7 among the rows: {view:?}"
    );
    assert_eq!(view.more, 0);
    assert_eq!(
        view.followers,
        [
            ("feature-b".to_owned(), "Sara".to_owned(), 2),
            ("feature-c".to_owned(), "Lina".to_owned(), 1),
        ]
    );
}
