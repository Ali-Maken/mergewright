//! `c` in the graph or the changes pane while an operation is live
//! (m6-conflicts/52), over plain data: the key does what the banner's `c`
//! does where that acts, keeps M3a's words where it does not, and the bar
//! says which. The real-git half is `tests/it/resolver/shell_begun.rs`.

use crate::spelled::Spelled as _;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::{StatusEntry, WorkingTreeStatus};
use graft_ui::detail::DetailFocus;
use graft_ui::graph_screen::{Divider, Hint, HintContext, LiveC, hints_for};

use crate::update::commit_editor::{Begun, live_operation, operation_refusal};
use crate::update::detail::DetailSpec;
use crate::update::finish::ReturnTo;
use crate::update::{AppEvent, AppState, Command, Size, Status, update};

/// Where the keyboard is when `c` is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frame {
    /// The changes pane, its file list focused.
    Pane,
    /// A graph row, the detail pane closed: one `⇥` from the banner.
    Graph,
    /// A graph row with the detail pane open beside it (frame 2).
    GraphWithDetails,
}

const FRAMES: [Frame; 3] = [Frame::Pane, Frame::Graph, Frame::GraphWithDetails];

/// The table's five rows: each kind, and who began it.
const TABLE: [(InProgress, Begun); 5] = [
    (InProgress::Merge, Begun::InGraft),
    (InProgress::Merge, Begun::InShell),
    (InProgress::Rebase, Begun::InGraft),
    (InProgress::Rebase, Begun::InShell),
    (InProgress::Bisect, Begun::InShell),
];

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

/// `kind` live, begun by `begun`, with `paths` still unmerged, the banner
/// unfocused and the keyboard in `frame`, at the user's 187x50.
///
/// Asserts it is what it claims (find-and-pin 3): `live_operation` reads
/// "Graft began it" off `live_entries`, so that is checked, not trusted;
/// and a bisect has no banner.
fn live(kind: InProgress, begun: Begun, paths: &[&str], frame: Frame) -> AppState {
    let mut state = AppState {
        in_progress: Some(InProgressDetail {
            kind,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        size: Size {
            columns: 187,
            rows: 50,
        },
        working_tree: Some(Arc::new(WorkingTreeStatus {
            entries: paths.iter().map(|path| unmerged(path)).collect(),
        })),
        ..AppState::default()
    };
    if begun == Begun::InGraft {
        state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    }
    match frame {
        Frame::Pane => {
            state.on_working_tree = true;
            state.detail.shown = true;
            state.detail.spec = Some(DetailSpec::WorkingTree);
            state.detail.focus = DetailFocus::Files;
        }
        Frame::Graph => {}
        Frame::GraphWithDetails => {
            state.detail.shown = true;
            state.detail.spec = Some(DetailSpec::WorkingTree);
            state.detail.focus = DetailFocus::Graph;
        }
    }
    assert_eq!(live_operation(&state), Some((kind, begun)), "the fixture");
    assert_eq!(
        crate::update::banner::banner_view(&state).is_some(),
        kind != InProgress::Bisect,
        "a bisect has no banner; everything else has one"
    );
    assert_eq!(
        state.on_changes_pane() && !state.graph_has_focus(),
        frame == Frame::Pane
    );
    state
}

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    let key = KeyPress {
        code,
        modifiers: Modifiers::NONE,
    };
    update(state, AppEvent::Core(Event::Key(key)))
}

fn c(state: &AppState) -> (AppState, Vec<Command>) {
    press(state, KeyCode::Char('c'))
}

/// The bar this state draws, as the frame builds it.
fn bar(state: &AppState) -> (HintContext, Vec<Hint>) {
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    let divider = if state.detail.shown {
        Divider::Shown
    } else {
        Divider::Hidden
    };
    let hints = hints_for(
        &context,
        divider,
        state.size.columns,
        graft_ui::keymap::Keymap::shipped(),
    );
    (context, hints)
}

fn c_hint(hints: &[Hint]) -> Option<&'static str> {
    hints
        .iter()
        .find(|hint| hint.key() == "c")
        .map(|hint| hint.verb)
}

fn refuses_in_words_and_sends_nothing(kind: InProgress, begun: Begun, frame: Frame) {
    let state = live(kind, begun, &[], frame);
    let (after, commands) = c(&state);
    assert!(commands.is_empty(), "{frame:?}: {commands:?}");
    assert!(after.commit_editor.is_none());
    assert!(after.finish.is_none());
    let words = operation_refusal((kind, begun), 1, graft_ui::keymap::Keymap::shipped()).status();
    assert!(
        matches!(&words, Status::Sticky { reason, .. } if !reason.is_empty()),
        "{words:?}"
    );
    assert_eq!(after.status, words, "{frame:?}");
}

/// `c` on a rebase or cherry-pick the shell began hands the terminal to
/// `git <kind> --continue` (07c), from the pane and the graph alike, and
/// the bar offers it as `c continue` (`view::rows::live_c`).
fn hands_over_to_git(kind: InProgress) {
    for frame in FRAMES {
        let state = live(kind, Begun::InShell, &[], frame);
        let (after, commands) = c(&state);
        assert!(
            matches!(commands.as_slice(), [Command::ContinueInGit { kind: sent, .. }] if *sent == kind),
            "{frame:?}: {commands:?}"
        );
        assert!(after.commit_editor.is_none());
        assert_eq!(c_hint(&bar(&state).1), Some("continue"), "{frame:?}");
    }
}

#[test]
fn c_in_a_shell_rebase_hands_over_to_git() {
    hands_over_to_git(InProgress::Rebase);
}

#[test]
fn c_in_a_shell_cherry_pick_hands_over_to_git() {
    hands_over_to_git(InProgress::CherryPick);
}

#[test]
fn a_shell_begun_revert_is_still_refused() {
    refuses_in_words_and_sends_nothing(InProgress::Revert, Begun::InShell, Frame::Pane);
    refuses_in_words_and_sends_nothing(InProgress::Revert, Begun::InShell, Frame::Graph);
}

#[test]
fn c_in_the_pane_mid_bisect_refuses_in_words_and_sends_nothing() {
    refuses_in_words_and_sends_nothing(InProgress::Bisect, Begun::InShell, Frame::Pane);
}

/// Frame 1: mid-bisect, a graph row focused.
#[test]
fn c_in_the_graph_mid_bisect_refuses_in_words_and_sends_nothing() {
    refuses_in_words_and_sends_nothing(InProgress::Bisect, Begun::InShell, Frame::Graph);
    refuses_in_words_and_sends_nothing(InProgress::Bisect, Begun::InShell, Frame::GraphWithDetails);
}

#[test]
fn c_in_the_graph_opens_the_summary_on_a_graft_begun_merge() {
    let state = live(InProgress::Merge, Begun::InGraft, &[], Frame::Graph);
    let (after, commands) = c(&state);
    let finish = after.finish.as_ref().expect("the summary is open");
    assert_eq!(finish.return_to, ReturnTo::Banner);
    assert!(after.commit_editor.is_none());
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "{commands:?}"
    );
}

/// Frame 2: the detail pane open, the graph focused.
#[test]
fn c_in_the_graph_with_details_open_opens_the_summary() {
    let state = live(
        InProgress::Merge,
        Begun::InShell,
        &[],
        Frame::GraphWithDetails,
    );
    let (after, _) = c(&state);
    let finish = after.finish.as_ref().expect("the summary is open");
    assert_eq!(finish.return_to, ReturnTo::Banner);
    assert!(after.commit_editor.is_none());
}

/// The banner's `c` from the same state, the banner focused.
fn from_the_banner(state: &AppState) -> (AppState, Vec<Command>) {
    let mut focused = state.clone();
    focused.banner_focus = true;
    c(&focused)
}

#[test]
fn c_in_the_graph_continues_a_graft_begun_rebase() {
    let state = live(InProgress::Rebase, Begun::InGraft, &[], Frame::Graph);
    let (after, commands) = c(&state);
    let (banner, banner_commands) = from_the_banner(&state);
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Resume(_, graft_ops::apply::Resume::Continue, ..)
        )),
        "{commands:?}"
    );
    assert_eq!(commands, banner_commands);
    assert_eq!(after.status, banner.status);
    assert!(after.commit_editor.is_none());
}

/// The banner's `c` is called, not copied: from each state the table
/// names, the pane's `c` and the banner's send the same, say the same and
/// open the same summary.
#[test]
fn c_from_the_pane_and_from_the_banner_emit_the_same() {
    let mut rows = vec![
        live(InProgress::Merge, Begun::InGraft, &[], Frame::Pane),
        live(InProgress::Merge, Begun::InShell, &[], Frame::Pane),
        live(InProgress::Rebase, Begun::InGraft, &[], Frame::Pane),
        live(
            InProgress::Merge,
            Begun::InGraft,
            &["notes.md"],
            Frame::Pane,
        ),
    ];
    // The status read still out: `CHECKING`, from both.
    let mut owed = live(InProgress::Merge, Begun::InGraft, &[], Frame::Pane);
    owed.status_read = Some(graft_core::job::JobId::new(9));
    rows.push(owed);
    for state in rows {
        let (pane, pane_commands) = c(&state);
        let (banner, banner_commands) = from_the_banner(&state);
        let what = (
            state.in_progress.as_ref().map(|detail| detail.kind),
            &state.working_tree,
        );
        assert_eq!(pane_commands, banner_commands, "{what:?}");
        // A refusal names the keys acting where each `c` was pressed
        // (m8/10l): reason the same, keys each from its own place.
        match (&pane.status, &banner.status) {
            (
                Status::Sticky {
                    reason: p,
                    keys: pk,
                },
                Status::Sticky {
                    reason: b,
                    keys: bk,
                },
            ) if p.ends_with("still conflicted") => {
                assert_eq!(p, b, "{what:?}");
                for (s, keys) in [(&pane, pk), (&banner, bk)] {
                    let place = crate::update::edge::context_of(s);
                    assert_eq!(
                        keys,
                        &crate::update::banner::resolve_keys(place, &s.keymap),
                        "{what:?} at {place:?}"
                    );
                }
            }
            _ => assert_eq!(pane.status, banner.status, "{what:?}"),
        }
        assert_eq!(pane.finish, banner.finish, "{what:?}");
        assert!(pane.commit_editor.is_none());
    }
}

/// The bar has a `c` hint exactly where `c` sends a command or opens the
/// summary, and never `c commit` while an operation is live — over every
/// row of the table, in every frame `c` is pressed from.
#[test]
fn the_bar_offers_c_exactly_where_c_acts() {
    for (kind, begun) in TABLE {
        for frame in FRAMES {
            let state = live(kind, begun, &[], frame);
            let (context, hints) = bar(&state);
            let (after, commands) = c(&state);
            let acts = !commands.is_empty() || after.finish.is_some();
            let offered = c_hint(&hints);
            assert_eq!(
                offered.is_some(),
                acts,
                "{kind:?} {begun:?} {frame:?}: bar {context:?} offers {offered:?}; `c` sent {commands:?}"
            );
            assert_ne!(offered, Some("commit"), "{kind:?} {begun:?} {frame:?}");
            let want = match (kind, acts) {
                (_, false) => None,
                (InProgress::Merge, true) => Some("finish"),
                (_, true) => Some("continue"),
            };
            assert_eq!(offered, want, "{kind:?} {begun:?} {frame:?}");
        }
    }
    // And with nothing live, `c commit` as before.
    let mut idle = live(InProgress::Merge, Begun::InShell, &[], Frame::Pane);
    idle.in_progress = None;
    assert_eq!(c_hint(&bar(&idle).1), Some("commit"));
}

/// `⇥ banner` is drawn only where one `⇥` reaches the banner: not in
/// frame 2, not mid-bisect (there is no banner); and where it is drawn,
/// `⇥` focuses the banner.
#[test]
fn tab_banner_is_hinted_only_where_one_tab_reaches_it() {
    let way_back = |hints: &[Hint]| {
        hints
            .iter()
            .any(|hint| (hint.key(), hint.verb) == ("⇥", "banner"))
    };

    let details = live(
        InProgress::Merge,
        Begun::InGraft,
        &[],
        Frame::GraphWithDetails,
    );
    assert!(!way_back(&bar(&details).1));
    let bisect = live(InProgress::Bisect, Begun::InShell, &[], Frame::Graph);
    assert!(!way_back(&bar(&bisect).1));

    let plain = live(InProgress::Merge, Begun::InGraft, &[], Frame::Graph);
    assert!(way_back(&bar(&plain).1));
    let (after, _) = press(&plain, KeyCode::Tab);
    assert!(after.banner_focus, "`⇥` lands on the banner");
}

/// The plain row, with no verbs: all a bar with no drawn rows can say.
const PLAIN: HintContext = HintContext::PlainRow {
    verbs: graft_ui::graph_screen::hints::RowVerbs::NONE,
};

/// Find-and-pin 2, after decision B: no graph-row bar is left bare while an
/// operation is live — bisect, frame 2 and the plain one-`⇥` frame are all
/// wrapped, each with its own `tab_to_banner` — and none is wrapped with
/// nothing live.
#[test]
fn no_graph_row_context_is_unwrapped_while_live() {
    let wrapped = |state: &AppState| match bar(state).0 {
        HintContext::OperationLive {
            row,
            c,
            tab_to_banner,
            ..
        } => Some((*row, c, tab_to_banner)),
        _ => None,
    };
    let bisect = live(InProgress::Bisect, Begun::InShell, &[], Frame::Graph);
    assert_eq!(wrapped(&bisect), Some((PLAIN, LiveC::Refuses, false)));
    let details = live(
        InProgress::Merge,
        Begun::InGraft,
        &[],
        Frame::GraphWithDetails,
    );
    assert_eq!(wrapped(&details), Some((PLAIN, LiveC::Finish, false)));
    let plain = live(InProgress::Merge, Begun::InGraft, &[], Frame::Graph);
    assert_eq!(wrapped(&plain), Some((PLAIN, LiveC::Finish, true)));
    // The opening view, before any history lands: wrapped too.
    assert!(plain.history.is_none());
    assert!(matches!(
        crate::view::rows::graph_view(&plain, 0).hint_context,
        HintContext::OperationLive {
            c: LiveC::Finish,
            ..
        }
    ));
    let mut idle = plain;
    idle.in_progress = None;
    idle.live_entries.clear();
    assert_eq!(wrapped(&idle), None);
    assert_eq!(bar(&idle).0, PLAIN);
}

/// `a` is unchanged: it refuses in words in every frame, for every row of
/// the table, and never aborts.
#[test]
fn a_mid_operation_still_refuses_in_words_on_plain_data() {
    for (kind, begun) in TABLE {
        for frame in FRAMES {
            let state = live(kind, begun, &[], frame);
            let (after, commands) = press(&state, KeyCode::Char('a'));
            assert!(commands.is_empty(), "{kind:?} {frame:?}: {commands:?}");
            assert!(after.confirm.is_none(), "no abort box");
            // The count is the state's own (m6-conflicts/70).
            let tabs = crate::update::commit_editor::refusal_tabs(&state);
            assert_eq!(
                after.status,
                operation_refusal((kind, begun), tabs, graft_ui::keymap::Keymap::shipped())
                    .status()
            );
        }
    }
}

/// Find-and-pin 2 (m6-conflicts/50): a box over a live operation — the
/// banner's own skip and abort boxes — draws the box's bar, bare, never
/// wrapped in `OperationLive`: its `c` hint would name a key the box
/// swallows. Every row of the table, every frame.
#[test]
fn the_bar_over_a_box_during_a_live_operation_is_the_box_s_own() {
    use graft_ui::graph_screen::ConfirmYes;
    for (kind, begun) in TABLE {
        for frame in FRAMES {
            for (label, what, yes) in [
                ("Abort", graft_ops::apply::Resume::Abort, ConfirmYes::Abort),
                ("Skip", graft_ops::apply::Resume::Skip, ConfirmYes::Skip),
            ] {
                let mut state = live(kind, begun, &["a.rs"], frame);
                state.confirm = Some(crate::update::confirm::PendingConfirm {
                    view: graft_ui::confirm::ConfirmView::new(
                        graft_term::sanitize_line("Abort?"),
                        Vec::new(),
                        graft_term::sanitize_line("git merge --abort"),
                    )
                    .with_labels("Cancel", label),
                    then: crate::update::confirm::ConfirmThen::Resume(what),
                });
                let (context, hints) = bar(&state);
                assert_eq!(
                    context,
                    HintContext::ConfirmBox { yes: Some(yes) },
                    "{kind:?} {begun:?} {frame:?}"
                );
                assert_eq!(c_hint(&hints), None, "{kind:?} {begun:?} {frame:?}");
                assert_eq!(hints.first().map(crate::spelled::Spelled::key), Some("y"));
            }
        }
    }
}

/// Review m6-look-finish-wave2 m1: `c` while a write runs, on a rebase
/// Graft began, is refused in the summary's words and sends nothing — from
/// the pane, both graph frames and the banner alike. Never silent (rule 3).
#[test]
fn c_while_a_write_runs_refuses_in_words() {
    for frame in FRAMES {
        let mut state = live(InProgress::Rebase, Begun::InGraft, &[], frame);
        state.operating = Some(graft_core::job::JobId::new(7));
        for (from, (after, commands)) in
            vec![("key", c(&state)), ("banner", from_the_banner(&state))].into_boxed_slice()
        {
            assert!(commands.is_empty(), "{frame:?} {from}: {commands:?}");
            assert_eq!(
                after.status,
                Status::sticky("a write is already running"),
                "{frame:?} {from}"
            );
            assert_eq!(after.operating, state.operating, "{frame:?} {from}");
        }
    }
}
