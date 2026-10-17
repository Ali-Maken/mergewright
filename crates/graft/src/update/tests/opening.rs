//! `update` tests: opening. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// Every field of [`AppState`] written out, so a field added to the state
/// stops this file compiling until [`the_same_event_always_gives_the_same_answer`]
/// has been given a value for it — a new field with hidden state is exactly
/// what that test is about.
#[allow(
    clippy::too_many_lines,
    reason = "one line per `AppState` field is the point: it grows with the state"
)]
fn every_field_spelled_out() -> AppState {
    AppState {
        quit: None,
        quit_when_done: None,
        harness: pinned_harness(),
        status: Status::Failed("earlier".to_owned()),
        unseen: Some(Status::sticky("arrived under a box")),
        size: Size {
            columns: 80,
            rows: 24,
        },
        repo_path: PathBuf::from("/code/api"),
        repository: Some(Box::new(facts("/code/api", "main"))),
        facts_pending: true,
        system_git: None,
        commit_graph: CommitGraphOffer::NotNeeded,
        awaiting: None,
        promoting: None,
        history: None,
        refs: None,
        refs_read: RefsRead::Pending(JobId::new(3)),
        working_tree: None,
        operating: None,
        resuming: None,
        screen: Screen::Graph,
        lanes_expanded: false,
        focus: false,
        hover: None,
        detail: DetailState::default(),
        local_jobs: LocalJobs::default(),
        renderer_override: None,
        settled_tier: None,
        icons: graft_ui::graph_screen::Icons::Plain,
        max_lanes: None,
        search_prompt: None,
        search: None,
        on_working_tree: false,
        compare_base: None,
        stash_only: crate::update::StashOnlyMarks::default(),
        context_menu: None,
        radar: crate::update::radar::RadarState::default(),
        radar_column_seen: std::cell::Cell::new(false),
        map: crate::update::map::MapState::default(),
        journal: crate::update::journal::JournalState::default(),
        stash: crate::update::stash::StashState::default(),
        picker: None,
        palette: None,
        palette_recent: crate::update::palette::RecentRuns::default(),
        remotes: detail::Loading::Idle,
        remote: crate::update::remote::RemoteState::default(),
        last_focus_refresh: None,
        strip_shown: false,
        confirm: None,
        credential: None,
        name_prompt: None,
        drag: None,
        commit_editor: None,
        commit_draft: None,
        in_progress: None,
        mergetool: None,
        banner_focus: false,
        ended_by_answer: false,
        live_entries: Vec::new(),
        interrupted_edits: vec![(
            graft_core::journal::EntryId::new(2, [0; 10]),
            graft_ops::EditsAt::Back { copy: Some(0) },
        )],
        stopped: None,
        clash: None,
        clash_facts_seen: false,
        clash_drop: None,
        clash_pane_owed: false,
        clash_index: crate::update::state::IndexRead::Unknown,
        land_on_head: false,
        head_landing: crate::update::HeadLanding::default(),
        return_to: Some(Oid::from_bytes(&[7; Oid::SHA1_BYTES]).expect("an oid")),
        tour: Some(Box::new(crate::update::tour::TourState {
            done: [true, true, false, false, false, false, false, false],
            asked: Some(JobId::new(11)),
            wanted: false,
        })),
        lands: None,
        line_landing: None,
        trail: super::super::place::Trail::default(),
        live_landing_owed: false,
        undo_landing_owed: false,
        status_read: None,
        continue_in_git_owed: None,
        head_message: None,
        edit_amend: false,
        commit_template: None,
        external_editor: None,
        suspend_on_ctrl_z: false,
        keymap: std::sync::Arc::default(),
        config_problems: Vec::new(),
        config_notice: false,
        first_launch: None,
        preview: None,
        resolver: None,
        choices: std::collections::BTreeMap::new(),
        picks: Box::default(),
        finish: None,
        finish_draft: None,
        conflict_editing: Some(std::path::PathBuf::from("notes.txt")),
        amend_into: super::super::amend_into::AmendInto::default(),
        rewrite: super::super::rewrite_editor::RewriteState::default(),
        row_tags: {
            let mut tags = super::super::row_tags::RowTags::default();
            tags.set(
                graft_core::oid::Oid::from_sha1([7; 20]),
                graft_ui::graph_screen::RowTag::Changed,
            );
            tags
        },
        fold_counts: [(
            graft_core::oid::Oid::from_sha1([8; 20]),
            super::super::fold_count::FoldCount::Counted(3),
        )]
        .into(),
        defer_follow: true,
        followed: Some(super::super::FollowKey::WorkingTree),
    }
}

#[test]
fn the_same_event_always_gives_the_same_answer() {
    // Purity's observable half: no clock, no counter, no hidden state, so
    // repeating a call repeats the answer.
    let state = every_field_spelled_out();

    let first = update(&state, key(KeyCode::Char('q')));
    let second = update(&state, key(KeyCode::Char('q')));

    assert_eq!(first, second);
    assert_eq!(state.quit, None, "the input state was mutated");
}

/// A repository that opened clears a read failure and nothing else.
///
/// The refresh a failed push asks for lands here, so wiping the status would
/// take `push rejected: …` off the screen before it could be read — and the
/// next `P`'s lease box is offered off the back of that rejection. `Busy` is
/// a running write's line and `Done` a finished one's; neither belongs to the
/// repository (m4-remotes/05 review, M2).
#[test]
fn only_a_read_failure_is_cleared_when_the_repository_opens() {
    let opened = |status: Status| {
        let before = AppState {
            status,
            ..AppState::default()
        };
        update(
            &before,
            AppEvent::Produced(
                JobId::new(1),
                Product::Repository(Ok(facts("/code/api", "main"))),
            ),
        )
        .0
        .status
    };

    let sticky = Status::sticky("push rejected: origin/main has moved · p pull first".to_owned());
    assert_eq!(opened(sticky.clone()), sticky, "a write's failure stays");
    assert_eq!(
        opened(Status::Failed("could not read the refs".to_owned())),
        Status::Ready,
        "a read failure is superseded"
    );
    let busy = Status::Busy("fetch all remotes · esc cancels".to_owned());
    assert_eq!(opened(busy.clone()), busy, "a running write's line stays");
    let done = Status::done("pushed main → origin/main");
    assert_eq!(opened(done.clone()), done, "a finished write's line stays");
    assert_eq!(opened(Status::Ready), Status::Ready);
}

#[test]
fn an_opened_repository_becomes_what_the_screen_shows() {
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(1),
            Product::Repository(Ok(facts("/code/api", "main"))),
        ),
    );

    assert_eq!(
        next.repository.as_deref(),
        Some(&facts("/code/api", "main"))
    );
    assert_eq!(next.status, Status::Ready);
    // Two commands: the launch reads of the undo journal and the remote
    // facts, which cannot run before the repository is open
    // (m3-working-tree-undo-and-stash/06, m4-remotes/05).
    assert!(
        matches!(
            commands.as_slice(),
            [Command::ReadJournal(_), Command::ReadRemotes(_)]
        ),
        "{commands:?}"
    );
}

#[test]
fn a_repository_that_opened_clears_an_earlier_failure() {
    let failed = AppState {
        status: Status::Failed("could not re-read the repository".to_owned()),
        repository: Some(Box::new(facts("/code/api", "main"))),
        ..AppState::default()
    };

    let (next, _) = update(
        &failed,
        AppEvent::Produced(
            JobId::new(2),
            Product::Repository(Ok(facts("/code/api", "release"))),
        ),
    );

    assert_eq!(
        next.repository.as_deref(),
        Some(&facts("/code/api", "release"))
    );
    assert_eq!(next.status, Status::Ready);
}

#[test]
fn a_path_that_is_not_a_repository_ends_the_run() {
    // Nothing was ever open, so there is nothing to show. Graft leaves and
    // says why, on a terminal it has handed back.
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(1),
            Product::Repository(Err("/tmp is not inside a git repository".to_owned())),
        ),
    );

    assert_eq!(
        next.quit,
        Some(Quit::Fatal(
            "/tmp is not inside a git repository".to_owned()
        ))
    );
    assert!(commands.is_empty());
}

#[test]
fn a_failed_refresh_of_an_open_repository_only_reaches_the_status_line() {
    let open = AppState {
        repository: Some(Box::new(facts("/code/api", "main"))),
        ..AppState::default()
    };

    let (next, _) = update(
        &open,
        AppEvent::Produced(
            JobId::new(3),
            Product::Repository(Err("the repository went away".to_owned())),
        ),
    );

    assert_eq!(next.quit, None, "a refresh must not end the run");
    assert_eq!(
        next.status,
        Status::Failed("the repository went away".to_owned())
    );
    // The repository already read stays on screen: stale beats blank.
    assert_eq!(
        next.repository.as_deref(),
        Some(&facts("/code/api", "main"))
    );
}

#[test]
fn what_the_system_git_is_is_remembered() {
    let (next, commands) = update(
        &AppState::default(),
        AppEvent::Produced(
            JobId::new(4),
            Product::SystemGit(GitCheck::Unavailable("no git on PATH".to_owned())),
        ),
    );

    assert_eq!(
        next.system_git,
        Some(GitCheck::Unavailable("no git on PATH".to_owned()))
    );
    assert!(commands.is_empty());
}
