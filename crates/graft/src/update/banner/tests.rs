//! Unit tests for the operation banner's pure half.
//!
//! The banner's behaviour over the real job pool — a real conflict, a real
//! `--continue`, a real abort — is `tests/it/conflict_interim.rs`. What is
//! here is the part that is a function over plain data: which keys the
//! modal claims, what the refusals say, and what the header draws.

use graft_core::event::{Event, Modifiers};
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{StatusEntry, WorkingTreeStatus};

use super::*;
use crate::update::{AppEvent, Product};

fn detail(kind: InProgress, step: Option<(u32, u32)>) -> InProgressDetail {
    InProgressDetail {
        kind,
        step,
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

/// An `Applied` naming a finished `--continue`. Only `description` and
/// `refresh` are read by `resumed`; the rest is a plausible journal entry.
pub(super) fn done_applied() -> graft_ops::apply::Applied {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    graft_ops::apply::Applied {
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Rebase,
            description: "continue rebase".to_owned(),
            status: EntryStatus::Committed,
            touches: Touches::default(),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
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
///
/// `⏎` was in this list until m6-conflicts/04 gave it a job (§2.6a: it opens
/// the resolver on the first conflicted file). It is still the banner's —
/// `every_hint_the_focused_banner_draws_is_answered_by_banner_key` presses
/// it and checks what it does — it just no longer does *nothing*.
#[test]
fn the_focused_banner_claims_every_key_but_control_c() {
    for code in [
        KeyCode::Char('u'),
        KeyCode::Char('j'),
        KeyCode::Char('S'),
        KeyCode::Char('m'),
        KeyCode::Char('q'),
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
    state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
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

/// `c` on a resolved merge **begun in the shell** (no live entry of Graft's)
/// opens the finish summary like any merge — no shell check here (m6-20 B1).
/// The `⏎` that follows sends `Resume::Continue`, which `graft_ops` routes
/// to `Operation::FinishMerge` since ticket 20 Part A.
#[test]
fn c_on_a_shell_begun_merge_opens_the_summary() {
    let mut state = stopped(InProgress::Merge, &[]);
    assert!(state.live_entries.is_empty(), "begun in the shell");
    assert_eq!(
        crate::update::commit_editor::live_operation(&state),
        Some((
            InProgress::Merge,
            crate::update::commit_editor::Begun::InShell
        ))
    );
    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    let finish = state.finish.as_ref().expect("the summary is open");
    assert_eq!(finish.return_to, crate::update::finish::ReturnTo::Banner);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Resume(..))),
        "no Resume before ⏎: {commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "the summary is read: {commands:?}"
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

/// `HEAD` on `main` at a commit, or detached there: the two shapes the
/// abort box names `<into>` from.
fn refs_on(branch: Option<&str>) -> std::sync::Arc<crate::refs::RefBadges> {
    use graft_core::ref_name::RefName;
    use graft_git::repo::HeadState;
    let at = graft_core::oid::Oid::from_bytes(&[0xab; 20]).expect("20 bytes");
    let name = |full: &str| RefName::new(full.as_bytes().to_vec()).expect("a valid ref name");
    let head = branch.map_or(HeadState::Detached(at), |short| {
        HeadState::Branch(name(&format!("refs/heads/{short}")))
    });
    let refs = [(name("refs/heads/main"), at)];
    std::sync::Arc::new(
        crate::refs::badges(
            &refs,
            &head,
            &std::collections::HashMap::new(),
            &[],
            |_, _| {
                Ok(graft_git::history::AheadBehind {
                    ahead: 0,
                    behind: 0,
                })
            },
        )
        .expect("no ahead/behind walk can fail here"),
    )
}

/// A merge stopped, the journal read, and `live` the entries git is still
/// inside: empty for a merge begun in the shell.
fn merge_read(live: Vec<graft_core::journal::EntryId>, branch: Option<&str>) -> AppState {
    AppState {
        live_entries: live.clone(),
        journal: crate::update::journal::JournalState {
            entries: crate::update::detail::Loading::Ready(std::sync::Arc::new(
                crate::jobs::undo::JournalRead {
                    entries: Vec::new(),
                    unreadable: Vec::new(),
                    live,
                    interrupted_edits: Vec::new(),
                },
            )),
            ..crate::update::journal::JournalState::default()
        },
        refs: Some(refs_on(branch)),
        ..stopped(InProgress::Merge, &["notes.txt", "gone.txt"])
    }
}

/// The abort box's message lines, as drawn.
fn abort_lines(state: &mut AppState) -> Vec<String> {
    let (claimed, commands) = press(state, KeyCode::Char('a'));
    assert!(claimed && commands.is_empty(), "{commands:?}");
    let pending = state.confirm.as_ref().expect("the abort box");
    assert_eq!(pending.view.selected(), 0, "the cursor starts on Cancel");
    assert!(matches!(pending.then, ConfirmThen::Resume(Resume::Abort)));
    pending
        .view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect()
}

/// **`a` on a merge begun in the shell** (m6-conflicts/57): the box says
/// what the abort does to the branch and that `u` takes it back — ops
/// journals it as an `AbortMerge`. `<into>` is `HEAD`'s branch, or its
/// short id when detached.
#[test]
fn a_on_a_shell_merge_says_u_puts_it_back() {
    let mut state = merge_read(Vec::new(), Some("main"));
    assert_eq!(
        abort_lines(&mut state),
        [
            "main goes back to before the merge",
            "u puts the merge back"
        ]
    );
    let mut detached = merge_read(Vec::new(), None);
    assert_eq!(
        abort_lines(&mut detached),
        [
            "abababa goes back to before the merge",
            "u puts the merge back"
        ]
    );
}

/// A merge Graft began, and a merge whose launch journal read is still
/// out, keep today's lines: the new ones promise a `u` only a shell-begun
/// abort is known to have.
#[test]
fn a_on_a_merge_begun_in_graft_keeps_its_lines() {
    let today = [
        "everything goes back to before it started",
        "the journal keeps the entry",
    ];
    let mut graft = merge_read(
        vec![graft_core::journal::EntryId::new(7, [7; 10])],
        Some("main"),
    );
    assert_eq!(abort_lines(&mut graft), today);
    let mut unread = AppState {
        journal: crate::update::journal::JournalState::default(),
        ..merge_read(Vec::new(), Some("main"))
    };
    assert_eq!(abort_lines(&mut unread), today);
}

/// **`u`/`U` on the focused banner of a merge begun in the shell are
/// sent** (keymap §2.6a's exception): the same job the global key sends,
/// and ops' gate decides.
#[test]
fn u_on_the_banner_of_a_shell_merge_is_sent() {
    for (code, redo) in [(KeyCode::Char('u'), false), (KeyCode::Char('U'), true)] {
        let mut state = merge_read(Vec::new(), Some("main"));
        let (claimed, commands) = press(&mut state, code);
        assert!(claimed, "the focused banner still claims {code:?}");
        let [command] = commands.as_slice() else {
            panic!("{code:?}: {commands:?} · {:?}", state.status)
        };
        assert!(
            matches!(
                (command, redo),
                (Command::Undo(_, false), false) | (Command::Redo(_, false), true)
            ),
            "{code:?}: {command:?}"
        );
        assert!(state.banner_focus, "the banner keeps the keyboard");
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
    state.resuming = Some((id, Resume::Continue));
    let commands = resumed(
        &mut state,
        id,
        crate::jobs::operate::Operated::Conflicted {
            rewrite: None,
            description: "rebase feature onto main".to_owned(),
            kind: InProgress::Rebase,
            paths: vec!["a.rs".into(), "b.rs".into()],
            stalled: None,
            entry: graft_core::journal::EntryId::new(1, [0; 10]),
        },
        &mut || JobId::new(8),
    );
    assert!(state.banner_focus, "the banner keeps the keyboard");
    // The stop is a state, not a sticky line (m6-conflicts/12): the footer
    // words it `rebase stopped again at step 3 of 7 · …`.
    assert_eq!(state.status, Status::Ready);
    assert_eq!(
        state.stopped,
        Some(crate::update::Stop {
            rewrite: None,
            origin: crate::update::LiveOrigin::ThisRun,
            again: true,
            kind: InProgress::Rebase,
            paths_at_stop: 2,
        })
    );
    assert!(
        !commands.is_empty(),
        "git's state moved: something must be re-read"
    );
    assert_eq!(
        state.live_entries,
        vec![graft_core::journal::EntryId::new(1, [0; 10])],
        "the stop names its entry, and the state keeps it (gap row 505)"
    );
}

/// A resume that failed leaves the operation live and `a` one key away —
/// whatever git refused, the way back to before it started still works.
///
/// Renamed: it was called `finishing_clears_the_focus` and tested neither
/// finishing nor focus (m5-09 review, Major 3). The name is now what it
/// asserts, and the test below is what the old name claimed.
#[test]
fn a_failed_resume_keeps_a_abort() {
    let mut state = stopped(InProgress::Rebase, &[]);
    let id = JobId::new(7);
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    resumed(
        &mut state,
        id,
        crate::jobs::operate::Operated::Failed {
            error: "a hook refused the commit".to_owned(),
            refresh: None,
            remote: None,
            refused: None,
            edits: None,
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
    assert!(
        state.banner_focus,
        "the operation is still live, so the banner still has the keyboard"
    );
}

/// Finishing gives the keyboard back. Leaving `banner_focus` set with no
/// banner to focus would swallow every key into a mode with no way out.
///
/// Asserted on `resumed` alone, before any re-read lands: the mutation that
/// deletes `state.banner_focus = false` from the `Done` arm must turn this
/// red here and now. Driving it through the loop instead would let
/// `took_fact` clear the flag when `in_progress` goes `None` and the test
/// would pass over a deleted line (m5-09 review, Major 3, mutation M6).
#[test]
fn finishing_clears_the_focus() {
    let mut state = stopped(InProgress::Rebase, &[]);
    let id = JobId::new(7);
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    resumed(
        &mut state,
        id,
        crate::jobs::operate::Operated::Done(done_applied()),
        &mut || JobId::new(8),
    );
    assert!(
        !state.banner_focus,
        "a finished operation has no banner to hold the keyboard"
    );
    assert_eq!(state.status, Status::done("continue rebase"));
}

/// **Gap row 569.** The answer to `--continue` already says the rebase
/// ended, so `in_progress` goes `None` in the same update, and `u` in the
/// very next one is an undo — not *"finish or abort the rebase first"*,
/// which it was for as long as the working-tree re-read took to land.
///
/// Driven through `update` with no `WorkingTree` product in between: the
/// re-read the answer scheduled is left unanswered on purpose.
#[test]
fn u_right_after_continue_is_accepted() {
    let mut state = stopped(InProgress::Rebase, &[]);
    let id = JobId::new(7);
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    let (answered, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Done(
                done_applied(),
            ))),
        ),
    );
    assert_eq!(answered.in_progress, None, "the answer ended it");
    assert!(!answered.banner_focus);

    let (pressed, commands) = crate::update::update(
        &answered,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('u')))),
    );
    let Some(undo) = pressed.operating else {
        panic!("u was refused: {:?}", pressed.status);
    };
    assert!(
        commands.contains(&Command::Undo(undo, false)),
        "{commands:?}"
    );
}

/// The re-read is a confirmation, never a silent contradiction: a
/// working-tree read that still finds the rebase after the answer said it
/// ended puts it back **and says so**.
#[test]
fn answer_and_read_disagree_says_so() {
    let mut state = stopped(InProgress::Rebase, &[]);
    let id = JobId::new(7);
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    let (answered, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Done(
                done_applied(),
            ))),
        ),
    );
    let read = crate::working_tree::WorkingTree {
        rewrite: None,
        status: std::sync::Arc::new(WorkingTreeStatus {
            entries: Vec::new(),
        }),
        in_progress: Some(detail(InProgress::Rebase, Some((4, 7)))),
        mergetool: None,
    };
    let (reread, _) = crate::update::update(
        &answered,
        AppEvent::Produced(JobId::new(9), Product::WorkingTree(Ok(Box::new(read)))),
    );
    assert!(reread.in_progress.is_some(), "git's word stands");
    assert_eq!(
        reread.status,
        Status::sticky("git says a rebase is still in progress \u{b7} Ctrl+R".to_owned())
    );

    // A read that agrees says nothing new.
    let agreeing = crate::working_tree::WorkingTree {
        rewrite: None,
        status: std::sync::Arc::new(WorkingTreeStatus {
            entries: Vec::new(),
        }),
        in_progress: None,
        mergetool: None,
    };
    let (confirmed, _) = crate::update::update(
        &answered,
        AppEvent::Produced(JobId::new(9), Product::WorkingTree(Ok(Box::new(agreeing)))),
    );
    assert_eq!(confirmed.status, answered.status);
    assert_eq!(confirmed.in_progress, None);
}

/// `M` and `e` with nothing unmerged: the banner acts on "the first
/// conflicted file", and there is none. They used to return `false` into a
/// `banner_key` that claimed the key anyway — silently nothing, which reads
/// as broken (m5-09 review, Major 2). The hint bar drops them in this state;
/// this is what a user who learned the key elsewhere gets.
#[test]
fn mergetool_and_edit_refuse_in_words_when_nothing_is_conflicted() {
    for code in [KeyCode::Char('M'), KeyCode::Char('e')] {
        let mut state = stopped(InProgress::Rebase, &[]);
        state.mergetool = Some("meld".to_owned());
        let (claimed, commands) = press(&mut state, code);
        assert!(claimed);
        assert!(commands.is_empty(), "{code:?} queued {commands:?}");
        assert_eq!(
            state.status,
            Status::Sticky {
                reason: "nothing conflicted".to_owned(),
                keys: Some("c continue  a abort".to_owned()),
            },
            "{code:?} said nothing"
        );
    }
}

/// `Tab` out of the banner is the next step of the cycle — graph → detail →
/// banner → **graph** — not a return ticket to the pane it came from.
///
/// It used to leave `detail.focus` alone, so `Tab` from the Diff pane went
/// Diff → banner → Diff → banner for as long as a banner was drawn, and the
/// graph was unreachable by `Tab` (m5-09 review, Major 1).
#[test]
fn tab_out_of_the_banner_reaches_the_graph() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    state.detail.shown = true;
    state.detail.focus = graft_ui::detail::DetailFocus::Diff;

    // The forward step the banner intercepted on its way to the graph.
    state.banner_focus = false;
    assert!(take_focus(&mut state, key(KeyCode::Tab)), "Tab lands on it");
    assert!(state.banner_focus);

    let (claimed, _) = press(&mut state, KeyCode::Tab);
    assert!(claimed);
    assert!(!state.banner_focus, "Tab gives the keyboard back");
    assert_eq!(
        state.detail.focus,
        graft_ui::detail::DetailFocus::Graph,
        "and the graph is where it lands"
    );
}

/// `BackTab` is the same cycle backwards, so it lands on the pane the
/// forward step came from rather than on the graph.
#[test]
fn backtab_out_of_the_banner_reaches_the_diff_pane() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    state.detail.shown = true;
    state.detail.focus = graft_ui::detail::DetailFocus::Files;
    state.banner_focus = false;
    assert!(take_focus(&mut state, key(KeyCode::BackTab)));

    let (claimed, _) = press(&mut state, KeyCode::BackTab);
    assert!(claimed);
    assert!(!state.banner_focus);
    assert_eq!(state.detail.focus, graft_ui::detail::DetailFocus::Diff);
}

/// `Esc` is "leave the mode", not a step of the cycle: it puts the keyboard
/// back where it found it.
#[test]
fn escape_out_of_the_banner_leaves_the_pane_focus_alone() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    state.detail.shown = true;
    state.detail.focus = graft_ui::detail::DetailFocus::Files;
    let (claimed, _) = press(&mut state, KeyCode::Escape);
    assert!(claimed);
    assert!(!state.banner_focus);
    assert_eq!(state.detail.focus, graft_ui::detail::DetailFocus::Files);
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

// ---------------------------------------------------------------------------
// The anchor: the hint bar against `banner_key` itself (m5/16).
// ---------------------------------------------------------------------------

/// The key `key` names, as a `KeyPress` code.
///
/// Panics on a key this test cannot press, which is the point: if the bar
/// learns a key, this test fails until somebody says what pressing it is
/// supposed to do.
fn code_for(key: &str) -> KeyCode {
    match key {
        "c" => KeyCode::Char('c'),
        "s" => KeyCode::Char('s'),
        "a" => KeyCode::Char('a'),
        "M" => KeyCode::Char('M'),
        "e" => KeyCode::Char('e'),
        "⏎" => KeyCode::Enter,
        "⇥" => KeyCode::Tab,
        other => panic!(
            "the focused banner's hint bar draws `{other}`, which this test \
             does not know how to press — teach it what that key is meant to \
             do rather than letting the bar advertise it unchecked"
        ),
    }
}

/// `c` with nothing unmerged: what it queued.
fn continued(key: &str, verb: &str, before: &AppState, after: &AppState, commands: &[Command]) {
    match live(before).map(|detail| detail.kind) {
        // A merge shows the finish summary first (m6-conflicts/19):
        // nothing is written until its `⏎`.
        Some(InProgress::Merge) => {
            assert!(after.finish.is_some(), "`{key} {verb}` opened no summary");
            assert!(
                matches!(commands, [Command::ReadFinishSummary(_)]),
                "`{key} {verb}` queued {commands:?}"
            );
        }
        // A rebase or cherry-pick the shell began: handed to git (07c).
        Some(InProgress::Rebase | InProgress::CherryPick) if before.live_entries.is_empty() => {
            assert!(
                matches!(commands, [Command::ContinueInGit { .. }]),
                "`{key} {verb}` queued {commands:?}"
            );
        }
        _ => assert!(
            matches!(commands, [Command::Resume(_, Resume::Continue)]),
            "`{key} {verb}` queued {commands:?}"
        ),
    }
}

/// `c`/`s`/`a`: the three that move the operation on.
fn did_a_resume_verb(
    verb: &str,
    key: &str,
    before: &AppState,
    after: &AppState,
    commands: &[Command],
) {
    match verb {
        // Unmerged files: a refusal in words. None: the real `--continue`.
        "continue" => match continue_refusal(before) {
            Some(reason) => assert_eq!(
                after.status,
                Status::Sticky {
                    reason,
                    keys: Some(RESOLVE_KEYS.to_owned()),
                },
                "`{key} {verb}`"
            ),
            None => continued(key, verb, before, after, commands),
        },
        "skip" | "abort" => {
            let want = if verb == "skip" {
                Resume::Skip
            } else {
                Resume::Abort
            };
            let confirm = after
                .confirm
                .as_ref()
                .unwrap_or_else(|| panic!("`{key} {verb}` opened no box"));
            assert!(
                matches!(&confirm.then, ConfirmThen::Resume(what) if *what == want),
                "`{key} {verb}` opened {:?}",
                confirm.then
            );
        }
        other => panic!("not a resume verb: {other}"),
    }
}

/// `M`/`e`: the two that hand a conflicted file to the user's own tool.
/// Hinted only while there is one, so "there is none" is a failure here.
fn did_a_file_verb(verb: &str, key: &str, before: &AppState, commands: &[Command]) {
    let path = first_conflicted(before).unwrap_or_else(|| {
        panic!("`{key} {verb}` is hinted with nothing conflicted for it to open")
    });
    let (("mergetool", [Command::OpenMergetool { path: queued }])
    | ("edit", [Command::OpenConflictEditor { path: queued }])) = (verb, commands)
    else {
        panic!("`{key} {verb}` queued {commands:?}")
    };
    assert_eq!(*queued, path, "`{key} {verb}` opened the wrong file");
}

/// `⏎`: the one that stays in the app. It opens the resolver on the first
/// conflicted file and queues the read of it. Hinted only while there is
/// one, so "there is none" is a failure here too.
fn did_resolve(key: &str, before: &AppState, after: &AppState, commands: &[Command]) {
    let path = first_conflicted(before).unwrap_or_else(|| {
        panic!("`{key} resolve` is hinted with nothing conflicted for it to open")
    });
    let resolver = after
        .resolver
        .as_ref()
        .unwrap_or_else(|| panic!("`{key} resolve` opened no resolver"));
    assert_eq!(resolver.path, path, "`{key} resolve` opened the wrong file");
    // The opening read, and the file list's conflict set (m6-conflicts/13).
    let [
        Command::OpenConflict(_, queued),
        Command::ReadConflictSet(_),
    ] = commands
    else {
        panic!("`{key} resolve` queued {commands:?}")
    };
    assert_eq!(*queued, path, "`{key} resolve` read the wrong file");
}

/// What pressing `verb`'s key must have done. Every arm asserts the verb's
/// own outcome; an unknown verb is a failure, not a pass.
fn did_the_verb(verb: &str, key: &str, before: &AppState, after: &AppState, commands: &[Command]) {
    match verb {
        "continue" | "skip" | "abort" => did_a_resume_verb(verb, key, before, after, commands),
        "mergetool" | "edit" => did_a_file_verb(verb, key, before, commands),
        "resolve" => did_resolve(key, before, after, commands),
        "graph" => assert!(
            !after.banner_focus,
            "`{key} {verb}` did not give the keyboard back"
        ),
        other => panic!(
            "the focused banner's hint bar draws the verb `{other}`, which \
             this test does not check — a verb nobody checks is a verb the \
             key need not do"
        ),
    }
}

/// **The anchor.** Every hint the focused banner's footer draws is checked
/// against what `banner_key` *does with that key*, by pressing it.
///
/// The two tests written to catch the dead banner hints
/// (`update::tests::every_key_the_bar_draws_is_answered` and
/// `every_planned_hint_is_still_unbound`) compare the hint table with
/// `keys::bound_actions` — another table. Group 09 bound `c`/`s`/`a` in
/// `banner_key` and flipped neither, so both tables were wrong in the same
/// direction, agreed with each other, and passed while the footer under a
/// modal banner taught `<> resize  . more` (m5/14, found on the way).
///
/// This test cannot agree its way out of that: the only thing it compares
/// the bar with is the dispatcher.
#[test]
fn every_hint_the_focused_banner_draws_is_answered_by_banner_key() {
    use graft_ui::graph_screen::{Divider, HintContext, hints_for};

    for (skippable, conflicted) in [(true, true), (false, true), (true, false), (false, false)] {
        let kind = if skippable {
            InProgress::Rebase
        } else {
            InProgress::Merge
        };
        let paths: &[&str] = if conflicted {
            &["src/auth.rs", "src/login.rs"]
        } else {
            &[]
        };
        let mut base = stopped(kind, paths);
        // So `M` can queue the hand-over rather than refuse for want of a
        // tool: the refusal is its own test.
        base.mergetool = Some("meld".to_owned());
        let banner = banner_view(&base).expect("an operation is live");
        assert_eq!(banner.skippable, skippable);
        assert_eq!(banner.conflict.is_some(), conflicted);
        let context = HintContext::Banner {
            skippable: banner.skippable,
            conflicted: banner.conflict.is_some(),
            merge: false,
        };

        for columns in [40u16, 80, 100, 120, 160, 200] {
            for divider in [Divider::Shown, Divider::Hidden] {
                let hints = hints_for(&context, divider, columns);
                assert!(
                    !hints.is_empty(),
                    "the modal banner drew no key at all at {columns} columns \
                     ({divider:?}) — ADR-0037 §4's one forbidden outcome"
                );
                for hint in hints {
                    let mut state = base.clone();
                    let (claimed, commands) = press(&mut state, code_for(hint.key));
                    assert!(
                        claimed,
                        "`{} {}` is drawn but `banner_key` does not claim it",
                        hint.key, hint.verb
                    );
                    did_the_verb(hint.verb, hint.key, &base, &state, &commands);
                }
            }
        }
    }
}

/// `c continue` and `a abort` are drawn in **every** banner state and at
/// every width: they are the two ways out of a stopped operation, and the
/// banner owns the keyboard while it is up.
#[test]
fn the_banner_always_teaches_continue_and_abort() {
    use graft_ui::graph_screen::{Divider, HintContext, hints_for};

    for (skippable, conflicted) in [(true, true), (false, true), (true, false), (false, false)] {
        for columns in [40u16, 80, 120, 160] {
            let hints = hints_for(
                &HintContext::Banner {
                    skippable,
                    conflicted,
                    merge: false,
                },
                Divider::Shown,
                columns,
            );
            for want in [("c", "continue"), ("a", "abort")] {
                assert!(
                    hints.iter().any(|hint| (hint.key, hint.verb) == want),
                    "{want:?} missing at {columns} columns from {hints:?}"
                );
            }
        }
    }
}

/// **§2.6, and review M3.** "`u` and `U` are refused while an operation is
/// live, **focused or not**". The focused banner used to drop them into its
/// silent modal arm, so the doc said one thing for both focus states and
/// the code said it in one: a key pressed on purpose drew nothing at all.
///
/// The words are `undo::in_progress_refusal`'s own, not a copy, so the two
/// focus states cannot drift apart again.
#[test]
fn a_focused_banner_answers_u_instead_of_swallowing_it() {
    for kind in [
        InProgress::Merge,
        InProgress::Rebase,
        InProgress::CherryPick,
        InProgress::Revert,
    ] {
        for code in [KeyCode::Char('u'), KeyCode::Char('U')] {
            let mut state = stopped(kind, &["src/auth.rs"]);
            assert!(state.banner_focus, "the banner has the keyboard");
            let (claimed, commands) = press(&mut state, code);
            assert!(claimed, "{code:?} is still the modal's: {commands:?}");
            assert!(
                commands.is_empty(),
                "{code:?} undid something: {commands:?}"
            );
            assert_eq!(state.operating, None, "{code:?} claimed a job");
            let unfocused = AppState {
                banner_focus: false,
                ..stopped(kind, &["src/auth.rs"])
            };
            assert_eq!(
                Some(state.status.clone()),
                crate::update::undo::in_progress_refusal(&unfocused),
                "{code:?} says what the unfocused banner says"
            );
            assert!(
                matches!(state.status, Status::Sticky { .. }),
                "{code:?} drew nothing: {:?}",
                state.status
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Landing after a stop (m6-conflicts/12)
// ---------------------------------------------------------------------------

use crate::update::{LiveOrigin, Size, Stop};

/// The frame `state` draws at `columns`×`rows`, its last row only.
fn bottom_row(state: &AppState, columns: u16, rows: u16) -> String {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let mut state = state.clone();
    state.size = Size { columns, rows };
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| crate::view::view(&crate::session::for_tests::session(), &state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..columns)
        .map(|x| buffer[(x, rows - 1)].symbol())
        .collect()
}

fn working_tree(in_progress: Option<InProgressDetail>, paths: &[&str]) -> AppEvent {
    AppEvent::Produced(
        JobId::new(90),
        Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
            rewrite: None,
            status: std::sync::Arc::new(WorkingTreeStatus {
                entries: paths.iter().map(|path| unmerged(path)).collect(),
            }),
            in_progress,
            mergetool: None,
        }))),
    )
}

/// A merge started in this run stops on `gone.txt`, answered through
/// `update` exactly as the job pool answers it, then the refresh's
/// working-tree read lands.
fn a_merge_stops() -> AppState {
    let id = JobId::new(7);
    let state = AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        operating: Some(id),
        status: Status::Busy("merge feature into main".to_owned()),
        // Not a launch: the first read is long gone.
        working_tree: Some(std::sync::Arc::new(WorkingTreeStatus {
            entries: Vec::new(),
        })),
        ..AppState::default()
    };
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Conflicted {
                rewrite: None,
                description: "merge feature into main".to_owned(),
                kind: InProgress::Merge,
                paths: vec!["gone.txt".into()],
                stalled: None,
                entry: graft_core::journal::EntryId::new(1, [0; 10]),
            })),
        ),
    );
    let (state, _) = crate::update::update(
        &state,
        working_tree(Some(detail(InProgress::Merge, None)), &["gone.txt"]),
    );
    state
}

fn step(state: &AppState, code: KeyCode) -> AppState {
    crate::update::update(state, AppEvent::Core(Event::Key(key(code)))).0
}

/// m6-conflicts/18: a rebase stopping on its next step lands in the
/// resolver on that step's first conflicted file.
#[test]
fn a_rebase_stopping_again_lands_in_the_resolver() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    let id = JobId::new(7);
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    let commands = resumed(
        &mut state,
        id,
        crate::jobs::operate::Operated::Conflicted {
            rewrite: None,
            description: "rebase feature onto main".to_owned(),
            kind: InProgress::Rebase,
            paths: vec!["b.rs".into(), "c.rs".into()],
            stalled: None,
            entry: graft_core::journal::EntryId::new(1, [0; 10]),
        },
        &mut || JobId::new(8),
    );
    let open = state.resolver.as_ref().expect("lands in the resolver");
    assert_eq!(open.path, std::path::PathBuf::from("b.rs"));
    assert_eq!(
        open.opened_from,
        crate::update::resolver::OpenedFrom::Banner
    );
    assert!(state.banner_focus);
    assert_eq!(state.status, Status::Ready);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{commands:?}"
    );
}

/// m6-conflicts/18: `Esc` from the resolver a stop landed in shows the
/// banner, focused, with the stop line and its keys on the bottom row.
/// Lives here rather than in `resolver::tests` for `a_merge_stops`.
#[test]
fn esc_from_a_landed_resolver_shows_the_banner_keys() {
    let state = a_merge_stops();
    assert!(state.resolver.is_some(), "the stop landed in the resolver");
    let state = step(&state, KeyCode::Escape);
    assert!(state.resolver.is_none());
    assert!(state.banner_focus);
    let row = bottom_row(&state, 120, 40);
    assert!(row.contains("⏎ resolve"), "{row}");
    assert!(
        row.trim_start()
            .starts_with("merge stopped · 1 file conflicted ·"),
        "{row}"
    );
}

#[test]
fn a_stop_keeps_the_banner_keys_on_the_bottom_row() {
    // m6-conflicts/18: the stop lands in the resolver; `Esc` is the way
    // back to the banner these assertions are about.
    let state = step(&a_merge_stops(), KeyCode::Escape);
    assert!(state.banner_focus);
    assert_eq!(
        state.status,
        Status::Ready,
        "the launch's `Busy` is cleared"
    );
    for (columns, rows) in [(80, 24), (120, 40), (200, 50)] {
        let row = bottom_row(&state, columns, rows);
        assert!(row.contains("⏎ resolve"), "{columns}x{rows}: {row}");
        assert!(row.contains("a abort"), "{columns}x{rows}: {row}");
        assert!(!row.contains("esc dismiss"), "{columns}x{rows}: {row}");
    }
    let row = bottom_row(&state, 120, 40);
    assert!(
        row.trim_start()
            .starts_with("merge stopped · 1 file conflicted ·"),
        "{row}"
    );
}

#[test]
fn the_stop_line_outlives_a_dismissed_problem() {
    // m6-conflicts/18: the stop lands in the resolver; `Esc` back to the
    // banner this test is about.
    let mut state = step(&a_merge_stops(), KeyCode::Escape);
    let id = JobId::new(8);
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Failed {
                error: "a hook refused the commit".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
                edits: None,
            })),
        ),
    );
    assert!(matches!(state.status, Status::Sticky { .. }));
    assert!(
        bottom_row(&state, 120, 40).contains("a hook refused the commit"),
        "the control: the problem takes the row while it stands"
    );
    // `Esc` dismisses the problem and, as it always has, leaves the banner
    // in the same press. The stop is not a status, so it is still there.
    let state = step(&state, KeyCode::Escape);
    assert_eq!(state.status, Status::Ready, "the problem is dismissed");
    assert!(!state.banner_focus);
    assert!(state.stopped.is_some(), "the stop outlives the problem");
    // Back on the banner, its row leads with the stop's reason again.
    let state = step(&state, KeyCode::Tab);
    assert!(state.banner_focus, "the control: one ⇥ is the way back");
    let row = bottom_row(&state, 120, 40);
    assert!(
        row.trim_start()
            .starts_with("merge stopped · 1 file conflicted"),
        "{row}"
    );
}

#[test]
fn tab_keeps_a_failed_resumes_words() {
    let mut state = stopped(InProgress::Rebase, &["a.rs"]);
    let failed = Status::Sticky {
        reason: "a hook refused the commit".to_owned(),
        keys: Some("a abort".to_owned()),
    };
    state.status = failed.clone();
    let (claimed, _) = press(&mut state, KeyCode::Tab);
    assert!(claimed);
    assert!(
        !state.banner_focus,
        "the control: `Tab` did leave the banner"
    );
    assert_eq!(state.status, failed);
}

#[test]
fn esc_on_the_banner_leaves_a_way_back_on_screen() {
    // m6-conflicts/18: the first `Esc` leaves the resolver the stop landed
    // in; this one is on the banner.
    let state = step(&a_merge_stops(), KeyCode::Escape);
    let state = step(&state, KeyCode::Escape);
    assert!(
        !state.banner_focus,
        "the control: `Esc` handed the keyboard back"
    );
    let row = bottom_row(&state, 120, 40);
    assert!(row.contains("⇥ banner"), "{row}");
    let state = step(&state, KeyCode::Tab);
    assert!(state.banner_focus, "one `Tab` is the way back");
    let state = step(&state, KeyCode::Enter);
    let resolver = state.resolver.as_ref().expect("`⏎` opens the resolver");
    assert_eq!(
        Some(resolver.path.clone()),
        first_conflicted(&state),
        "on the first conflicted file"
    );
}

fn stop(origin: LiveOrigin, again: bool, kind: InProgress, paths_at_stop: usize) -> Stop {
    Stop {
        rewrite: None,
        origin,
        again,
        kind,
        paths_at_stop,
    }
}

/// What `stop_reason` is handed from the working-tree read: its detail and
/// its unmerged count.
type LiveRead<'a> = (&'a InProgressDetail, usize);

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one table: every row of the ticket's wording, read top to bottom"
)]
fn stop_reason_words() {
    let merge = detail(InProgress::Merge, None);
    let rebase = detail(InProgress::Rebase, Some((3, 7)));
    let again = detail(InProgress::Rebase, Some((4, 7)));
    let pick = detail(InProgress::CherryPick, None);
    let revert = detail(InProgress::Revert, None);
    let cases: [(Stop, Option<LiveRead<'_>>, &str); 12] = [
        (
            stop(LiveOrigin::ThisRun, false, InProgress::Merge, 1),
            Some((&merge, 1)),
            "merge stopped · 1 file conflicted",
        ),
        (
            stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 2),
            Some((&rebase, 2)),
            "rebase stopped at step 3 of 7 · 2 files conflicted",
        ),
        (
            stop(LiveOrigin::ThisRun, true, InProgress::Rebase, 1),
            Some((&again, 1)),
            "rebase stopped again at step 4 of 7 · 1 file conflicted",
        ),
        (
            stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 0),
            Some((&rebase, 0)),
            "rebase stopped at step 3 of 7",
        ),
        (
            stop(LiveOrigin::ThisRun, false, InProgress::CherryPick, 1),
            Some((&pick, 1)),
            "cherry-pick stopped · 1 file conflicted",
        ),
        (
            stop(LiveOrigin::ThisRun, false, InProgress::Revert, 2),
            Some((&revert, 2)),
            "revert stopped · 2 files conflicted",
        ),
        (
            stop(LiveOrigin::PreviousRun, false, InProgress::Merge, 1),
            Some((&merge, 1)),
            "merge still in progress from a previous run · 1 file conflicted",
        ),
        (
            stop(LiveOrigin::PreviousRun, false, InProgress::Rebase, 1),
            Some((&rebase, 1)),
            "rebase still in progress from a previous run · step 3 of 7 · 1 file conflicted",
        ),
        (
            stop(LiveOrigin::OutsideGraft, false, InProgress::Merge, 1),
            Some((&merge, 1)),
            "merge in progress · begun outside Graft · 1 file conflicted",
        ),
        (
            stop(LiveOrigin::OutsideGraft, false, InProgress::Rebase, 2),
            Some((&rebase, 2)),
            "rebase in progress · begun outside Graft · step 3 of 7 · 2 files conflicted",
        ),
        // The read names another operation: the stop's own kind and count,
        // and no step.
        (
            stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 2),
            Some((&merge, 5)),
            "rebase stopped · 2 files conflicted",
        ),
        // No read at all: the same fallback.
        (
            stop(LiveOrigin::ThisRun, false, InProgress::Merge, 0),
            None,
            "merge stopped",
        ),
    ];
    for (stop, live, words) in cases {
        assert_eq!(stop_reason(&stop, live), words, "{stop:?}");
    }
}

#[test]
fn the_count_follows_the_read() {
    let state = AppState {
        stopped: Some(stop(LiveOrigin::ThisRun, false, InProgress::Merge, 2)),
        in_progress: Some(detail(InProgress::Merge, None)),
        history: Some(crate::view::tests::history_of(5, 0)),
        working_tree: Some(std::sync::Arc::new(WorkingTreeStatus {
            entries: vec![unmerged("a.rs"), unmerged("b.rs")],
        })),
        ..AppState::default()
    };
    let line = crate::view::rows::stopped_line(&state).expect("a stop is standing");
    assert_eq!(
        line.full().as_str(),
        "merge stopped · 2 files conflicted",
        "the control"
    );
    let (state, _) = crate::update::update(
        &state,
        working_tree(Some(detail(InProgress::Merge, None)), &["b.rs"]),
    );
    let line = crate::view::rows::stopped_line(&state).expect("a stop is standing");
    assert_eq!(line.full().as_str(), "merge stopped · 1 file conflicted");
}

/// Review m6-amend-11-12 M1, and m6-conflicts/61 ("shorten both"): the
/// shorter wordings the footer falls back to drop whole parts — who began
/// it, then the step, then the count — so the count is never cut inside,
/// and the last wording carries no number. Only the full rung spells the
/// count out; every rung that had to shorten says it short.
#[test]
fn stop_wordings_drop_whole_parts_lowest_value_first() {
    let merge = detail(InProgress::Merge, None);
    assert_eq!(
        super::stop_wordings(
            &stop(LiveOrigin::OutsideGraft, false, InProgress::Merge, 13),
            Some((&merge, 13)),
        ),
        [
            "merge in progress · begun outside Graft · 13 files conflicted",
            "merge in progress · from the shell · 13 conflicted",
            "merge in progress · 13 conflicted",
            "merge in progress",
        ]
    );
    let rebase = detail(InProgress::Rebase, Some((3, 7)));
    assert_eq!(
        super::stop_wordings(
            &stop(LiveOrigin::PreviousRun, false, InProgress::Rebase, 2),
            Some((&rebase, 2)),
        ),
        [
            "rebase still in progress from a previous run · step 3 of 7 · 2 files conflicted",
            "rebase in progress · from last run · step 3 of 7 · 2 conflicted",
            "rebase still in progress · step 3 of 7 · 2 conflicted",
            "rebase still in progress · 2 conflicted",
            "rebase still in progress",
        ]
    );
    let again = detail(InProgress::Rebase, Some((4, 7)));
    let this_run = stop(LiveOrigin::ThisRun, true, InProgress::Rebase, 1);
    let wordings = super::stop_wordings(&this_run, Some((&again, 1)));
    assert_eq!(
        wordings,
        [
            "rebase stopped again at step 4 of 7 · 1 file conflicted",
            "rebase stopped again at step 4 of 7 · 1 conflicted",
            "rebase stopped again · 1 conflicted",
            "rebase stopped again",
        ]
    );
    // The first wording is `stop_reason`'s, always.
    assert_eq!(
        wordings[0],
        super::stop_reason(&this_run, Some((&again, 1)))
    );
}

/// m6-conflicts/61: the shell's origin survives, short, one rung longer
/// than it did — before the count or the step is given up.
#[test]
fn a_shell_stop_keeps_a_short_origin_before_dropping_it() {
    let merge = detail(InProgress::Merge, None);
    let wordings = super::stop_wordings(
        &stop(LiveOrigin::OutsideGraft, false, InProgress::Merge, 13),
        Some((&merge, 13)),
    );
    assert_eq!(
        wordings[1],
        "merge in progress · from the shell · 13 conflicted"
    );
}

/// m6-conflicts/61: a previous run's stop keeps `from last run`, with the
/// step where the full rung has one, and without `still`.
#[test]
fn a_previous_run_stop_keeps_a_short_origin() {
    let merge = detail(InProgress::Merge, None);
    let rebase = detail(InProgress::Rebase, Some((3, 7)));
    let merge_rungs = super::stop_wordings(
        &stop(LiveOrigin::PreviousRun, false, InProgress::Merge, 1),
        Some((&merge, 1)),
    );
    assert_eq!(
        merge_rungs[1],
        "merge in progress · from last run · 1 conflicted"
    );
    let rebase_rungs = super::stop_wordings(
        &stop(LiveOrigin::PreviousRun, false, InProgress::Rebase, 13),
        Some((&rebase, 13)),
    );
    assert_eq!(
        rebase_rungs[1],
        "rebase in progress · from last run · step 3 of 7 · 13 conflicted"
    );
}

/// m6-conflicts/61: a stop from this run has no origin clause, so its
/// short-origin rung is the full rung with the short count, and the rung
/// that would drop the origin repeats it and is skipped.
#[test]
fn a_this_run_stop_has_no_duplicate_rung() {
    let merge = detail(InProgress::Merge, None);
    let wordings = super::stop_wordings(
        &stop(LiveOrigin::ThisRun, false, InProgress::Merge, 13),
        Some((&merge, 13)),
    );
    assert_eq!(
        wordings,
        [
            "merge stopped · 13 files conflicted",
            "merge stopped · 13 conflicted",
            "merge stopped",
        ]
    );
}

/// Every origin × step × count (0, 1, 13): each rung strictly narrower than
/// the one before (so none twice), and a count drawn whole or not at all.
#[test]
fn every_stop_ladder_narrows_and_keeps_its_count_whole() {
    let origins = [
        LiveOrigin::ThisRun,
        LiveOrigin::PreviousRun,
        LiveOrigin::OutsideGraft,
    ];
    for origin in origins {
        for (kind, step) in [
            (InProgress::Merge, None),
            (InProgress::Rebase, Some((3, 7))),
        ] {
            let read = detail(kind, step);
            for count in [0, 1, 13] {
                let wordings =
                    super::stop_wordings(&stop(origin, false, kind, count), Some((&read, count)));
                for pair in wordings.windows(2) {
                    assert!(
                        pair[1].chars().count() < pair[0].chars().count(),
                        "{origin:?} {kind:?} {count}: {wordings:?}"
                    );
                }
                let short = format!(" · {count} conflicted");
                let full = format!(
                    " · {count} {} conflicted",
                    if count == 1 { "file" } else { "files" }
                );
                for rung in &wordings {
                    let whole = count > 0 && (rung.ends_with(&short) || rung.ends_with(&full));
                    assert!(
                        whole || !rung.contains("conflicted"),
                        "{origin:?} {count}: {rung}"
                    );
                }
            }
        }
    }
}

/// m6-conflicts/74: a rebase that stopped with edits set aside. Its stop
/// line says so after the rest — ` · edits aside` — and that part is the
/// first a shorter wording drops. A stop whose Pending entry set nothing
/// aside words exactly as before.
#[test]
fn a_stop_with_edits_aside_says_so_and_drops_it_first() {
    use graft_core::journal::{EntryStatus, PutBack, PutBackWhy};

    let mut entry = done_applied().entry;
    entry.status = EntryStatus::Pending;
    entry.description = "rebase main onto origin/main".to_owned();
    let plain_entry = entry.clone();
    entry.put_back = Some(PutBack {
        stash: oid(0x5a),
        why: PutBackWhy::SetAside,
    });
    let with = |entry: graft_core::journal::JournalEntry| {
        let mut state = stopped(InProgress::Rebase, &["notes.md"]);
        state.stopped = Some(stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 1));
        state.live_entries = vec![entry.id.clone()];
        state.journal.entries = crate::update::detail::Loading::Ready(std::sync::Arc::new(
            crate::jobs::undo::JournalRead {
                entries: vec![entry],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            },
        ));
        state
    };

    let aside = with(entry);
    let ladder = stop_line_wordings(&aside).expect("a stop").wordings;
    assert_eq!(
        ladder[0],
        "rebase stopped at step 3 of 7 \u{b7} 1 file conflicted \u{b7} edits aside"
    );
    assert_eq!(
        ladder[1],
        "rebase stopped at step 3 of 7 \u{b7} 1 file conflicted"
    );
    assert!(
        ladder[1..].iter().all(|rung| !rung.contains("aside")),
        "{ladder:?}"
    );
    assert_eq!(stop_line(&aside).as_deref(), Some(ladder[0].as_str()));

    let plain = with(plain_entry);
    let plain_ladder = stop_line_wordings(&plain).expect("a stop").wordings;
    assert_eq!(plain_ladder, ladder[1..].to_vec());
}

fn oid(byte: u8) -> graft_core::oid::Oid {
    graft_core::oid::Oid::from_bytes(&[byte; graft_core::oid::Oid::SHA1_BYTES]).expect("20 bytes")
}

fn refs_at(head: graft_core::oid::Oid) -> crate::refs::RefBadges {
    crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(head),
        &std::collections::HashMap::new(),
        &[],
        |_, _| Err("no upstreams in this fixture".to_owned()),
    )
    .expect("a detached HEAD needs no count")
}

/// A rebase whose last step was resolved, `c` pressed and its `Done` in:
/// the refresh's two reads are named in `commands`.
fn continued_rebase() -> (AppState, JobId, JobId) {
    let id = JobId::new(7);
    let mut state = stopped(InProgress::Rebase, &[]);
    state.history = Some(crate::view::tests::history_of(5, 2));
    state.refs = Some(std::sync::Arc::new(refs_at(oid(5))));
    state.stopped = Some(stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 1));
    state.detail.shown = true;
    state.detail.focus = graft_ui::detail::DetailFocus::Files;
    state.on_working_tree = true;
    state.operating = Some(id);
    state.resuming = Some((id, Resume::Continue));
    let (state, commands) = crate::update::update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Done(
                done_applied(),
            ))),
        ),
    );
    let refs = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadRefs(id) => Some(*id),
            _ => None,
        })
        .expect("the refresh re-reads the refs");
    let walk = state.awaiting.expect("the refresh restarts the walk");
    (state, refs, walk)
}

#[test]
fn continue_lands_on_the_new_commit() {
    let (state, refs, walk) = continued_rebase();
    assert_eq!(state.status, Status::done("continue rebase"));
    assert_eq!(state.stopped, None);
    assert!(state.land_on_head);
    assert_eq!(
        state.detail.focus,
        graft_ui::detail::DetailFocus::Graph,
        "the graph is focused"
    );
    // The fresh walk starts, then the refs name the rebased tip.
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Started(walk, crate::update::Task::ReadHistory),
    );
    let new_head = oid(6);
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_at(new_head)))),
    );
    assert!(!state.land_on_head, "spent");
    // The walk answers the `Find` with rows holding the new tip.
    let asked = state.history.as_ref().expect("a walk").asked;
    let mut window = crate::view::tests::history_of(6, 0)
        .window
        .expect("the fixture has rows");
    window.request = asked;
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Progress(
            walk,
            crate::update::Progress::History(crate::history::HistoryUpdate::Window(window)),
        ),
    );
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(
        history.selected().map(|row| row.commit.id),
        Some(new_head),
        "the cursor row is the new HEAD"
    );
    assert!(!state.on_working_tree);
    assert!(state.graph_has_focus());
}

#[test]
fn a_stale_refs_answer_does_not_spend_the_landing() {
    let (state, refs, walk) = continued_rebase();
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Started(walk, crate::update::Task::ReadHistory),
    );
    // An older read — from before the `c` — lands after `Done`.
    let older_read = JobId::new(refs.raw() ^ 0x55);
    assert_ne!(older_read, refs);
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(older_read, Product::Refs(Ok(refs_at(oid(5))))),
    );
    assert!(state.land_on_head, "an older read leaves the landing owed");
    assert_eq!(state.head_landing.refs, Some(refs));
}

/// Review m6-amend-11-12 Minor 3's question: `Ctrl+R` straight after `c`
/// replaces the walk the landing waits on before it starts, and
/// `update::history_started` drops the superseded walk's `Started` before
/// `walk_started` sees it. The landing must still be spent — on the walk
/// that replaced it — not left owed for good.
#[test]
fn a_refresh_before_the_walk_starts_still_lands_on_head() {
    let (state, refs, first_walk) = continued_rebase();
    // `Ctrl+R` before the refresh's walk has started.
    let (state, commands) = crate::update::update(
        &state,
        AppEvent::Core(graft_core::event::Event::Key(KeyPress {
            code: KeyCode::Char('r'),
            modifiers: graft_core::event::Modifiers {
                control: true,
                ..graft_core::event::Modifiers::default()
            },
        })),
    );
    let second_walk = state.awaiting.expect("the refresh restarts the walk");
    assert_ne!(second_walk, first_walk, "{commands:?}");
    // The superseded walk's `Started` still arrives; the new one follows.
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Started(first_walk, crate::update::Task::ReadHistory),
    );
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Started(second_walk, crate::update::Task::ReadHistory),
    );
    let new_head = oid(6);
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_at(new_head)))),
    );
    assert!(!state.land_on_head, "spent, not owed for good");
    let asked = state.history.as_ref().expect("a walk").asked;
    let mut window = crate::view::tests::history_of(6, 0)
        .window
        .expect("the fixture has rows");
    window.request = asked;
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Progress(
            second_walk,
            crate::update::Progress::History(crate::history::HistoryUpdate::Window(window)),
        ),
    );
    assert_eq!(
        state
            .history
            .as_ref()
            .and_then(|history| history.selected())
            .map(|row| row.commit.id),
        Some(new_head),
        "the cursor row is the new HEAD"
    );
}

mod continue_in_git;
mod resume_words;
