//! Unit tests for the operation banner's pure half.
//!
//! The banner's behaviour over the real job pool — a real conflict, a real
//! `--continue`, a real abort — is `tests/it/conflict_interim.rs`. What is
//! here is the part that is a function over plain data: which keys the
//! modal claims, what the refusals say, and what the header draws.

use graft_core::event::Modifiers;
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{StatusEntry, WorkingTreeStatus};

use super::*;

fn detail(kind: InProgress, step: Option<(u32, u32)>) -> InProgressDetail {
    InProgressDetail {
        kind,
        step,
        head_name: None,
        onto: None,
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
        submodule: false,
    }
}

/// A state standing in a `kind` with `paths` conflicted, banner focused.
fn stopped(kind: InProgress, paths: &[&str]) -> AppState {
    AppState {
        in_progress: Some(detail(kind, Some((3, 7)))),
        banner_focus: true,
        working_tree: Some(std::sync::Arc::new(WorkingTreeStatus {
            entries: paths.iter().map(|path| unmerged(path)).collect(),
        })),
        ..AppState::default()
    }
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn press(state: &mut AppState, code: KeyCode) -> (bool, Vec<Command>) {
    let mut commands = Vec::new();
    let mut next = 1u64;
    let claimed = banner_key(state, &mut commands, key(code), &mut || {
        next += 1;
        JobId::new(next)
    });
    (claimed, commands)
}

#[test]
fn the_banner_names_the_kind_the_step_and_the_first_conflict() {
    let state = stopped(InProgress::Rebase, &["src/auth.rs", "src/login.rs"]);
    let banner = banner_view(&state).expect("a rebase is live");
    assert_eq!(banner.kind.as_str(), "REBASING");
    assert_eq!(banner.step, Some((3, 7)));
    assert_eq!(
        banner.conflict.as_ref().map(SanitizedTextExt::text),
        Some("src/auth.rs")
    );
    assert_eq!(banner.more_conflicts, 1);
    assert!(banner.focused);
    assert!(banner.skippable, "a rebase has a step to skip");
}

/// `git merge --skip` does not exist, so the key is never offered on one.
#[test]
fn a_merge_is_not_skippable() {
    let state = stopped(InProgress::Merge, &["a.rs"]);
    let banner = banner_view(&state).expect("a merge is live");
    assert_eq!(banner.kind.as_str(), "MERGING");
    assert!(!banner.skippable);
}

/// A bisect has no Continue/Skip/Abort shape, so it draws no banner — and
/// without a banner the modal never claims a key, whatever `banner_focus`
/// happens to say.
#[test]
fn a_bisect_draws_no_banner_and_claims_no_key() {
    let mut state = stopped(InProgress::Bisect, &[]);
    assert!(banner_view(&state).is_none());
    let (claimed, commands) = press(&mut state, KeyCode::Char('a'));
    assert!(!claimed, "the key falls through to the graph");
    assert!(commands.is_empty());
}

/// §2.1's modal rule: every key but `Ctrl+C` is the banner's. `u` is the one
/// that matters — undoing the entry *before* the live one, mid-rebase, is
/// the bug this swallow exists to stop.
#[test]
fn the_focused_banner_claims_every_key_but_control_c() {
    for code in [
        KeyCode::Char('u'),
        KeyCode::Char('j'),
        KeyCode::Char('S'),
        KeyCode::Char('m'),
        KeyCode::Char('q'),
        KeyCode::Enter,
    ] {
        let mut state = stopped(InProgress::Rebase, &["a.rs"]);
        let (claimed, commands) = press(&mut state, code);
        assert!(claimed, "{code:?} should be swallowed by the modal banner");
        assert!(commands.is_empty(), "{code:?} queued {commands:?}");
    }

    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    let mut commands = Vec::new();
    let claimed = banner_key(
        &mut state,
        &mut commands,
        KeyPress {
            code: KeyCode::Char('c'),
            modifiers: Modifiers::CONTROL,
        },
        &mut || JobId::new(1),
    );
    assert!(!claimed, "Ctrl+C passes through to quit");
}

#[test]
fn tab_escape_and_backtab_give_the_keyboard_back() {
    for code in [KeyCode::Tab, KeyCode::BackTab, KeyCode::Escape] {
        let mut state = stopped(InProgress::Rebase, &["a.rs"]);
        let (claimed, _) = press(&mut state, code);
        assert!(claimed);
        assert!(!state.banner_focus, "{code:?} left the banner focused");
    }
}

/// `c` with files still unmerged costs no job and names the three keys that
/// get the user out of it — which git's own `--continue` message does not.
#[test]
fn continue_refuses_in_words_while_anything_is_unmerged() {
    let mut state = stopped(InProgress::Rebase, &["a.rs", "b.rs"]);
    assert_eq!(
        continue_refusal(&state).as_deref(),
        Some("2 files still conflicted")
    );
    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    assert!(commands.is_empty(), "no job runs: {commands:?}");
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "2 files still conflicted".to_owned(),
            keys: Some(RESOLVE_KEYS.to_owned()),
        }
    );
    assert!(state.banner_focus, "the banner keeps the keyboard");
}

/// One file reads "1 file", not "1 files".
#[test]
fn the_refusal_counts_one_file_in_the_singular() {
    let state = stopped(InProgress::Rebase, &["a.rs"]);
    assert_eq!(
        continue_refusal(&state).as_deref(),
        Some("1 file still conflicted")
    );
}

#[test]
fn continue_with_nothing_unmerged_queues_an_unstoppable_resume() {
    let mut state = stopped(InProgress::Rebase, &[]);
    assert_eq!(continue_refusal(&state), None);
    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    let [Command::Resume(id, Resume::Continue)] = commands.as_slice() else {
        panic!("expected one Resume: {commands:?}");
    };
    assert_eq!(state.operating, Some(*id));
    assert_eq!(state.status, Status::Busy("continue rebase".to_owned()));
    let running = state.remote.running.as_ref().expect("a running record");
    assert!(
        matches!(running.stoppable, crate::update::remote::Stoppable::No(_)),
        "a --continue is not stoppable: {:?}",
        running.stoppable
    );
}

#[test]
fn skip_on_a_merge_says_there_is_nothing_to_skip() {
    let mut state = stopped(InProgress::Merge, &["a.rs"]);
    let (claimed, commands) = press(&mut state, KeyCode::Char('s'));
    assert!(claimed);
    assert!(commands.is_empty());
    assert!(state.confirm.is_none(), "no box opens on a merge");
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "nothing to skip in a merge".to_owned(),
            keys: Some("c continue  a abort".to_owned()),
        }
    );
}

#[test]
fn skip_on_a_rebase_boxes_first_and_names_the_step() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    let (claimed, commands) = press(&mut state, KeyCode::Char('s'));
    assert!(claimed);
    assert!(commands.is_empty(), "the box runs nothing yet");
    let pending = state.confirm.as_ref().expect("a box");
    assert_eq!(pending.view.title.as_str(), "Skip this commit?");
    assert_eq!(
        pending.view.message[0].as_str(),
        "step 3 of 7 is left out of the result"
    );
    assert_eq!(pending.view.command.as_str(), "git rebase --skip");
    let labels: Vec<&str> = pending
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(
        labels,
        ["Cancel", "Skip"],
        "Cancel is index 0 (ADR-0038 §1)"
    );
    assert!(matches!(pending.then, ConfirmThen::Resume(Resume::Skip)));
}

/// The one that must never be unreachable. `a` opens its box in every
/// conflicted state the banner can be in, including with nothing unmerged
/// (a stalled `--continue`) and on a merge (which has no `s`).
#[test]
fn abort_is_offered_in_every_state_the_banner_can_be_in() {
    for kind in [
        InProgress::Merge,
        InProgress::Rebase,
        InProgress::CherryPick,
        InProgress::Revert,
    ] {
        for paths in [&[][..], &["a.rs"][..], &["a.rs", "b.rs"][..]] {
            let mut state = stopped(kind, paths);
            let (claimed, _) = press(&mut state, KeyCode::Char('a'));
            assert!(claimed, "{kind:?} {paths:?}");
            let pending = state
                .confirm
                .as_ref()
                .unwrap_or_else(|| panic!("no abort box for {kind:?} {paths:?}"));
            assert_eq!(
                pending.view.title.as_str(),
                format!("Abort the {}?", kind.name())
            );
            assert_eq!(pending.view.command.as_str(), kind.abort_command());
            let labels: Vec<&str> = pending
                .view
                .choices
                .iter()
                .map(graft_term::SanitizedText::as_str)
                .collect();
            assert_eq!(labels, ["Cancel", "Abort"]);
            assert!(matches!(pending.then, ConfirmThen::Resume(Resume::Abort)));
        }
    }
}

/// `git mergetool` with no `merge.tool` asks "which one?" on stdin, and
/// stdin at that moment belongs to a terminal Graft has given away. So the
/// key refuses before the hand-over, and the line names what does work.
#[test]
fn mergetool_with_none_configured_refuses_before_handing_over() {
    let mut state = stopped(InProgress::Merge, &["src/auth.rs"]);
    assert!(state.mergetool.is_none());
    let (claimed, commands) = press(&mut state, KeyCode::Char('M'));
    assert!(claimed);
    assert!(
        commands.is_empty(),
        "the terminal is never handed over: {commands:?}"
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "no merge.tool configured".to_owned(),
            keys: Some("e edits src/auth.rs  a abort".to_owned()),
        }
    );
}

#[test]
fn mergetool_with_one_configured_asks_the_loop_for_the_first_conflict() {
    let mut state = stopped(InProgress::Merge, &["src/auth.rs", "src/login.rs"]);
    state.mergetool = Some("vimdiff".to_owned());
    let (claimed, commands) = press(&mut state, KeyCode::Char('M'));
    assert!(claimed);
    let [Command::OpenMergetool { path }] = commands.as_slice() else {
        panic!("expected one OpenMergetool: {commands:?}");
    };
    assert_eq!(path, std::path::Path::new("src/auth.rs"));
}

/// A rebase that stops again on the next commit is the same entry and the
/// same banner: focus stays, and the line says it is a second stop.
#[test]
fn stopping_again_keeps_the_banner_focused() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    let id = JobId::new(7);
    state.operating = Some(id);
    let commands = resumed(
        &mut state,
        id,
        crate::jobs::operate::Operated::Conflicted {
            description: "rebase feature onto main".to_owned(),
            kind: InProgress::Rebase,
            paths: vec!["a.rs".into(), "b.rs".into()],
            stalled: None,
        },
        &mut || JobId::new(8),
    );
    assert!(state.banner_focus, "the banner keeps the keyboard");
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "stopped again: conflicts in 2 files".to_owned(),
            keys: Some(STUCK_KEYS.to_owned()),
        }
    );
    assert!(
        !commands.is_empty(),
        "git's state moved: something must be re-read"
    );
}

/// Finishing gives the keyboard back. Leaving `banner_focus` set with no
/// banner to focus would swallow every key into a mode with no way out.
#[test]
fn finishing_clears_the_focus() {
    let mut state = stopped(InProgress::Rebase, &[]);
    let id = JobId::new(7);
    state.operating = Some(id);
    resumed(
        &mut state,
        id,
        crate::jobs::operate::Operated::Failed {
            error: "a hook refused the commit".to_owned(),
            refresh: None,
            remote: None,
            refused: None,
        },
        &mut || JobId::new(8),
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "a hook refused the commit".to_owned(),
            keys: Some("a abort".to_owned()),
        },
        "a failed resume still offers the way out"
    );
}

/// Convenience for reading a `SanitizedText` back as `&str` in assertions.
trait SanitizedTextExt {
    fn text(&self) -> &str;
}

impl SanitizedTextExt for graft_term::SanitizedText {
    fn text(&self) -> &str {
        self.as_str()
    }
}

/// `Tab` out of the banner and `Tab` back in. A mode with a door out and no
/// door in strands `c`, `s` and `a`: the whole point of focusing the banner
/// is that the next key goes to the thing that stopped.
#[test]
fn tab_comes_back_round_to_the_banner() {
    let mut state = stopped(InProgress::Merge, &["src/auth.rs"]);
    let (claimed, _) = press(&mut state, KeyCode::Tab);
    assert!(claimed, "the focused banner answers Tab itself");
    assert!(!state.banner_focus, "and lets go");

    // Detail pane hidden: one `Tab` from the graph is the whole cycle.
    assert!(!state.detail.shown, "this state has no detail pane open");
    assert!(
        take_focus(&mut state, key(KeyCode::Tab)),
        "Tab takes the keyboard back"
    );
    assert!(state.banner_focus);
}

#[test]
fn tab_into_the_banner_waits_its_turn_behind_the_detail_pane() {
    let mut state = stopped(InProgress::Rebase, &["src/auth.rs"]);
    state.banner_focus = false;
    state.detail.shown = true;

    // Graph → Files → Diff belongs to the detail pane; only the step that
    // would land back on the graph lands on the banner instead.
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    assert!(!take_focus(&mut state, key(KeyCode::Tab)));
    state.detail.focus = graft_ui::detail::DetailFocus::Files;
    assert!(!take_focus(&mut state, key(KeyCode::Tab)));
    state.detail.focus = graft_ui::detail::DetailFocus::Diff;
    assert!(take_focus(&mut state, key(KeyCode::Tab)), "then the banner");
    assert!(state.banner_focus);
}

#[test]
fn nothing_takes_focus_when_there_is_no_banner_to_focus() {
    let mut state = AppState::default();
    assert!(
        !take_focus(&mut state, key(KeyCode::Tab)),
        "Tab with no operation in progress is the panes' own"
    );
    assert!(!state.banner_focus);
}
