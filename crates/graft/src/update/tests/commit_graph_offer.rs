use super::*;
use crate::repository::CommitGraphFact;
use graft_git::commit_graph::Written;
use graft_git::version::GitVersion;
use std::time::Duration;

fn opened(fact: CommitGraphFact, git: GitCheck, harness: Harness) -> AppState {
    let state = AppState {
        harness,
        ..AppState::default()
    };
    let mut repository = facts("/code/api", "main");
    repository.commit_graph = fact;
    let (state, _) = update(&state, AppEvent::Produced(job(1), Product::SystemGit(git)));
    let (state, _) = update(
        &state,
        AppEvent::Produced(job(2), Product::Repository(Ok(repository))),
    );
    state
}

fn job(n: u64) -> JobId {
    JobId::new(n)
}

fn supported() -> GitCheck {
    GitCheck::Ran(SystemGit::Supported(GitVersion::MINIMUM))
}

/// `state` after this run's sort, which took `took`, was taken up.
fn sorted_in(mut state: AppState, took: Duration) -> AppState {
    super::commit_graph::sort_landed(&mut state, took);
    state
}

fn offered() -> AppState {
    sorted_in(
        opened(
            CommitGraphFact::Absent { declined: false },
            supported(),
            pinned_harness(),
        ),
        super::commit_graph::SLOW_SORT,
    )
}

#[test]
fn a_missing_file_with_a_usable_git_is_offered() {
    let state = offered();
    assert_eq!(state.commit_graph, CommitGraphOffer::Offered);
    assert_eq!(
        state.commit_graph_line(),
        Some(CommitGraphLine::Offered { keys: true })
    );
}

/// A walk sorting its order (`super::sorting`) in a repository with no
/// commit-graph file, and the sort answering after `took`.
fn sort_answered(took: Duration) -> AppState {
    let (walk, sort) = (JobId::new(1), JobId::new(2));
    let mut state = super::sorting(walk, sort);
    let mut repository = facts("/code/api", "main");
    repository.commit_graph = CommitGraphFact::Absent { declined: false };
    state.system_git = Some(supported());
    state.repository = Some(Box::new(repository));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            sort,
            Product::TopologicalOrder(SortedOrder {
                order: super::order(),
                took,
            }),
        ),
    );
    state
}

/// m7-power-tools/15 (:778): a sort nobody waited for is not worth a file.
#[test]
fn a_fast_sort_gets_no_commit_graph_offer() {
    let absent = CommitGraphFact::Absent { declined: false };
    let under = Duration::from_millis(299);
    assert!(!super::commit_graph::offer_after_sort(&absent, under));
    let state = sort_answered(Duration::from_millis(40));
    assert_eq!(state.commit_graph, CommitGraphOffer::NotNeeded);
    assert_eq!(state.commit_graph_line(), None);
    // The facts alone, before any sort, offer nothing either.
    let before = opened(absent, supported(), pinned_harness());
    assert_eq!(before.commit_graph, CommitGraphOffer::NotNeeded);
}

#[test]
fn a_slow_sort_gets_the_offer() {
    let slow = super::commit_graph::SLOW_SORT;
    assert!(super::commit_graph::offer_after_sort(
        &CommitGraphFact::Absent { declined: false },
        slow
    ));
    for fact in [
        CommitGraphFact::Absent { declined: true },
        CommitGraphFact::Present { commits: 5 },
        CommitGraphFact::Shallow,
    ] {
        assert!(
            !super::commit_graph::offer_after_sort(&fact, slow),
            "{fact:?}"
        );
    }
    let state = sort_answered(Duration::from_millis(900));
    assert_eq!(state.commit_graph, CommitGraphOffer::Offered);
    // A refresh's facts keep it up.
    let mut repository = facts("/code/api", "main");
    repository.commit_graph = CommitGraphFact::Absent { declined: false };
    let (refreshed, _) = update(
        &state,
        AppEvent::Produced(job(9), Product::Repository(Ok(repository))),
    );
    assert_eq!(refreshed.commit_graph, CommitGraphOffer::Offered);
}

/// `Esc` kept the provisional order: the sort's late answer, however
/// slow, offers nothing.
#[test]
fn a_stopped_sort_gets_no_offer() {
    let (walk, sort) = (JobId::new(1), JobId::new(2));
    let mut state = super::sorting(walk, sort);
    let mut repository = facts("/code/api", "main");
    repository.commit_graph = CommitGraphFact::Absent { declined: false };
    state.system_git = Some(supported());
    state.repository = Some(Box::new(repository));
    let (stopped, _) = update(&state, key(KeyCode::Escape));
    let (late, _) = update(
        &stopped,
        AppEvent::Produced(
            sort,
            Product::TopologicalOrder(SortedOrder {
                order: super::order(),
                took: Duration::from_secs(5),
            }),
        ),
    );
    assert_eq!(late.commit_graph, CommitGraphOffer::NotNeeded);
}

fn pane_focused(mut state: AppState) -> AppState {
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Diff;
    state
}

/// m7-power-tools/36 (M3): in the pane `B` is blame; the offer's keys
/// answer only while the graph has the keyboard.
#[test]
fn the_offer_keys_answer_only_with_the_graph_focused() {
    let (_, commands) = update(&offered(), key(KeyCode::Char('B')));
    assert_eq!(commands, vec![Command::WriteCommitGraph]);
    let (_, commands) = update(&offered(), key(KeyCode::Char('X')));
    assert_eq!(commands, vec![Command::DeclineCommitGraph]);
    for focus in [DetailFocus::Files, DetailFocus::Diff] {
        let mut state = pane_focused(offered());
        state.detail.focus = focus;
        assert!(!super::commit_graph::offer_keys_live(&state));
        for letter in ['B', 'X'] {
            let (after, commands) = update(&state, key(KeyCode::Char(letter)));
            assert!(
                !commands.iter().any(|command| matches!(
                    command,
                    Command::WriteCommitGraph | Command::DeclineCommitGraph
                )),
                "{letter} in {focus:?}: {commands:?}"
            );
            assert_eq!(after.commit_graph, CommitGraphOffer::Offered, "{letter}");
        }
    }
}

#[test]
fn the_offer_row_drops_its_keys_off_the_graph() {
    let state = pane_focused(offered());
    assert_eq!(
        state.commit_graph_line(),
        Some(CommitGraphLine::Offered { keys: false })
    );
    let mut back = state;
    back.detail.focus = DetailFocus::Graph;
    assert_eq!(
        back.commit_graph_line(),
        Some(CommitGraphLine::Offered { keys: true })
    );
}

#[test]
fn b_writes_and_the_written_line_lasts_one_key() {
    let (writing, commands) = update(&offered(), key(KeyCode::Char('B')));
    assert_eq!(writing.commit_graph, CommitGraphOffer::Writing(None));
    assert_eq!(commands, vec![Command::WriteCommitGraph]);
    let (again, commands) = update(&writing, key(KeyCode::Char('B')));
    assert!(commands.is_empty(), "a second B starts nothing");

    let (writing, _) = update(&again, AppEvent::Started(job(7), Task::WriteCommitGraph));
    assert_eq!(
        writing.commit_graph,
        CommitGraphOffer::Writing(Some(job(7)))
    );
    let written = Written {
        commits: 200,
        elapsed: Duration::from_millis(5),
    };
    let (done, _) = update(
        &writing,
        AppEvent::Produced(job(7), Product::CommitGraph(Ok(written))),
    );
    assert_eq!(
        done.commit_graph,
        CommitGraphOffer::Written { commits: 200 }
    );
    assert_eq!(done.status, Status::Ready);

    let (next, _) = update(&done, key(KeyCode::Char('j')));
    assert_eq!(next.commit_graph, CommitGraphOffer::NotNeeded);
    assert_eq!(next.commit_graph_line(), None);
}

#[test]
fn a_failed_write_puts_git_s_words_in_the_problem_line() {
    let (writing, _) = update(&offered(), key(KeyCode::Char('B')));
    let (writing, _) = update(&writing, AppEvent::Started(job(7), Task::WriteCommitGraph));
    let why = "commit-graph write failed (exit 128): fatal: Unable to create lock";
    let (failed, _) = update(
        &writing,
        AppEvent::Produced(job(7), Product::CommitGraph(Err(why.to_owned()))),
    );
    assert_eq!(failed.commit_graph, CommitGraphOffer::Failed);
    assert_eq!(failed.commit_graph_line(), None);
    let Status::Failed(problem) = &failed.status else {
        panic!("{:?}", failed.status);
    };
    assert!(problem.contains("exit 128") && problem.contains("fatal: Unable"));
    let (still, commands) = update(&failed, key(KeyCode::Char('B')));
    assert!(commands.is_empty(), "no retry this run");
    assert_eq!(still.commit_graph, CommitGraphOffer::Failed);
}

#[test]
fn x_declines_and_a_failed_decline_offers_again() {
    let (declining, commands) = update(&offered(), key(KeyCode::Char('X')));
    assert_eq!(declining.commit_graph, CommitGraphOffer::Declining(None));
    assert_eq!(commands, vec![Command::DeclineCommitGraph]);
    let (declining, _) = update(
        &declining,
        AppEvent::Started(job(8), Task::DeclineCommitGraph),
    );

    let (declined, _) = update(
        &declining,
        AppEvent::Produced(job(8), Product::CommitGraphDeclined(Ok(()))),
    );
    assert_eq!(declined.commit_graph, CommitGraphOffer::Declined);
    assert_eq!(declined.commit_graph_line(), None);

    let (refused, _) = update(
        &declining,
        AppEvent::Produced(
            job(8),
            Product::CommitGraphDeclined(Err("permission denied".to_owned())),
        ),
    );
    assert_eq!(refused.commit_graph, CommitGraphOffer::Offered);
    assert_eq!(
        refused.status,
        Status::Failed("could not remember the choice: permission denied".to_owned())
    );
}

#[test]
fn nothing_is_offered_with_a_file_a_decline_or_a_harness_that_says_no() {
    let present = opened(
        CommitGraphFact::Present { commits: 5 },
        supported(),
        pinned_harness(),
    );
    assert_eq!(present.commit_graph, CommitGraphOffer::NotNeeded);
    let declined = opened(
        CommitGraphFact::Absent { declined: true },
        supported(),
        pinned_harness(),
    );
    assert_eq!(declined.commit_graph, CommitGraphOffer::NotNeeded);
    let suppressed = sorted_in(
        opened(
            CommitGraphFact::Absent { declined: false },
            supported(),
            Harness {
                offer_commit_graph: false,
                clock: crate::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
                ..Harness::default()
            },
        ),
        super::commit_graph::SLOW_SORT,
    );
    assert_eq!(suppressed.commit_graph, CommitGraphOffer::Suppressed);
    assert_eq!(suppressed.commit_graph_line(), None);
}

#[test]
fn without_a_usable_git_the_line_says_why_and_offers_nothing() {
    for git in [
        GitCheck::Unavailable("no git on PATH".to_owned()),
        GitCheck::Ran(SystemGit::for_version(GitVersion {
            major: 2,
            minor: 30,
            patch: 0,
        })),
    ] {
        let state = sorted_in(
            opened(
                CommitGraphFact::Absent { declined: false },
                git,
                pinned_harness(),
            ),
            super::commit_graph::SLOW_SORT,
        );
        assert_eq!(
            state.commit_graph_line(),
            Some(CommitGraphLine::NoGitToBuild)
        );
        for letter in ['B', 'X'] {
            let (_, commands) = update(&state, key(KeyCode::Char(letter)));
            assert!(commands.is_empty(), "{letter} with no git: {commands:?}");
        }
    }
}

#[test]
fn an_unreadable_file_is_a_problem_not_an_offer() {
    let state = opened(
        CommitGraphFact::Unreadable("commit-graph unreadable: bad signature".to_owned()),
        supported(),
        pinned_harness(),
    );
    assert_eq!(state.commit_graph, CommitGraphOffer::NotNeeded);
    assert_eq!(
        state.status,
        Status::Failed("commit-graph unreadable: bad signature".to_owned())
    );
}

#[test]
fn b_and_x_do_nothing_outside_the_offer() {
    let states = [
        CommitGraphOffer::NotNeeded,
        CommitGraphOffer::Writing(Some(job(3))),
        CommitGraphOffer::Written { commits: 1 },
        CommitGraphOffer::Declining(Some(job(3))),
        CommitGraphOffer::Declined,
        CommitGraphOffer::Failed,
        CommitGraphOffer::Suppressed,
    ];
    for offer in states {
        let state = AppState {
            commit_graph: offer,
            ..offered()
        };
        for letter in ['B', 'X'] {
            let (_, commands) = update(&state, key(KeyCode::Char(letter)));
            assert!(commands.is_empty(), "{letter} in {offer:?}: {commands:?}");
        }
    }
}

#[test]
fn the_line_costs_a_graph_row_while_shown() {
    let mut state = offered();
    state.size = Size {
        columns: 80,
        rows: 24,
    };
    let with = state.graph_rows();
    state.commit_graph = CommitGraphOffer::Declined;
    assert_eq!(state.graph_rows(), with + 1);
}
