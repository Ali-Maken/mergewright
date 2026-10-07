//! Every `⇥` word names where `⇥` really lands (m6-conflicts/70), over
//! plain data: the one reading of the cycle (`banner::tab_target`) against
//! the key itself, the pane's bar, and the refusal's counted keys.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::state::{InProgress, InProgressDetail};
use graft_git::status::WorkingTreeStatus;
use graft_ui::detail::DetailFocus;
use graft_ui::graph_screen::{Divider, HintContext, hints_for, hints_text};

use crate::update::banner::{TabTarget, tab_target};
use crate::update::commit_editor::{Begun, live_operation, operation_refusal};
use crate::update::detail::DetailSpec;
use crate::update::{AppEvent, AppState, Size, Status, update};

/// Where the keyboard is before the press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum At {
    Graph,
    Files,
    Diff,
    Banner,
}

/// `kind` live (or nothing, `None`), begun by `begun`, the pane shown or
/// not, the keyboard `at`, `columns` wide.
fn state(kind: Option<(InProgress, Begun)>, shown: bool, at: At, columns: u16) -> AppState {
    let mut state = AppState {
        in_progress: kind.map(|(kind, _)| InProgressDetail {
            kind,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        size: Size { columns, rows: 40 },
        working_tree: Some(Arc::new(WorkingTreeStatus {
            entries: Vec::new(),
        })),
        ..AppState::default()
    };
    if kind.is_some_and(|(_, begun)| begun == Begun::InGraft) {
        state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    }
    assert_eq!(live_operation(&state), kind, "the fixture");
    if shown {
        state.on_working_tree = true;
        state.detail.shown = true;
        state.detail.spec = Some(DetailSpec::WorkingTree);
    }
    match at {
        At::Graph => state.detail.focus = DetailFocus::Graph,
        At::Files => state.detail.focus = DetailFocus::Files,
        At::Diff => state.detail.focus = DetailFocus::Diff,
        At::Banner => state.banner_focus = true,
    }
    state
}

fn press(state: &AppState, code: KeyCode) -> AppState {
    let key = KeyPress {
        code,
        modifiers: Modifiers::NONE,
    };
    update(state, AppEvent::Core(Event::Key(key))).0
}

/// Where the keyboard is, in [`TabTarget`]'s terms.
fn focus(state: &AppState) -> TabTarget {
    if state.banner_focus {
        return TabTarget::Banner;
    }
    match state.detail.focus {
        DetailFocus::Graph => TabTarget::Graph,
        DetailFocus::Files => TabTarget::Files,
        DetailFocus::Diff => TabTarget::Diff,
    }
}

/// Every cell of the scout's cycle table, at 80 and 120 columns: pressing
/// `⇥` lands where `tab_target` says. The cell with nothing live, the pane
/// hidden and the graph focused is asserted as whatever the key does:
/// `None`, `⇥` moves no focus there.
#[test]
fn tab_target_is_where_tab_lands() {
    let lives = [
        None,
        Some((InProgress::Rebase, Begun::InGraft)),
        Some((InProgress::Merge, Begun::InShell)),
    ];
    let mut cells = 0;
    for columns in [80, 120] {
        for kind in lives {
            for shown in [true, false] {
                let ats: &[At] = match (kind.is_some(), shown) {
                    (true, true) => &[At::Graph, At::Files, At::Diff, At::Banner],
                    (true, false) => &[At::Graph, At::Banner],
                    (false, true) => &[At::Graph, At::Files, At::Diff],
                    (false, false) => &[At::Graph],
                };
                for &at in ats {
                    let before = state(kind, shown, at, columns);
                    let target = tab_target(&before);
                    let after = press(&before, KeyCode::Tab);
                    let cell = format!("{kind:?} shown {shown} at {at:?} {columns} cols");
                    match target {
                        Some(target) => assert_eq!(focus(&after), target, "{cell}"),
                        None => assert_eq!(focus(&after), focus(&before), "{cell}"),
                    }
                    cells += 1;
                }
            }
        }
    }
    assert_eq!(cells, 2 * (2 * 4 + 2 * 2 + 3 + 1));
}

/// The changes pane's bar during a stop: `⇥ diff` from the file list,
/// `⇥ banner` from the diff; and `⇥ graph` from the diff with nothing live.
#[test]
fn the_diff_bar_names_the_banner_during_a_stop() {
    let bar = |state: &AppState| {
        let banner = crate::update::banner::banner_view(state);
        let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
        assert!(
            matches!(context, HintContext::ChangesPane { .. }),
            "{context:?}"
        );
        hints_text(
            &hints_for(
                &context,
                Divider::Shown,
                187,
                graft_ui::keymap::Keymap::shipped(),
            ),
            graft_ui::keymap::Keymap::shipped(),
        )
    };
    let stop = Some((InProgress::Rebase, Begun::InGraft));
    let files = bar(&state(stop, true, At::Files, 187));
    assert!(files.contains("⇥ diff"), "{files:?}");
    let diff = bar(&state(stop, true, At::Diff, 187));
    assert!(diff.contains("⇥ banner"), "{diff:?}");
    let quiet = bar(&state(None, true, At::Diff, 187));
    assert!(quiet.contains("⇥ graph"), "{quiet:?}");
}

/// The keys a refusal names, as the status line holds them.
fn refusal_keys(state: &AppState) -> String {
    match &state.status {
        Status::Sticky {
            keys: Some(keys), ..
        } => keys.clone(),
        other => panic!("not a refusal with keys: {other:?}"),
    }
}

/// A rebase stop, the file list focused, `a` (amend): the keys read
/// `⇥ ⇥ banner, then c continue  a abort`, and two `⇥` put the keyboard on
/// the banner.
#[test]
fn the_amend_refusal_counts_the_tabs_it_names() {
    let before = state(
        Some((InProgress::Rebase, Begun::InGraft)),
        true,
        At::Files,
        187,
    );
    let refused = press(&before, KeyCode::Char('a'));
    assert_eq!(
        refusal_keys(&refused),
        "⇥ ⇥ banner, then c continue  a abort"
    );
    let once = press(&refused, KeyCode::Tab);
    assert!(!once.banner_focus, "one ⇥ is the diff");
    let twice = press(&once, KeyCode::Tab);
    assert!(twice.banner_focus, "two ⇥ are the banner");
}

/// A merge begun in the shell, from the file list: `⇥ ⇥ banner, then c
/// finishes it`.
#[test]
fn a_shell_merge_refusal_counts_its_tabs() {
    let before = state(
        Some((InProgress::Merge, Begun::InShell)),
        true,
        At::Files,
        187,
    );
    let refused = press(&before, KeyCode::Char('a'));
    assert_eq!(refusal_keys(&refused), "⇥ ⇥ banner, then c finishes it");
}

/// A merge Graft began: `c finish`, never `c continue`, in the pane's
/// refusal and the editor's.
#[test]
fn a_graft_merge_refusal_says_c_finish() {
    let before = state(
        Some((InProgress::Merge, Begun::InGraft)),
        true,
        At::Files,
        187,
    );
    let refused = press(&before, KeyCode::Char('a'));
    assert_eq!(refusal_keys(&refused), "⇥ ⇥ banner, then c finish  a abort");
    let words = operation_refusal(
        (InProgress::Merge, Begun::InGraft),
        1,
        graft_ui::keymap::Keymap::shipped(),
    )
    .editor_words();
    assert_eq!(
        words,
        "merge in progress — esc, ⇥ banner, then c finish  a abort"
    );
}
