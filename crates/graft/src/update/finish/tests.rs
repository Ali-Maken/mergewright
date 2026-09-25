//! `update::finish` over plain data, driven through `update` the way the
//! loop drives it. The real-git half — a real merge, a real `--continue`, a
//! real hook — is `tests/it/finish.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{StatusEntry, WorkingTreeStatus};
use graft_ops::apply::Resume;
use graft_ops::finish::{ByGit, ByYou, Change, FinishSummary, ResultIs};

use super::*;
use crate::update::{AppEvent, Product, Size, Status, update};

fn detail(kind: InProgress) -> InProgressDetail {
    InProgressDetail {
        kind,
        step: None,
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
        submodule: false,
    }
}

/// A `kind` stopped with `paths` still conflicted, the banner focused, at
/// the harness' 120x40.
fn stopped(kind: InProgress, paths: &[&str]) -> AppState {
    AppState {
        in_progress: Some(detail(kind)),
        banner_focus: true,
        size: Size {
            columns: 120,
            rows: 40,
        },
        working_tree: Some(Arc::new(WorkingTreeStatus {
            entries: paths.iter().map(|path| unmerged(path)).collect(),
        })),
        ..AppState::default()
    }
}

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

fn plain(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn control(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }
}

fn key(state: &AppState, code: KeyCode) -> AppState {
    press(state, plain(code)).0
}

fn typed(state: &AppState, text: &str) -> AppState {
    text.chars().fold(state.clone(), |state, character| {
        key(&state, KeyCode::Char(character))
    })
}

/// git's message for a merge that stopped, comments already gone.
const GITS_MESSAGE: &[u8] = b"Merge branch 'feature'\n";

fn summary() -> Arc<FinishSummary> {
    Arc::new(FinishSummary {
        into: b"main".to_vec(),
        from: b"feature".to_vec(),
        by_you: vec![ByYou {
            path: PathBuf::from("notes.txt"),
            result: ResultIs::Theirs,
        }],
        by_git: vec![ByGit {
            path: PathBuf::from("README.md"),
            change: Change::Modified,
            added: 1,
            removed: 1,
        }],
        message: GITS_MESSAGE.to_vec(),
        recorded: true,
    })
}

/// `c` on a resolved merge's banner, and the summary's read answered.
fn opened() -> AppState {
    let state = stopped(InProgress::Merge, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .expect("`c` asks for the summary");
    update(
        &state,
        AppEvent::Produced(id, Product::FinishSummary(Ok(summary()))),
    )
    .0
}

fn finish(state: &AppState) -> &FinishState {
    state.finish.as_ref().expect("the summary is open")
}

fn resumes(commands: &[Command]) -> Vec<&Resume> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Resume(_, what) => Some(what),
            _ => None,
        })
        .collect()
}

#[test]
fn c_on_a_resolved_merge_opens_the_summary_not_the_commit() {
    let state = stopped(InProgress::Merge, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(
        resumes(&commands).is_empty(),
        "nothing written: {commands:?}"
    );
    assert!(state.operating.is_none());
    let open = finish(&state);
    assert!(
        commands.iter().any(
            |command| matches!(command, Command::ReadFinishSummary(id) if Some(*id) == open.job)
        ),
        "the summary is read under the id it waits on: {commands:?}"
    );
    assert_eq!(open.focus, FinishFocus::Files);
    assert_eq!(
        state.graph_overlay(),
        Some(crate::update::GraphOverlay::Finish)
    );
}

#[test]
fn c_with_a_file_unmerged_is_still_refused() {
    let state = stopped(InProgress::Merge, &["notes.txt"]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(state.finish.is_none());
    assert!(resumes(&commands).is_empty());
    assert_eq!(
        state.status,
        crate::update::banner::continue_refused("1 file still conflicted".to_owned())
    );
}

#[test]
fn a_rebase_c_still_continues_directly() {
    let state = stopped(InProgress::Rebase, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(state.finish.is_none());
    assert_eq!(resumes(&commands), vec![&Resume::Continue]);
}

#[test]
fn the_summary_lands_with_gits_message_and_the_cursor_on_what_git_combined() {
    let state = opened();
    let open = finish(&state);
    assert_eq!(open.subject.text(), "Merge branch 'feature'");
    assert!(open.body.is_empty());
    assert_eq!(open.cursor, 1, "the first `by_git` row");
    let view = crate::view::finish::finish_view(&state).expect("drawn");
    let (from, into) = view.names.as_ref().expect("the read names the sides");
    assert_eq!((from.as_str(), into.as_str()), ("feature", "main"));
    assert!(view.expandable, "the cursor is on a row git combined");
    assert_eq!(view.by_git[0].what.as_str(), "modified  +1 −1");
}

#[test]
fn unread_summary_has_no_names() {
    let state = key(&stopped(InProgress::Merge, &[]), KeyCode::Char('c'));
    let view = crate::view::finish::finish_view(&state).expect("drawn");
    assert_eq!(view.names, None, "no invented names while reading");
    assert_eq!(
        graft_ui::finish_summary::preview_title(None, 120),
        "Preview: finish the merge"
    );
    assert_eq!(
        graft_ui::finish_summary::preview_line(None),
        "1 merge commit · nothing is committed until y"
    );
}

#[test]
fn y_commits_with_gits_message_when_unedited() {
    let state = opened();
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert_eq!(resumes(&commands), vec![&Resume::Continue]);
    assert_eq!(finish(&state).committing, state.operating);
    assert!(state.operating.is_some());
}

#[test]
fn an_edited_subject_commits_continue_with() {
    let state = opened();
    let state = key(&state, KeyCode::Tab);
    let state = typed(&state, " at last");
    let (_, commands) = press(&state, plain(KeyCode::Enter));
    assert_eq!(
        resumes(&commands),
        vec![&Resume::ContinueWith {
            message: b"Merge branch 'feature' at last".to_vec()
        }]
    );
}

#[test]
fn an_empty_subject_is_refused_in_words() {
    let mut state = opened();
    if let Some(open) = state.finish.as_mut() {
        open.subject = TextField::new("");
    }
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert!(resumes(&commands).is_empty());
    assert_eq!(
        finish(&state).notice,
        Some(("the message is empty · ⇥ to write one".to_owned(), true))
    );
}

#[test]
fn y_while_reading_says_wait() {
    let state = stopped(InProgress::Merge, &[]);
    let state = key(&state, KeyCode::Char('c'));
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert!(resumes(&commands).is_empty());
    assert_eq!(
        finish(&state).notice,
        Some(("still reading · wait".to_owned(), false))
    );
}

#[test]
fn an_unreadable_summary_commits_with_gits_message() {
    let state = stopped(InProgress::Merge, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    let Some(Command::ReadFinishSummary(id)) = commands
        .iter()
        .find(|command| matches!(command, Command::ReadFinishSummary(_)))
    else {
        panic!("no read: {commands:?}");
    };
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            *id,
            Product::FinishSummary(Err("1 still conflicted".to_owned())),
        ),
    );
    let (_, commands) = press(&state, plain(KeyCode::Char('y')));
    assert_eq!(resumes(&commands), vec![&Resume::Continue]);
}

#[test]
fn the_editor_answer_for_the_commit_editor_is_unchanged() {
    // The commit editor's own `$EDITOR` round, with a summary open beside
    // it that is waiting on a different id.
    let mut state = opened();
    state.commit_editor = Some(crate::update::commit_editor::CommitEditor::new());
    let commit_editor_id = JobId::new(500);
    state.external_editor = Some(commit_editor_id);
    if let Some(open) = state.finish.as_mut() {
        open.editing = Some(JobId::new(501));
    }
    let before = finish(&state).clone();
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            commit_editor_id,
            Product::ExternalEditor(Ok("Fix login\n\n# a comment\nbody\n".to_owned())),
        ),
    );
    let editor = state.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.subject.text(), "Fix login");
    assert_eq!(
        editor.body.text(),
        "body",
        "the commit editor still strips comment lines"
    );
    assert_eq!(finish(&state), &before, "the summary is untouched");
}

#[test]
fn ctrl_e_hands_the_message_over_and_takes_it_back_unstripped() {
    let state = opened();
    let (state, commands) = press(&state, control(KeyCode::Char('e')));
    let Some(Command::EditCommitMessage { id, message }) = commands
        .iter()
        .find(|command| matches!(command, Command::EditCommitMessage { .. }))
    else {
        panic!("no handover: {commands:?}");
    };
    assert_eq!(message, "Merge branch 'feature'");
    assert_eq!(finish(&state).editing, Some(*id));
    assert_eq!(state.external_editor, Some(*id));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            *id,
            Product::ExternalEditor(Ok("Merge branch 'feature'\n\n#123 fixes login\n".to_owned())),
        ),
    );
    let open = finish(&state);
    assert_eq!(open.body.text(), "#123 fixes login");
    assert_eq!(open.editing, None);
    assert_eq!(state.external_editor, None);
}

#[test]
fn esc_goes_back_and_keeps_the_draft() {
    let state = opened();
    let state = key(&state, KeyCode::Tab);
    let state = typed(&state, " for 2.4");
    let state = key(&state, KeyCode::Escape);
    assert!(state.finish.is_none());
    assert!(state.banner_focus, "back on the banner, focused");
    assert_eq!(
        state.finish_draft,
        Some(("Merge branch 'feature' for 2.4".to_owned(), String::new()))
    );
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(resumes(&commands).is_empty());
    assert_eq!(
        finish(&state).subject.text(),
        "Merge branch 'feature' for 2.4"
    );
    // git's message does not overwrite the draft when the summary lands.
    let id = finish(&state).job.expect("reading");
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::FinishSummary(Ok(summary()))),
    );
    assert_eq!(
        finish(&state).subject.text(),
        "Merge branch 'feature' for 2.4"
    );
}

/// Where `Esc` from the summary lands when the changes pane had focus
/// underneath (m6-conflicts/52, find-and-pin 1). Opened from the banner's
/// `c` — today's `ReturnTo::Banner` path, the one the pane's `c` will take.
#[test]
fn esc_from_a_summary_opened_in_the_pane_lands_on_the_banner() {
    use graft_ui::detail::DetailFocus;
    let mut state = stopped(InProgress::Merge, &[]);
    state.detail.shown = true;
    state.detail.spec = Some(crate::update::detail::DetailSpec::WorkingTree);
    state.detail.focus = DetailFocus::Files;
    assert!(state.on_changes_pane(), "the changes pane is underneath");
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .expect("`c` asks for the summary");
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::FinishSummary(Ok(summary()))),
    );
    assert_eq!(finish(&state).return_to, ReturnTo::Banner);
    let state = key(&state, KeyCode::Tab);
    let state = typed(&state, " for 2.4");
    let state = key(&state, KeyCode::Escape);

    // (a) closed, the draft kept.
    assert!(state.finish.is_none());
    assert_eq!(
        state.finish_draft,
        Some(("Merge branch 'feature' for 2.4".to_owned(), String::new()))
    );
    // (b) the header draws the banner focused.
    let banner = crate::update::banner::banner_view(&state).expect("the banner is drawn");
    assert!(banner.focused);
    // (c) the footer is the banner's.
    assert_eq!(
        crate::view::rows::footer_context(&state, &[], Some(&banner)),
        crate::view::rows::banner_context(&banner)
    );
    // (e) the pane's own focus is left where it was.
    assert_eq!(state.detail.focus, DetailFocus::Files);
    // (d) the next key is the banner's: its `⇥` hands the keyboard to the
    // graph. The pane's `⇥` would have moved Files → Diff.
    let state = key(&state, KeyCode::Tab);
    assert!(!state.banner_focus);
    assert_eq!(state.detail.focus, DetailFocus::Graph);
}

#[test]
fn esc_with_nothing_typed_keeps_no_draft() {
    let state = opened();
    let state = key(&state, KeyCode::Escape);
    assert!(state.finish.is_none());
    assert_eq!(state.finish_draft, None);
}

#[test]
fn space_expands_a_git_combined_file() {
    let state = opened();
    let (state, commands) = press(&state, plain(KeyCode::Char(' ')));
    let reads: Vec<_> = commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadFinishDiff(id, path) => Some((*id, path.clone())),
            _ => None,
        })
        .collect();
    let [(first, path)] = reads.as_slice() else {
        panic!("one diff read: {commands:?}");
    };
    assert_eq!(path, &PathBuf::from("README.md"));
    assert_eq!(finish(&state).expanded, Some((1, Loading::Pending(*first))));

    // `space` again collapses; the first read's answer is now stale.
    let collapsed = key(&state, KeyCode::Char(' '));
    assert_eq!(finish(&collapsed).expanded, None);
    let (after, _) = update(
        &collapsed,
        AppEvent::Produced(*first, Product::FinishDiff(Err("late".to_owned()))),
    );
    assert_eq!(finish(&after).expanded, None, "a stale answer is dropped");

    // Expanded again: only the new read's answer lands.
    let (again, commands) = press(&collapsed, plain(KeyCode::Char(' ')));
    let Some(Command::ReadFinishDiff(second, _)) = commands
        .iter()
        .find(|command| matches!(command, Command::ReadFinishDiff(..)))
    else {
        panic!("a second read: {commands:?}");
    };
    let (again, _) = update(
        &again,
        AppEvent::Produced(*first, Product::FinishDiff(Err("stale".to_owned()))),
    );
    assert_eq!(
        finish(&again).expanded,
        Some((1, Loading::Pending(*second)))
    );
    let (again, _) = update(
        &again,
        AppEvent::Produced(*second, Product::FinishDiff(Err("not staged".to_owned()))),
    );
    assert_eq!(
        finish(&again).expanded,
        Some((1, Loading::Failed("not staged".to_owned())))
    );
}

#[test]
fn keys_outside_2_6b_do_nothing() {
    let state = opened();
    // Arrows, pages and `⇧⇥` are bound since ticket 59
    // (`arrows_and_pages_move_the_file_cursor`, `backtab_steps_focus_back`).
    for key in [
        plain(KeyCode::Home),
        plain(KeyCode::End),
        plain(KeyCode::Char('u')),
        plain(KeyCode::Char('q')),
    ] {
        let (after, commands) = press(&state, key);
        assert_eq!(after, state, "{key:?} changed the state");
        assert!(commands.is_empty(), "{key:?} sent {commands:?}");
    }
}

#[test]
fn ctrl_c_passes_through() {
    let state = opened();
    let mut next = state.clone();
    let mut commands = Vec::new();
    assert!(!finish_key(
        &mut next,
        &mut commands,
        control(KeyCode::Char('c')),
        &mut || JobId::new(9)
    ));
}

#[test]
fn committing_swallows_every_key() {
    let state = opened();
    let state = key(&state, KeyCode::Char('y'));
    for code in [
        KeyCode::Escape,
        KeyCode::Char('j'),
        KeyCode::Tab,
        KeyCode::Enter,
    ] {
        let (after, commands) = press(&state, plain(code));
        assert_eq!(after, state, "{code:?}");
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
    }
}

/// The answer routes (scope 7), over plain data: the real-git ones are in
/// `tests/it/finish.rs`.
fn committing() -> (AppState, JobId) {
    let state = key(&opened(), KeyCode::Char('y'));
    let id = state.operating.expect("the commit runs");
    (state, id)
}

#[test]
fn a_refusal_keeps_the_summary_open_with_the_words() {
    let (state, id) = committing();
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Failed {
                error: "the commit-msg hook said no".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
            })),
        ),
    );
    let open = finish(&state);
    assert_eq!(open.committing, None);
    assert_eq!(
        open.notice,
        Some(("the commit-msg hook said no".to_owned(), true))
    );
    assert_eq!(
        state.status,
        Status::Ready,
        "the notice says it, not the status"
    );
    assert!(state.operating.is_none());
}

#[test]
fn a_stop_with_new_conflicts_closes_and_keeps_the_draft() {
    let (mut state, id) = committing();
    if let Some(open) = state.finish.as_mut() {
        open.subject = TextField::new("Merge feature, reworded");
    }
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Conflicted {
                description: "continue merge".to_owned(),
                kind: InProgress::Merge,
                paths: vec!["notes.txt".into()],
                stalled: None,
                entry: graft_core::journal::EntryId::new(1, [0; 10]),
            })),
        ),
    );
    assert!(state.finish.is_none());
    assert_eq!(
        state.finish_draft,
        Some(("Merge feature, reworded".to_owned(), String::new()))
    );
}

#[test]
fn by_you_words_first_match_wins() {
    use graft_ui::conflict_resolver::SideRoles;
    let names = SideNames {
        ours: graft_term::sanitize_line("yours · main"),
        theirs: graft_term::sanitize_line("theirs · feature"),
        roles: SideRoles::Merge,
    };
    let whole = Choices {
        whole: Some(Side::Ours),
        edited: true,
        ..Choices::default()
    };
    assert_eq!(
        by_you_words(Some(&whole), ResultIs::Deleted, &names),
        "deleted · as yours · main"
    );
    assert_eq!(
        by_you_words(Some(&whole), ResultIs::Yours, &names),
        "kept · yours · main"
    );
    let edited = Choices {
        edited: true,
        takes: vec![Some(Take::Ours)],
        ..Choices::default()
    };
    assert_eq!(
        by_you_words(Some(&edited), ResultIs::Combined, &names),
        "edited in your editor"
    );
    let theirs = Choices {
        takes: vec![Some(Take::Theirs), Some(Take::Theirs)],
        ..Choices::default()
    };
    assert_eq!(
        by_you_words(Some(&theirs), ResultIs::Theirs, &names),
        "theirs · feature in 2 of 2 conflicts"
    );
    let mixed = Choices {
        takes: vec![Some(Take::Both), Some(Take::Theirs), Some(Take::Ours), None],
        ..Choices::default()
    };
    assert_eq!(
        by_you_words(Some(&mixed), ResultIs::Combined, &names),
        "yours · main in 1, theirs · feature in 1, both in 1"
    );
    assert_eq!(
        by_you_words(None, ResultIs::Yours, &names),
        "same as yours · main"
    );
    assert_eq!(
        by_you_words(None, ResultIs::Theirs, &names),
        "same as theirs · feature"
    );
    assert_eq!(by_you_words(None, ResultIs::Deleted, &names), "deleted");
    assert_eq!(by_you_words(None, ResultIs::Combined, &names), "combined");
}

#[test]
fn split_and_compose_round_trip() {
    assert_eq!(
        split_message("Subject\n\nBody line\n#123 kept\n\n"),
        ("Subject".to_owned(), "Body line\n#123 kept".to_owned())
    );
    assert_eq!(
        split_message("Subject\r\nno blank"),
        ("Subject".to_owned(), "no blank".to_owned())
    );
    assert_eq!(compose("Subject", "  \n"), "Subject");
    assert_eq!(compose("Subject", "Body"), "Subject\n\nBody");
}

/// P12 (keymap §2.6b): in the file list `y` commits and `⏎` shows the
/// cursor file's change, as `space` does.
#[test]
fn y_in_the_file_list_commits_and_enter_toggles_the_diff() {
    let state = opened();
    assert_eq!(finish(&state).cursor, 1, "on README.md");
    let (shown, commands) = press(&state, plain(KeyCode::Enter));
    assert!(resumes(&commands).is_empty(), "⏎ does not commit");
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishDiff(id, path) => Some((*id, path.clone())),
            _ => None,
        })
        .expect("⏎ reads the diff");
    assert_eq!(read.1, PathBuf::from("README.md"));
    assert_eq!(finish(&shown).expanded, Some((1, Loading::Pending(read.0))));
    let hidden = key(&shown, KeyCode::Enter);
    assert_eq!(finish(&hidden).expanded, None, "⏎ again hides it");

    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert_eq!(resumes(&commands), vec![&Resume::Continue]);
    assert_eq!(finish(&state).committing, state.operating);
    assert!(state.operating.is_some());
}

#[test]
fn y_in_the_subject_is_typed_text() {
    let state = key(&opened(), KeyCode::Tab);
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert!(resumes(&commands).is_empty());
    assert_eq!(finish(&state).subject.text(), "Merge branch 'feature'y");
    let state = key(&state, KeyCode::Tab);
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert!(resumes(&commands).is_empty());
    assert_eq!(finish(&state).body.text(), "y");
}

#[test]
fn enter_in_the_subject_still_commits() {
    let state = key(&opened(), KeyCode::Tab);
    let (_, commands) = press(&state, plain(KeyCode::Enter));
    assert_eq!(resumes(&commands), vec![&Resume::Continue]);
}

#[test]
fn arrows_and_pages_move_the_file_cursor() {
    let state = opened();
    let up = key(&state, KeyCode::Up);
    assert_eq!(finish(&up).cursor, 0);
    assert_eq!(finish(&key(&up, KeyCode::Down)).cursor, 1);
    assert_eq!(finish(&key(&state, KeyCode::PageUp)).cursor, 0);
    assert_eq!(finish(&key(&up, KeyCode::PageDown)).cursor, 1);

    // A page is the list's visible height.
    let mut long = opened();
    let mut many = (*summary()).clone();
    many.by_git = (0..80)
        .map(|n| ByGit {
            path: PathBuf::from(format!("file{n}.txt")),
            change: Change::Modified,
            added: 1,
            removed: 0,
        })
        .collect();
    if let Some(open) = long.finish.as_mut() {
        open.summary = Loading::Ready(Arc::new(many));
        open.cursor = 0;
    }
    let view = crate::view::finish::finish_view(&long).expect("drawn");
    let height =
        graft_ui::finish_summary::list_height(&view, crate::view::finish::summary_area(&long));
    assert!(height > 2, "{height}");
    let paged = key(&long, KeyCode::PageDown);
    assert_eq!(finish(&paged).cursor, height);
    let back = key(&paged, KeyCode::PageUp);
    assert_eq!(finish(&back).cursor, 0);
    assert_eq!(finish(&back).top, 0, "the heading comes back with it");
}

#[test]
fn backtab_steps_focus_back() {
    let state = opened();
    let shift_tab = KeyPress {
        code: KeyCode::Tab,
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    };
    let body = press(&state, plain(KeyCode::BackTab)).0;
    assert_eq!(finish(&body).focus, FinishFocus::Body);
    let subject = press(&body, shift_tab).0;
    assert_eq!(finish(&subject).focus, FinishFocus::Subject);
    let files = press(&subject, plain(KeyCode::BackTab)).0;
    assert_eq!(finish(&files).focus, FinishFocus::Files);
}
