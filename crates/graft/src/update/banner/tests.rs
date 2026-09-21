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

/// An `Applied` naming a finished `--continue`. Only `description` and
/// `refresh` are read by `resumed`; the rest is a plausible journal entry.
fn done_applied() -> graft_ops::apply::Applied {
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
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
        },
        refresh: graft_ops::apply::Refresh::Everything,
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
    assert_eq!(state.status, Status::Done("continue rebase".to_owned()));
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
        "⇥" => KeyCode::Tab,
        other => panic!(
            "the focused banner's hint bar draws `{other}`, which this test \
             does not know how to press — teach it what that key is meant to \
             do rather than letting the bar advertise it unchecked"
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
            None => assert!(
                matches!(commands, [Command::Resume(_, Resume::Continue)]),
                "`{key} {verb}` queued {commands:?}"
            ),
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
                matches!(confirm.then, ConfirmThen::Resume(what) if what == want),
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

/// What pressing `verb`'s key must have done. Every arm asserts the verb's
/// own outcome; an unknown verb is a failure, not a pass.
fn did_the_verb(verb: &str, key: &str, before: &AppState, after: &AppState, commands: &[Command]) {
    match verb {
        "continue" | "skip" | "abort" => did_a_resume_verb(verb, key, before, after, commands),
        "mergetool" | "edit" => did_a_file_verb(verb, key, before, commands),
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
/// (`update::tests::every_hinted_key_is_bound` and
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
