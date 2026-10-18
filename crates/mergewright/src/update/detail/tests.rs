use std::sync::Arc;

use super::state::{MAX_LINES, RESIZE_STEP};
use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_core::mistake_radar::MistakeRadarKind;
use mergewright_core::oid::Oid;
use mergewright_git::diff::DiffOptions;
use mergewright_ops::mistake_radar::Policy;
use mergewright_ops::mistake_radar::probes::EvilVerdict;
use mergewright_term::sanitize::sanitize_line;
use mergewright_ui::detail::{DetailFocus, DiffMode, DiffView, FileListView, LineKindView};
use mergewright_ui::detail_pane::DetailLayout;

use crate::update::events::Command;
use crate::update::state::AppState;

use super::*;
use mergewright_git::diff::FileList;
use mergewright_ui::detail::{
    FileRow, FileRowKind, FileStatusView, HunkView, LineView, WorkingTreeSection,
};

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn next_id_from(counter: &mut u64) -> JobId {
    *counter += 1;
    JobId::new(LOCAL_JOB_BIT | *counter)
}

/// `history_of(3, 0)` with a ready, empty radar context, so
/// `findings_for` can run without a real repository: `AppState` for
/// the `open_explain`/`close_explain`/Esc-priority tests
/// (`m2-details-and-diffs/17`).
fn state_with_duplicate_finding(mark: Oid, copy: Oid) -> AppState {
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        ..AppState::default()
    };
    state.radar.context = Loading::Ready(Arc::new(crate::jobs::radar::RadarContext::of_ids(
        [],
        Vec::new(),
    )));
    state.radar.duplicate_findings.insert(
        mark,
        mergewright_ops::mistake_radar::Finding {
            kind: MistakeRadarKind::DuplicateCommit,
            severity: mergewright_core::mistake_radar::Severity::Warning,
            at: mark,
            evidence: mergewright_ops::mistake_radar::Evidence::Duplicate { copies: vec![copy] },
        },
    );
    state
}

/// `open_explain` on a marked, selected commit resolves its finding,
/// opens the pane on it, and fills `explain`/`explain_highlight`.
#[test]
fn open_explain_resolves_the_selected_rows_finding() {
    let cursor = oid(3); // `history_of(3, 0)`'s cursor row.
    let mut state = state_with_duplicate_finding(cursor, oid(9));
    state.detail.explain_requested = Some(cursor);
    let mut counter = 0;

    let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));

    assert_eq!(state.detail.explain_requested, None);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(cursor)));
    let explain = state.detail.explain.expect("a finding was found");
    assert!(!explain.title.as_str().is_empty());
    assert!(!state.detail.explain_highlight.is_empty());
}

/// A stale request — the marked commit no longer has a finding — opens
/// the pane (the commit is still real) but leaves `explain` alone
/// rather than erroring.
#[test]
fn open_explain_with_no_finding_opens_the_pane_but_not_explain() {
    let cursor = oid(3);
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        ..AppState::default()
    };
    state.radar.context = Loading::Ready(Arc::new(crate::jobs::radar::RadarContext::of_ids(
        [],
        Vec::new(),
    )));
    state.detail.explain_requested = Some(cursor);
    let mut counter = 0;

    let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));

    assert_eq!(state.detail.explain_requested, None);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(cursor)));
    assert!(state.detail.explain.is_none());
}

/// Nothing requested this tick: a no-op, no commands.
#[test]
fn open_explain_with_nothing_requested_is_a_no_op() {
    let mut state = AppState::default();
    let mut counter = 0;

    let commands = open_explain(&mut state, &mut || next_id_from(&mut counter));

    assert!(commands.is_empty());
    assert!(state.detail.explain.is_none());
}

/// `close_explain` clears the view and the highlight, and nothing else.
#[test]
fn close_explain_clears_the_view_and_highlight() {
    let mut state = state_with_duplicate_finding(oid(3), oid(9));
    state.detail.explain_requested = Some(oid(3));
    let mut counter = 0;
    let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));
    assert!(state.detail.explain.is_some());

    close_explain(&mut state);

    assert!(state.detail.explain.is_none());
    assert!(state.detail.explain_highlight.is_empty());
}

/// `Esc` closes Explain first, on its own, leaving the pane open on the
/// same commit; a second `Esc` then falls through to closing the pane.
#[test]
fn escape_closes_explain_before_anything_else_esc_does() {
    let mut state = state_with_duplicate_finding(oid(3), oid(9));
    state.detail.explain_requested = Some(oid(3));
    let mut counter = 0;
    let mut commands = Vec::new();
    let _ = open_explain(&mut state, &mut || next_id_from(&mut counter));
    assert!(state.detail.explain.is_some());
    state.detail.shown = true;

    let handled = detail_key(&mut state, &mut commands, KeyPress::plain(KeyCode::Escape));

    assert!(handled);
    assert!(state.detail.explain.is_none(), "first Esc closes Explain");
    assert!(state.detail.shown, "the pane itself stays open");

    let handled_again = detail_key(&mut state, &mut commands, KeyPress::plain(KeyCode::Escape));
    assert!(handled_again);
    assert!(!state.detail.shown, "second Esc closes the pane");
}

/// `toggle_compare`'s first branch: no base yet pins one, no command.
/// A commit spec probes once; the same merge asked about again is
/// answered from the cache without a second `git` spawn.
#[test]
fn a_commit_spec_probes_once_and_a_revisit_is_a_cache_hit() {
    let mut detail = DetailState::default();
    let policy = Policy::default();
    let mut counter = 0;
    let mut ids = || next_id_from(&mut counter);

    let commands = detail.probe_evil(Some(oid(1)), &policy, &mut ids);
    let [Command::EvilProbe(id, probed)] = commands.as_slice() else {
        panic!("a fresh merge asks for one probe, got {commands:?}");
    };
    assert_eq!(*probed, oid(1));
    assert_eq!(detail.evil, Loading::Pending(*id));

    detail.evil_probed(*id, oid(1), EvilVerdict::Clean);
    assert_eq!(detail.evil, Loading::Ready(EvilVerdict::Clean));

    // Away to another commit, then back.
    let away = detail.probe_evil(Some(oid(2)), &policy, &mut ids);
    assert!(matches!(away.as_slice(), [Command::EvilProbe(..)]));
    let back = detail.probe_evil(Some(oid(1)), &policy, &mut ids);
    assert!(
        back.iter()
            .all(|command| !matches!(command, Command::EvilProbe(..))),
        "a cached verdict asks for nothing, got {back:?}"
    );
    assert_eq!(detail.evil, Loading::Ready(EvilVerdict::Clean));
}

/// Moving off a merge before its probe answers cancels it, and the late
/// answer changes nothing.
#[test]
fn moving_off_a_pending_probe_cancels_it_and_drops_its_late_answer() {
    let mut detail = DetailState::default();
    let policy = Policy::default();
    let mut counter = 0;
    let mut ids = || next_id_from(&mut counter);

    let first = detail.probe_evil(Some(oid(1)), &policy, &mut ids);
    let [Command::EvilProbe(stale, _)] = first.as_slice() else {
        panic!("expected one probe, got {first:?}");
    };
    let stale = *stale;

    let moved = detail.probe_evil(Some(oid(2)), &policy, &mut ids);
    assert!(
        moved.contains(&Command::CancelJob(stale)),
        "the superseded probe is cancelled, got {moved:?}"
    );

    detail.evil_probed(stale, oid(1), EvilVerdict::Evil { files: vec![] });
    assert!(
        matches!(detail.evil, Loading::Pending(id) if id != stale),
        "the stale answer is dropped, leaving the newer probe pending"
    );
    assert!(detail.evil_cache.get(&oid(1)).is_none());
}

/// Nothing is probed with the evil-merge kind off or with nothing
/// selected — but a hidden pane is no longer a reason not to probe: the
/// probe follows the selection so a selected evil merge is flagged in
/// the graph (M2 review M3).
#[test]
fn no_probe_with_the_kind_off_or_nothing_selected_but_a_hidden_pane_still_probes() {
    let mut counter = 0;
    let mut ids = || next_id_from(&mut counter);
    let mut off = Policy::default();
    off.set(MistakeRadarKind::EvilMerge, None);

    let mut detail = DetailState::default();
    assert_eq!(detail.probe_evil(Some(oid(1)), &off, &mut ids), Vec::new());
    assert_eq!(detail.evil, Loading::Idle);

    let policy = Policy::default();
    assert_eq!(detail.probe_evil(None, &policy, &mut ids), Vec::new());
    assert_eq!(detail.evil, Loading::Idle);

    assert!(!detail.shown, "the pane is closed");
    let probes = detail.probe_evil(Some(oid(1)), &policy, &mut ids);
    assert!(
        matches!(probes.as_slice(), [Command::EvilProbe(_, at)] if *at == oid(1)),
        "a closed pane still probes the selected merge, got {probes:?}"
    );
}

/// A refresh keeps the verdicts: a merge's tree is immutable, so no
/// answer can go stale.
#[test]
fn a_refresh_keeps_the_evil_cache() {
    let mut detail = DetailState::default();
    let mut counter = 0;
    detail.evil_cache.insert(oid(1), EvilVerdict::Clean);
    let _ = detail.refresh(&mut || next_id_from(&mut counter));
    assert_eq!(detail.evil_cache.get(&oid(1)), Some(&EvilVerdict::Clean));
}

/// m7-14: the first `x` pins the base **and** opens the pane on it, so
/// journey `pr-contents` is `x`, motion from a closed pane.
#[test]
fn the_first_x_opens_the_compare_on_the_base() {
    let mut state = super::super::state::AppState::default();
    let mut ids = 0u64;
    let commands = toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
    assert!(!commands.is_empty(), "the compare on the base loads");
    assert!(state.detail.shown);
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare {
            base: oid(1),
            head: oid(1)
        })
    );
    assert_eq!(
        state.compare_base,
        Some(CompareBase {
            oid: oid(1),
            row: 0
        })
    );
}

/// `toggle_compare`'s second branch: the same oid again clears it, no
/// command.
#[test]
fn toggle_compare_on_the_same_oid_clears_it_and_emits_nothing() {
    let mut state = super::super::state::AppState::default();
    let mut ids = 0u64;
    toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
    let commands = toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
    assert!(commands.is_empty());
    assert_eq!(state.compare_base, None);
}

/// `toggle_compare`'s third branch: a different oid opens the compare
/// and keeps the base set.
#[test]
fn toggle_compare_on_a_different_oid_opens_compare_and_keeps_the_base() {
    let mut state = super::super::state::AppState::default();
    let mut ids = 0u64;
    toggle_compare(&mut state, 0, oid(1), &mut || next_id_from(&mut ids));
    let commands = toggle_compare(&mut state, 1, oid(2), &mut || next_id_from(&mut ids));
    assert!(!commands.is_empty());
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare {
            base: oid(1),
            head: oid(2)
        })
    );
    assert_eq!(
        state.compare_base,
        Some(CompareBase {
            oid: oid(1),
            row: 0
        }),
        "the base stays live as the cursor moves"
    );
}

/// `Esc` clears a pinned base and closes the pane in the same press
/// (ADR-0023's existing `Escape` order, m2-details-and-diffs/11).
#[test]
fn escape_clears_a_pinned_base_and_closes_the_pane_in_one_press() {
    let mut state = super::super::state::AppState {
        compare_base: Some(CompareBase {
            oid: oid(1),
            row: 0,
        }),
        ..Default::default()
    };
    state.detail.shown = true;
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Escape)
    ));

    assert_eq!(state.compare_base, None);
    assert!(!state.detail.shown);
}

/// `Esc` clears a pinned base even while the pane never opened (`x`
/// pins a base with no command, so the pane can be hidden the whole
/// time).
#[test]
fn escape_clears_a_pinned_base_while_the_pane_is_hidden() {
    let mut state = super::super::state::AppState {
        compare_base: Some(CompareBase {
            oid: oid(1),
            row: 0,
        }),
        ..Default::default()
    };
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Escape)
    ));

    assert_eq!(state.compare_base, None);
    assert!(!state.detail.shown);
}

/// An empty, valid diff — enough for the option/scroll keys to act on.
pub(crate) fn empty_diff_view() -> DiffView {
    DiffView {
        path: sanitize_line("f"),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        untracked: false,
    }
}

#[test]
fn angle_keys_resize_the_right_pane_and_leave_below_alone() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = super::super::state::Size {
        columns: 200,
        rows: 50,
    };
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('>'))
    ));

    let grown = state.detail.pane_size.right_columns.expect("resized once");
    assert!(grown > 0);
    assert_eq!(state.detail.pane_size.below_rows, None);
    assert_eq!(commands, vec![Command::SaveLayout(state.detail.pane_size)]);

    commands.clear();
    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('<'))
    ));
    let shrunk = state.detail.pane_size.right_columns.expect("still set");
    assert_eq!(shrunk, grown - RESIZE_STEP);
}

#[test]
fn angle_keys_resize_rows_in_the_below_layout() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = super::super::state::Size {
        columns: 120,
        rows: 50,
    };
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('>'))
    ));

    assert!(state.detail.pane_size.below_rows.is_some());
    assert_eq!(state.detail.pane_size.right_columns, None);
}

#[test]
fn angle_keys_are_a_no_op_in_the_tab_layout() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = super::super::state::Size {
        columns: 80,
        rows: 50,
    };
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('>'))
    ));

    assert_eq!(state.detail.pane_size, PaneSize::default());
    assert!(commands.is_empty());
}

#[test]
fn angle_keys_are_a_no_op_with_the_pane_hidden() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = false;
    state.size = super::super::state::Size {
        columns: 200,
        rows: 50,
    };
    let mut commands = Vec::new();

    assert!(!detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('>'))
    ));
    assert_eq!(state.detail.pane_size, PaneSize::default());
    assert!(commands.is_empty());
}

#[test]
fn plus_and_minus_never_resize_the_pane() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = super::super::state::Size {
        columns: 200,
        rows: 50,
    };
    let mut commands = Vec::new();

    detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('+')),
    );
    detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('-')),
    );

    assert_eq!(state.detail.pane_size, PaneSize::default());
}

#[test]
fn set_pane_cells_and_for_layout_round_trip() {
    let size = (200, 50);
    let layout = DetailLayout::compute(size.0, size.1, DetailFocus::Files, None);
    let mut state = DetailState {
        shown: true,
        ..DetailState::default()
    };

    assert!(state.set_pane_cells(&layout, 60, size));
    assert_eq!(
        state.pane_size.for_layout(&layout),
        state.pane_size.right_columns
    );
    assert_eq!(state.pane_size.right_columns, Some(60));
}

#[test]
fn bounded_forgets_its_oldest_entry_and_replaces_in_place() {
    let mut map = Bounded::new(2);
    map.insert(1, "a");
    map.insert(2, "b");
    map.insert(1, "a2");
    map.insert(3, "c");
    assert_eq!(map.get(&2), None);
    assert_eq!(map.get(&1), Some(&"a2"));
    assert_eq!(map.get(&3), Some(&"c"));
    map.clear();
    assert_eq!(map.get(&3), None);
}

#[test]
fn v_toggles_side_by_side_and_back_but_is_ignored_from_graph_focus() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    let mut commands = Vec::new();
    let key = KeyPress::plain(KeyCode::Char('v'));

    assert!(detail_key(&mut state, &mut commands, key));
    assert_eq!(state.detail.mode, DiffMode::SideBySide);
    assert!(commands.is_empty());

    assert!(detail_key(&mut state, &mut commands, key));
    assert_eq!(state.detail.mode, DiffMode::Unified);

    state.detail.focus = DetailFocus::Graph;
    assert!(!detail_key(&mut state, &mut commands, key));
    assert_eq!(state.detail.mode, DiffMode::Unified);
}

#[test]
fn focus_cycles_both_ways() {
    let mut state = DetailState::default();
    state.cycle_focus(true);
    assert_eq!(state.focus, DetailFocus::Files);
    state.cycle_focus(true);
    state.cycle_focus(true);
    assert_eq!(state.focus, DetailFocus::Graph);
    state.cycle_focus(false);
    assert_eq!(state.focus, DetailFocus::Diff);
}

#[test]
fn enter_is_a_no_op_once_diff_already_has_focus() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    let mut commands = Vec::new();
    let key = KeyPress::plain(KeyCode::Enter);

    assert!(detail_key(&mut state, &mut commands, key));
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    assert!(commands.is_empty());
}

#[test]
fn bracket_keys_step_context_and_reissue_read_file_diff() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    state.detail.diff = Loading::Ready(Arc::new(empty_diff_view()));
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char(']'))
    ));
    assert_eq!(
        state.detail.options.context,
        DiffOptions::default().context + 1
    );

    commands.clear();
    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('['))
    ));
    assert_eq!(state.detail.options.context, DiffOptions::default().context);
}

#[test]
fn w_toggles_ignore_whitespace_and_capital_w_toggles_word_tint_locally() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    state.detail.diff = Loading::Ready(Arc::new(empty_diff_view()));
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('w'))
    ));
    assert!(state.detail.options.ignore_whitespace);

    let before = state.detail.word_tint;
    commands.clear();
    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('W'))
    ));
    assert_eq!(state.detail.word_tint, !before);
    assert!(
        commands.is_empty(),
        "W is a local display toggle, no re-fetch"
    );
}

#[test]
fn capital_l_loads_the_full_file() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    state.detail.diff = Loading::Ready(Arc::new(empty_diff_view()));
    state.detail.max_lines = MAX_LINES;
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('L'))
    ));
    assert_eq!(state.detail.max_lines, usize::MAX);
}

#[test]
fn moving_the_file_cursor_resets_load_all() {
    // Review M5 (m2-details-and-diffs round 1b): `L` used to stick for
    // the rest of the session; it must apply only to the file it was
    // pressed on.
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.detail.max_lines = usize::MAX; // as if `L` was just pressed
    let mut files = FileListView::default();
    files.rows.push(FileRow {
        depth: 0,
        name: sanitize_line("a.rs"),
        kind: FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        },
        entry: Some(0),
    });
    files.rows.push(FileRow {
        depth: 0,
        name: sanitize_line("b.rs"),
        kind: FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        },
        entry: Some(1),
    });
    files.cursor = 0;
    state.detail.files = Loading::Ready(Arc::new(files));
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('j'))
    ));
    assert_eq!(state.detail.max_lines, MAX_LINES);
}

#[test]
fn space_folds_a_directory_row_in_the_files_list() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    let mut files = FileListView::default();
    files.rows.push(FileRow {
        depth: 0,
        name: sanitize_line("src/"),
        kind: FileRowKind::Dir { collapsed: false },
        entry: None,
    });
    files.rows.push(FileRow {
        depth: 1,
        name: sanitize_line("lib.rs"),
        kind: FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        },
        entry: Some(0),
    });
    files.cursor = 0;
    state.detail.files = Loading::Ready(Arc::new(files));
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char(' '))
    ));
    let Loading::Ready(files) = &state.detail.files else {
        unreachable!()
    };
    assert_eq!(files.rows[0].kind, FileRowKind::Dir { collapsed: true });
}

#[test]
fn g_and_capital_g_jump_the_diff_body_to_its_ends() {
    let mut state = super::super::state::AppState::default();
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    state.detail.diff = Loading::Ready(Arc::new(DiffView {
        hunks: vec![HunkView {
            heading: sanitize_line("@@ -1,3 +1,3 @@"),
            lines: (0..5)
                .map(|_| LineView {
                    kind: LineKindView::Context,
                    old_no: None,
                    new_no: None,
                    text: sanitize_line(""),
                    words: Vec::new(),
                    syntax: Vec::new(),
                    pair: None,
                })
                .collect(),
        }],
        ..empty_diff_view()
    }));
    let mut commands = Vec::new();

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('G'))
    ));
    assert!(state.detail.body_top > 0);

    assert!(detail_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Char('g'))
    ));
    assert_eq!(state.detail.body_top, 0);
}

/// A changes-pane file list: `sections` in order, each with its files.
/// Returns the view and the git list its rows index into.
pub(super) fn changes_pane(sections: &[(WorkingTreeSection, &[&str])]) -> (FileListView, FileList) {
    use mergewright_git::diff::{FileEntry, FileStatus};

    let mut rows = Vec::new();
    let mut files = Vec::new();
    for (section, paths) in sections {
        rows.push(FileRow {
            depth: 0,
            name: sanitize_line(section.title()),
            kind: FileRowKind::Section {
                section: *section,
                count: paths.len(),
            },
            entry: None,
        });
        for path in *paths {
            rows.push(FileRow {
                depth: 1,
                name: sanitize_line(path),
                kind: FileRowKind::File {
                    status: FileStatusView::Modified,
                    added: Some(1),
                    removed: Some(0),
                },
                entry: Some(u32::try_from(files.len()).expect("a test list is short")),
            });
            files.push(FileEntry {
                path: (*path).as_bytes().to_vec(),
                old_path: None,
                status: FileStatus::Modified,
                added: Some(1),
                removed: Some(0),
            });
        }
    }
    let count = u32::try_from(files.len()).expect("a test list is short");
    let view = FileListView {
        rows,
        cursor: 1,
        top: 0,
        files: count,
        added: 1,
        removed: 0,
        message: Vec::new(),
        compare: None,
    };
    (
        view,
        FileList {
            files,
            added: 1,
            removed: 0,
        },
    )
}

/// A changes pane showing `sections`, cursor on row `cursor`, line cursor at
/// `body`.
pub(crate) fn pane_on(
    sections: &[(WorkingTreeSection, &[&str])],
    cursor: usize,
    body: usize,
) -> DetailState {
    let (mut view, list) = changes_pane(sections);
    view.cursor = cursor;
    DetailState {
        shown: true,
        spec: Some(DetailSpec::WorkingTree),
        files: Loading::Ready(Arc::new(view)),
        list: Some(Arc::new(list)),
        body_cursor: body,
        ..DetailState::default()
    }
}

/// Run `refresh` and hand back the list a write would produce, as the job
/// would. Returns the pane after both.
fn refresh_into(mut detail: DetailState, after: &[(WorkingTreeSection, &[&str])]) -> DetailState {
    let mut counter = 0;
    let commands = detail.refresh(&mut || next_id_from(&mut counter));
    let Some(Command::ListFiles(id, _, _)) = commands.first() else {
        panic!("a refresh asks for the file list: {commands:?}");
    };
    let id = *id;
    let (view, list) = changes_pane(after);
    detail.files_arrived(id, Ok((Arc::new(view), Arc::new(list))), &mut || {
        next_id_from(&mut counter)
    });
    detail
}

#[test]
fn a_write_leaves_the_changes_cursor_on_the_same_file() {
    // m3/04 review M7: every write reloads the list and the fresh one starts
    // on the first file, usually under STAGED — so the next `Space`, meant to
    // keep staging, unstaged something else instead.
    let before = [
        (WorkingTreeSection::Staged, &["a.rs"][..]),
        (WorkingTreeSection::Unstaged, &["b.rs", "c.rs"][..]),
    ];
    // Row 4 is c.rs: heading, a.rs, heading, b.rs, c.rs.
    let detail = refresh_into(pane_on(&before, 4, 6), &before);
    let Loading::Ready(files) = &detail.files else {
        panic!("the list arrived");
    };
    assert_eq!(files.cursor, 4);
    assert_eq!(files.selected_section(), Some(WorkingTreeSection::Unstaged));
    assert_eq!(detail.body_cursor, 6, "the line cursor comes back too");
    assert!(detail.kept_cursor.is_none(), "the note is spent, not kept");
}

#[test]
fn staging_the_file_under_the_cursor_keeps_the_cursor_in_its_own_group() {
    // c.rs has moved to STAGED. Following it there would put the cursor on a
    // row whose `Space` means *unstage* — the direction flip M7 names. The
    // user is still working in UNSTAGED, so the cursor stays there.
    let before = [
        (WorkingTreeSection::Staged, &["a.rs"][..]),
        (WorkingTreeSection::Unstaged, &["b.rs", "c.rs"][..]),
    ];
    let after = [
        (WorkingTreeSection::Staged, &["a.rs", "c.rs"][..]),
        (WorkingTreeSection::Unstaged, &["b.rs"][..]),
    ];
    let detail = refresh_into(pane_on(&before, 4, 6), &after);
    let Loading::Ready(files) = &detail.files else {
        panic!("the list arrived");
    };
    assert_eq!(files.selected_section(), Some(WorkingTreeSection::Unstaged));
    let entry = files.selected_entry().expect("a file row");
    let list = detail.list.as_ref().expect("a list");
    assert_eq!(list.files[entry as usize].path, b"b.rs");
    assert_eq!(
        detail.body_cursor, 0,
        "a different file's diff has no row to come back to"
    );
}

#[test]
fn a_group_that_has_gone_leaves_the_fresh_lists_own_cursor() {
    let before = [
        (WorkingTreeSection::Staged, &["a.rs"][..]),
        (WorkingTreeSection::Unstaged, &["b.rs"][..]),
    ];
    let after = [(WorkingTreeSection::Staged, &["a.rs", "b.rs"][..])];
    let detail = refresh_into(pane_on(&before, 3, 2), &after);
    let Loading::Ready(files) = &detail.files else {
        panic!("the list arrived");
    };
    assert_eq!(files.cursor, 1);
    assert_eq!(files.selected_section(), Some(WorkingTreeSection::Staged));
}

/// 30n m6: `Ctrl+A` with nothing stageable says so; it never fell through
/// to anything (it has no other meaning on the pane).
#[test]
fn ctrl_a_with_nothing_to_stage_says_so() {
    let mut state = AppState {
        detail: pane_on(&[(WorkingTreeSection::Unstaged, &["a.rs"][..])], 1, 0),
        ..AppState::default()
    };
    state.detail.focus = DetailFocus::Files;
    state.working_tree = Some(Arc::new(mergewright_git::status::WorkingTreeStatus::default()));
    let mut commands = Vec::new();
    let ctrl_a = KeyPress {
        code: KeyCode::Char('a'),
        modifiers: mergewright_core::event::Modifiers::CONTROL,
    };
    assert!(keys::detail_key(&mut state, &mut commands, ctrl_a));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        state.status,
        crate::update::state::Status::sticky("nothing to stage")
    );
}

/// m8/09g C: `Home` is `app.top`'s other chord. In the changes pane's file
/// list it goes to the first file as `g` does — not past the pane to the
/// graph behind it, which is where it went (`files_key` had no `Home`).
/// `End` likewise, as `G`.
#[test]
fn home_in_the_changes_pane_acts_as_g() {
    use crate::update::events::AppEvent;
    use mergewright_core::event::Event;

    let sections = [
        (WorkingTreeSection::Unstaged, &["a.rs", "b.rs", "c.rs"][..]),
        (WorkingTreeSection::Untracked, &["d.rs", "e.rs"][..]),
    ];
    // Row 3 is c.rs, the third file: heading, a.rs, b.rs, c.rs.
    let mut state = AppState {
        detail: pane_on(&sections, 3, 0),
        ..AppState::default()
    };
    state.detail.focus = DetailFocus::Files;
    let press =
        |code| crate::update::update(&state, AppEvent::Core(Event::Key(KeyPress::plain(code))));
    let cursor = |state: &AppState| match &state.detail.files {
        Loading::Ready(files) => files.cursor,
        other => panic!("the list is loaded: {other:?}"),
    };
    for (chord, letter) in [(KeyCode::Home, 'g'), (KeyCode::End, 'G')] {
        let (by_chord, chord_commands) = press(chord);
        let (by_letter, letter_commands) = press(KeyCode::Char(letter));
        assert_ne!(cursor(&by_letter), 3, "{letter} moves the files cursor");
        assert_eq!(
            cursor(&by_chord),
            cursor(&by_letter),
            "{chord:?} is {letter}"
        );
        assert_eq!(by_chord.detail, by_letter.detail, "{chord:?} is {letter}");
        assert_eq!(chord_commands, letter_commands, "{chord:?} is {letter}");
    }
}

/// m8/09h R5: in the diff pane `Home`/`End` are `app.top`/`app.bottom`'s
/// other chords (keymap §2.6) and do what `g`/`G` do to the diff — they
/// went past the pane and moved the graph (`diff_key` answered `g`/`G`
/// only). Moved to `t`, `app.top` goes with it: `t` tops the diff and
/// `Home` leaves it where it was.
#[test]
fn home_and_end_in_the_diff_pane_act_as_g_and_capital_g() {
    use crate::update::events::AppEvent;
    use mergewright_core::event::Event;

    let press = |state: &AppState, code| {
        crate::update::update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))))
    };
    let mut mid = crate::update::tests::commit_pane(JobId::new(701), false);
    mid.size = crate::update::Size {
        columns: 120,
        rows: 36,
    };
    for _ in 0..5 {
        mid = press(&mid, KeyCode::Char('j')).0;
    }
    assert_eq!(mid.detail.focus, DetailFocus::Diff);
    assert_eq!(mid.detail.body_cursor, 5, "mid-diff");
    for (chord, letter) in [(KeyCode::Home, 'g'), (KeyCode::End, 'G')] {
        let (by_chord, chord_commands) = press(&mid, chord);
        let (by_letter, letter_commands) = press(&mid, KeyCode::Char(letter));
        assert_ne!(by_letter.detail.body_cursor, 5, "{letter} moves the diff");
        assert_eq!(by_chord.detail, by_letter.detail, "{chord:?} is {letter}");
        assert_eq!(chord_commands, letter_commands, "{chord:?} is {letter}");
        assert_eq!(by_chord.history, mid.history, "{chord:?} left the graph");
    }

    let (top_on_t, _) = mergewright_ui::keymap::Keymap::with_overrides(
        &[("app.top".to_owned(), vec!["t".to_owned()])],
        mergewright_ui::keymap::KeySwitches::default(),
    );
    let remapped = AppState {
        keymap: Arc::new(top_on_t),
        ..mid.clone()
    };
    assert_eq!(
        press(&remapped, KeyCode::Char('t')).0.detail.body_cursor,
        0,
        "t tops the diff"
    );
    let (home, _) = press(&remapped, KeyCode::Home);
    assert_eq!(home.detail, mid.detail, "Home went with app.top");
    assert_eq!(home.history, mid.history, "Home went with app.top");
}
