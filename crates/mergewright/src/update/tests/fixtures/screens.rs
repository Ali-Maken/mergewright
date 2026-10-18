//! `update` test fixtures: an operation live, the commit pane, the resolver.
//! Moved out of `update/tests.rs` (m8/21b-7, move-only).

use super::*;

/// An operation live over `state` whose `c` in the graph or the changes
/// pane is `c` (m6-conflicts/52), the banner unfocused: a merge for
/// `Finish`, a rebase Mergewright began for `Continue`. For `Refuses`, a revert
/// the shell began where the banner is to be one `⇥` away, else a bisect —
/// which has no banner at all.
pub(crate) fn operation_live_as(
    state: &mut AppState,
    c: mergewright_ui::graph_screen::LiveC,
    banner: bool,
) {
    use mergewright_git::state::InProgress;
    use mergewright_ui::graph_screen::LiveC;
    let kind = match c {
        LiveC::Finish => InProgress::Merge,
        LiveC::Continue | LiveC::ContinueAmends => InProgress::Rebase,
        LiveC::Refuses if banner => InProgress::Revert,
        LiveC::Refuses => InProgress::Bisect,
    };
    state.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    if matches!(c, LiveC::Continue | LiveC::ContinueAmends) {
        state.live_entries = vec![mergewright_core::journal::EntryId::new(1, [0; 10])];
    }
    // A rewrite's `edit` stop (07b): the stop carries the edit-stop read.
    if c == LiveC::ContinueAmends {
        state.stopped = Some(crate::update::Stop {
            origin: crate::update::LiveOrigin::ThisRun,
            again: false,
            kind,
            paths_at_stop: 0,
            rewrite: Some(Ok(crate::update::banner::RewriteStop {
                progress: mergewright_ops::rewrite::RewriteProgress {
                    step: (1, 2),
                    current: None,
                    pending: Vec::new(),
                    branch: None,
                    begun_in_mergewright: true,
                    followers: Vec::new(),
                    fork: None,
                },
                edit: Some(mergewright_ops::rewrite::EditStop {
                    stopped: mergewright_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes"),
                    head_on_it: true,
                }),
            })),
        });
    }
    state.banner_focus = false;
    // Checked, not trusted: `live_operation` reads "Mergewright began it" off
    // `live_entries`, and a bisect has no banner.
    let live = crate::update::commit_editor::live_operation(state).expect("an operation is live");
    assert_eq!(
        crate::update::commit_editor::pane_c_acts(live),
        c != LiveC::Refuses,
        "{live:?} is not a `{c:?}` state"
    );
    assert_eq!(
        crate::update::banner::banner_view(state).is_some(),
        kind != InProgress::Bisect
    );
}

/// The detail pane open on the cursor's commit, with a file list and a long
/// diff in it, focused where `opens` says: `⏎` opens the diff from the
/// files list, and has nothing left to open once the diff has it
/// (`detail::keys::enter_opens`).
pub(crate) fn commit_pane(walk: JobId, opens: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    let spec = state.selected_spec().expect("the cursor is on a commit");
    let focus = if opens {
        DetailFocus::Files
    } else {
        DetailFocus::Diff
    };
    show_pane(&mut state, spec, focus);
    let (view, raw) = pane_files();
    state.detail.files = Loading::Ready(view);
    state.detail.list = Some(raw);
    let (diff, diff_raw) = long_diff("a.rs");
    state.detail.diff = Loading::Ready(diff);
    state.detail.diff_raw = Some(diff_raw);
    state
}

/// The resolver open on a real two-block conflict, the state every one of
/// `resolver_list`'s live keys has to act in: a cursor on the first block so
/// `o`/`t`/`b` write, a second open block so a take advances, a mergetool
/// configured so `M` hands over instead of refusing, and a working tree that
/// lists the file as unmerged so the title and `⏎` have a file to name —
/// and a second unmerged file after it, so `j` has a row to step to
/// (m6-conflicts/17).
pub(crate) fn resolver_on_screen(walk: JobId) -> AppState {
    const CONFLICTED: &[u8] = b"one\n\
        <<<<<<< HEAD\nours a\n=======\ntheirs a\n>>>>>>> topic\n\
        two\n\
        <<<<<<< HEAD\nours b\n=======\ntheirs b\n>>>>>>> topic\n\
        three\n";
    let file = mergewright_git::conflict::parse(CONFLICTED).expect("two open blocks");
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    state.working_tree = Some(Arc::new(mergewright_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("conflicted.rs", true),
            status_entry("later.rs", true),
        ],
    }));
    state.mergetool = Some("vimdiff".to_string());
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("conflicted.rs"),
        opened_from: resolver::OpenedFrom::Pane,
        file: Some(Arc::new(file)),
        whole_file: None,
        how: None,
        cursor_block: Some(0),
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        combined: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
        done: None,
        in_flight: None,
        queued: std::collections::VecDeque::new(),
        land_last: false,
        seek_open: false,
        dropped: 0,
        settling: None,
        whole_side: None,
    });
    state
}

/// The resolver open on a whole-file card (`deleted by them`), the state
/// every key a card's bar draws has to act in (m6-05 review, Major 1): a
/// second conflicted file after it so `N` has somewhere to cross to, and a
/// mergetool so `M` hands over.
pub(crate) fn resolver_card_on_screen(walk: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    state.working_tree = Some(Arc::new(mergewright_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("gone.txt", true),
            status_entry("later.rs", true),
        ],
    }));
    state.mergetool = Some("vimdiff".to_string());
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("gone.txt"),
        opened_from: resolver::OpenedFrom::Pane,
        file: None,
        whole_file: Some(resolver::WholeFileShape {
            how: mergewright_git::status::UnmergedHow::DeletedByThem,
            binary: false,
            on_disk: Some(crate::jobs::conflict::OnDisk {
                size: 11,
                head: Arc::from(vec![b"kept by us".to_vec()]),
                cut: false,
            }),
            start_over: crate::jobs::conflict::StartOver::Cannot {
                why: mergewright_ops::operation::Refusal::NotMarkerShape {
                    path: std::path::PathBuf::from("gone.txt"),
                    how: mergewright_git::status::UnmergedHow::DeletedByThem,
                }
                .explain(&crate::update::op_keys::OpKeys(
                    mergewright_ui::keymap::Keymap::shipped(),
                )),
            },
        }),
        how: Some(mergewright_git::status::UnmergedHow::DeletedByThem),
        cursor_block: None,
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        combined: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
        done: None,
        in_flight: None,
        queued: std::collections::VecDeque::new(),
        land_last: false,
        seek_open: false,
        dropped: 0,
        settling: None,
        whole_side: None,
    });
    state
}
