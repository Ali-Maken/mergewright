//! `update` tests: detail pane. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

// --- The detail pane (m2-details-and-diffs/06, ADR-0023) ---

pub(super) fn list_commands(commands: &[Command]) -> Vec<(JobId, graft_git::diff::DiffSpec)> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::ListFiles(id, spec, _) => Some((*id, *spec)),
            _ => None,
        })
        .collect()
}

/// Two files, flat, as a files job would hand them back.
pub(super) fn two_files() -> (DetailArc<FileListView>, DetailArc<FileList>) {
    let entry = |path: &str| FileEntry {
        path: path.into(),
        old_path: None,
        status: FileStatus::Modified,
        added: Some(1),
        removed: Some(1),
    };
    let row = |name: &str, index| FileRow {
        depth: 0,
        name: sanitize_line(name),
        kind: FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        },
        entry: Some(index),
    };
    let view = FileListView {
        rows: vec![row("a.rs", 0), row("b.rs", 1)],
        files: 2,
        added: 2,
        removed: 2,
        ..FileListView::default()
    };
    let list = FileList {
        files: vec![entry("a.rs"), entry("b.rs")],
        added: 2,
        removed: 2,
    };
    (DetailArc::new(view), DetailArc::new(list))
}

pub(super) fn diff_of(path: &str) -> (DetailArc<DiffView>, DetailArc<graft_git::diff::FileDiff>) {
    let view = DetailArc::new(DiffView {
        path: sanitize_line(path),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        untracked: false,
    });
    let raw = DetailArc::new(graft_git::diff::FileDiff {
        entry: graft_git::diff::FileEntry {
            path: path.as_bytes().to_vec(),
            old_path: None,
            status: graft_git::diff::FileStatus::Modified,
            added: Some(0),
            removed: Some(0),
        },
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    });
    (view, raw)
}

#[test]
fn enter_opens_the_pane_on_the_selected_commit_and_asks_for_its_files() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let selected = state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| row.commit.id)
        .expect("row 0 has arrived");
    let (opened, commands) = update(&state, key(KeyCode::Enter));
    assert!(opened.detail.shown);
    assert_eq!(opened.detail.focus, DetailFocus::Files);
    let lists = list_commands(&commands);
    assert_eq!(lists.len(), 1, "{commands:?}");
    assert_eq!(
        lists[0].1,
        graft_git::diff::DiffSpec::Commit {
            commit: selected,
            against: None
        }
    );
    assert_ne!(
        lists[0].0.raw() & LOCAL_JOB_BIT,
        0,
        "update names the job itself"
    );
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::ContainedIn(_, oid) if *oid == selected))
    );

    // The evil-merge probe follows the selection, not the pane (M2 review
    // M3): row 0 here is an ordinary commit, so opening the pane on it asks
    // for no probe, and closing the pane cancels nothing.
    assert!(
        !commands.iter().any(|c| matches!(c, Command::EvilProbe(..))),
        "an ordinary commit is not probed: {commands:?}"
    );

    let (again, commands) = update(&opened, key(KeyCode::Escape));
    assert!(!again.detail.shown);
    assert!(commands.is_empty(), "{commands:?}");
}

/// M2 review M3: the probe follows the selection, so the merge the cursor
/// rests on is probed with the pane closed — that is what puts the `⚠` on
/// the graph row rather than only inside the pane.
#[test]
fn the_selected_merge_is_probed_with_the_pane_closed() {
    let (state, merge) = merge_on_screen(JobId::new(41), 1);
    assert!(!state.detail.shown, "the pane starts closed");
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    assert!(!next.detail.shown, "folding does not open the pane");
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::EvilProbe(_, oid) if *oid == merge)),
        "the selected merge is probed: {commands:?}"
    );
    assert_eq!(next.detail.evil_at, Some(merge));
}

/// Refs naming `stash` as `stash@{0}` and nothing else.
fn refs_with_stash(stash: Oid) -> Arc<crate::refs::RefBadges> {
    let badges = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        &[(stash, 0)],
        |_, _| unreachable!("no branches to count"),
    )
    .expect("no refs to fail reading");
    Arc::new(badges)
}

/// m6/66: a stash commit is a merge whose tree differs from both parents by
/// construction, so probing it always said `EVIL MERGE`. Through
/// `follow_selection`, not `probe_evil`: the filter is on the selection.
/// `view::radar` draws `⚠` only when `evil_at` names the row, so `evil_at`
/// staying `None` is what keeps the mark off.
#[test]
fn a_stash_commit_is_never_probed_for_an_evil_merge() {
    let (mut state, stash) = merge_on_screen(JobId::new(41), 1);
    state.refs = Some(refs_with_stash(stash));
    // Even a verdict already cached for it must not surface.
    state.detail.evil_cache.insert(
        stash,
        EvilVerdict::Evil {
            files: vec![b"x".to_vec()],
        },
    );
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    assert!(
        !commands.iter().any(|c| matches!(c, Command::EvilProbe(..))),
        "a stash commit is not probed: {commands:?}"
    );
    assert_eq!(next.detail.evil_at, None);
    assert!(!matches!(
        next.detail.evil,
        EvilLoading::Ready(EvilVerdict::Evil { .. })
    ));
}

/// m6/66: before the refs land the app cannot tell a stash from a merge,
/// and unknown is not "not a stash" — so no probe is asked yet.
#[test]
fn no_evil_probe_before_the_refs_are_read() {
    let (mut state, _) = merge_on_screen(JobId::new(41), 1);
    state.refs = None;
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    assert!(
        !commands.iter().any(|c| matches!(c, Command::EvilProbe(..))),
        "no probe before the refs: {commands:?}"
    );
    assert_eq!(next.detail.evil_at, None);
}

/// m6/66 control: a real merge, with refs read and a stash elsewhere, is
/// still probed and carries its verdict.
#[test]
fn an_evil_merge_on_a_branch_is_still_marked() {
    let (mut state, merge) = merge_on_screen(JobId::new(41), 1);
    let elsewhere = Oid::from_bytes(&[0x77; Oid::SHA1_BYTES]).expect("20 bytes");
    state.refs = Some(refs_with_stash(elsewhere));
    let (next, commands) = update(&state, key(KeyCode::Char(' ')));
    let Some(Command::EvilProbe(id, _)) = commands
        .iter()
        .find(|c| matches!(c, Command::EvilProbe(_, oid) if *oid == merge))
    else {
        panic!("the merge is probed: {commands:?}");
    };
    let evil = EvilVerdict::Evil {
        files: vec![b"x".to_vec()],
    };
    let (marked, _) = update(
        &next,
        AppEvent::Produced(*id, Product::EvilProbe(merge, evil.clone())),
    );
    assert_eq!(marked.detail.evil_at, Some(merge));
    assert_eq!(marked.detail.evil, EvilLoading::Ready(evil));
}

#[test]
fn a_second_enter_on_the_same_row_asks_nothing() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (opened, _) = update(&state, key(KeyCode::Enter));
    let (mut graph, _) = update(&opened, key(KeyCode::Tab));
    graph.detail.focus = DetailFocus::Graph;
    let (again, commands) = update(&graph, key(KeyCode::Enter));
    assert!(again.detail.shown);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn moving_the_graph_selection_while_shown_cancels_and_reloads() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (mut opened, first) = update(&state, key(KeyCode::Enter));
    let old = list_commands(&first)[0].0;
    opened.detail.focus = DetailFocus::Graph;
    let (moved, commands) = update(&opened, key(KeyCode::Char('j')));
    assert!(commands.contains(&Command::CancelJob(old)), "{commands:?}");
    assert_eq!(list_commands(&commands).len(), 1, "{commands:?}");
    assert_ne!(moved.detail.files, opened.detail.files);

    let (mut hidden, _) = update(&state, key(KeyCode::Char('j')));
    hidden.detail = DetailState::default();
    let (_, commands) = update(&hidden, key(KeyCode::Char('k')));
    assert!(
        list_commands(&commands).is_empty(),
        "a hidden pane costs nothing"
    );
}

#[test]
fn a_stale_file_list_changes_nothing() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (opened, _) = update(&state, key(KeyCode::Enter));
    let old_answer = AppEvent::Produced(
        JobId::new(LOCAL_JOB_BIT | 0x3e7),
        Product::Files(Ok(two_files())),
    );
    let (next, commands) = update(&opened, old_answer);
    assert_eq!(next.detail, opened.detail);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn files_arriving_ask_for_the_first_diff_and_revisits_hit_the_cache() {
    let (state, _) = merge_on_screen(JobId::new(23), 0);
    let (opened, first) = update(&state, key(KeyCode::Enter));
    let id = list_commands(&first)[0].0;
    let (listed, commands) = update(
        &opened,
        AppEvent::Produced(id, Product::Files(Ok(two_files()))),
    );
    let Loading::Ready(files) = &listed.detail.files else {
        panic!("files ready")
    };
    assert_eq!(files.cursor, 0);
    let reads: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            Command::ReadFileDiff(id, key, _) => Some((*id, *key)),
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 1, "{commands:?}");
    let (first_id, first_key) = reads[0];
    assert_eq!(first_key.entry, 0);
    assert_eq!(first_key.options, DiffOptions::default());
    let (shown, _) = update(
        &listed,
        AppEvent::Produced(first_id, Product::FileDiff(first_key, Ok(diff_of("a.rs")))),
    );
    assert!(matches!(shown.detail.diff, Loading::Ready(_)));

    let (down, commands) = update(&shown, key(KeyCode::Char('j')));
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::ReadFileDiff(_, key, _) if key.entry == 1)),
        "{commands:?}"
    );
    let (back, commands) = update(&down, key(KeyCode::Char('k')));
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ReadFileDiff(..))),
        "{commands:?}"
    );
    assert!(
        matches!(back.detail.diff, Loading::Ready(_)),
        "cache hit shows at once"
    );

    let (diff_focus, _) = update(&back, key(KeyCode::Enter));
    assert_eq!(diff_focus.detail.focus, DetailFocus::Diff);
    assert!(
        matches!(back.detail.spec, Some(DetailSpec::Commit(_))),
        "spec kept"
    );
}
