use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::remote::{Remote, TrackedUpstream};

use super::*;
use crate::update::{AppEvent, update};

fn remote(name: &str) -> Remote {
    Remote {
        name: name.to_owned(),
        fetch_url: format!("file:///srv/{name}.git"),
        push_url: None,
    }
}

fn oid(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("40 hex digits")
}

/// `origin/main`'s tip as last fetched.
const THEIRS: &str = "9f8e7d6c5b4a39281706f5e4d3c2b1a098765432";

fn upstream(tip: Option<&str>) -> TrackedUpstream {
    TrackedUpstream {
        remote: "origin".to_owned(),
        remote_branch: "refs/heads/main".to_owned(),
        tracking_ref: "refs/remotes/origin/main".to_owned(),
        tracking_tip: tip.map(oid),
    }
}

/// A state whose remote facts have landed.
fn with_facts(
    remotes: Vec<Remote>,
    head_branch: Option<&str>,
    head_upstream: Option<TrackedUpstream>,
) -> AppState {
    AppState {
        remotes: Loading::Ready(Arc::new(RemoteFacts {
            remotes,
            head_branch: head_branch.map(str::to_owned),
            head_upstream,
        })),
        ..AppState::default()
    }
}

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

fn plain(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

/// `p` with nothing to pull from says why, in the design note's words, and
/// sends nothing to the job runner.
#[test]
fn pull_refusals_are_words_not_commands() {
    let cases = [
        (
            with_facts(vec![remote("origin")], None, None),
            "HEAD is detached — check out a branch first",
        ),
        (
            with_facts(vec![remote("origin")], Some("refs/heads/main"), None),
            "main has no upstream — P pushes and sets one",
        ),
    ];
    for (state, words) in cases {
        let (next, commands) = press(&state, plain('p'));
        assert!(commands.is_empty(), "{words}: {commands:?}");
        assert_eq!(next.operating, None, "{words}");
        assert_eq!(next.status, Status::Failed(words.to_owned()));
    }
}

/// `p` with an upstream launches the pull, one write at a time.
#[test]
fn pull_with_an_upstream_launches_it() {
    let state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let (next, commands) = press(&state, plain('p'));
    let Some(id) = next.operating else {
        panic!("no write launched: {commands:?}");
    };
    assert_eq!(
        commands,
        [Command::Operate(
            id,
            Box::new(Operation::Pull(PullParams {
                branch: "refs/heads/main".to_owned(),
                upstream: upstream(Some(THEIRS)),
            }))
        )]
    );
    assert_eq!(
        next.status,
        Status::Busy("pull origin/main · esc cancels".to_owned())
    );
    let (again, commands) = press(&next, plain('f'));
    assert!(commands.is_empty(), "a second write waits: {commands:?}");
    assert_eq!(again.operating, Some(id));
}

/// One box's title, consequence lines and command, exactly.
fn assert_box(view: &ConfirmView, title: &str, message: &[&str], command: &str) {
    assert_eq!(view.title.as_str(), title);
    let said: Vec<&str> = view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(said, message);
    assert_eq!(view.command.as_str(), command);
}

/// The push box's title, consequence and command, exactly, for the three
/// shapes: an upstream, none (`-u`), and the lease after a rejection.
#[test]
fn push_box_text_table() {
    let ours = oid("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678");

    // 1. An upstream: its remote and branch, two commits ahead.
    let state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let params = push_params(&state).expect("an upstream to push to");
    assert!(!params.set_upstream && params.lease.is_none());
    let view = push_box(&params, Some(2), Some(ours));
    assert_box(
        &view,
        "Push main → origin/main",
        &["2 commits · can't be undone from here — origin keeps what you send"],
        "git push --progress origin refs/heads/main:refs/heads/main",
    );
    assert_eq!(
        (view.choices[0].as_str(), view.choices[1].as_str()),
        ("Cancel", "Push")
    );

    // 2. No upstream: `origin`, same name, sets one. Count unknown: omitted.
    let state = with_facts(
        vec![remote("fork"), remote("origin")],
        Some("refs/heads/main"),
        None,
    );
    let params = push_params(&state).expect("origin to push to");
    assert_eq!(params.remote, "origin");
    assert_eq!(params.remote_branch, "refs/heads/main");
    assert!(params.set_upstream);
    assert_box(
        &push_box(&params, None, Some(ours)),
        "Push main → origin/main",
        &["sets upstream · can't be undone from here — origin keeps what you send"],
        "git push --progress -u origin refs/heads/main:refs/heads/main",
    );

    // 3. The same push after a non-fast-forward rejection: the lease box.
    let mut state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    state.remote.last_rejected_push = Some(push_params(&state).expect("an upstream"));
    let params = push_params(&state).expect("the lease variant");
    assert_eq!(
        params.lease,
        Some(Lease {
            expected: oid(THEIRS)
        })
    );
    let view = push_box(&params, Some(1), Some(ours));
    assert_box(
        &view,
        "Force-push main → origin/main?",
        &[
            "origin/main 9f8e7d6 → a1b2c3d · refused if origin moved since your fetch (lease)",
            "rewrites history for everyone who has 9f8e7d6",
        ],
        &format!(
            "git push --progress --force-with-lease=refs/heads/main:{THEIRS} origin refs/heads/main:refs/heads/main"
        ),
    );
    assert_eq!(view.choices[1].as_str(), "Force push");
}

/// Every refusal `P` can give, as words on the status line.
#[test]
fn push_refusals_are_words() {
    let cases = [
        (
            with_facts(Vec::new(), Some("refs/heads/main"), None),
            "no remote — add one in the shell: git remote add origin <url>",
        ),
        (
            with_facts(
                vec![remote("a"), remote("b")],
                Some("refs/heads/main"),
                None,
            ),
            "several remotes and no upstream — set one: git push -u <remote> main",
        ),
        (
            with_facts(vec![remote("origin")], None, None),
            "HEAD is detached — check out a branch first",
        ),
    ];
    for (state, words) in cases {
        assert_eq!(push_params(&state), Err(words.to_owned()));
        let (next, commands) = press(&state, plain('P'));
        assert!(commands.is_empty() && next.confirm.is_none(), "{words}");
        assert_eq!(next.status, Status::Failed(words.to_owned()));
    }
    // The only remote, when it is not called origin.
    let only = with_facts(vec![remote("fork")], Some("refs/heads/main"), None);
    assert_eq!(
        push_params(&only).map(|params| params.remote),
        Ok("fork".to_owned())
    );
    // A rejection with no fetched tip has nothing to lease against.
    let mut unfetched = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(None)),
    );
    unfetched.remote.last_rejected_push = Some(push_params(&unfetched).expect("an upstream"));
    assert_eq!(
        push_params(&unfetched),
        Err("f to fetch first — nothing to lease against".to_owned())
    );
}

/// With the push box open, the global keys — undo, redo, `Ctrl+Z` and the
/// remote keys themselves — only close the box: no write, no second box
/// (M3 review 06 C1's shape: a modal answers first).
#[test]
fn box_is_modal() {
    let state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let (open, commands) = press(&state, plain('P'));
    assert!(commands.is_empty());
    assert!(open.confirm.is_some(), "P opens the box");
    let ctrl_z = KeyPress {
        code: KeyCode::Char('z'),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    };
    for key in [
        plain('u'),
        plain('U'),
        ctrl_z,
        plain('f'),
        plain('p'),
        plain('P'),
    ] {
        let (next, commands) = press(&open, key);
        assert!(commands.is_empty(), "{key:?}: {commands:?}");
        assert_eq!(next.operating, None, "{key:?}");
        let mut expected = open.clone();
        expected.confirm = None;
        assert_eq!(next, expected, "{key:?} changed more than the box");
    }
}

/// `Esc` while a write runs asks the job to stop and says so; a second
/// `Esc` asks nothing more; the cancelled answer clears the write as
/// `<desc> cancelled` and re-reads everything.
#[test]
fn esc_cancels_the_running_write_once() {
    let state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let (running, _) = press(&state, plain('f'));
    let id = running.operating.expect("f launched the fetch");
    let (asked, commands) = press(&running, KeyPress::plain(KeyCode::Escape));
    assert_eq!(commands, [Command::CancelJob(id)]);
    assert_eq!(
        asked.status,
        Status::Busy("fetch all remotes · cancelling".to_owned())
    );
    let (again, commands) = press(&asked, KeyPress::plain(KeyCode::Escape));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(again.status, asked.status);

    // Progress that was already on its way does not overwrite the notice.
    let (still, _) = update(
        &again,
        AppEvent::Progress(
            id,
            crate::update::Progress::RemoteProgress {
                brief: "receiving objects 45%".to_owned(),
            },
        ),
    );
    assert_eq!(still.status, asked.status);

    let (done, commands) = update(&still, AppEvent::Core(Event::JobCancelled(id)));
    assert_eq!(done.operating, None);
    assert_eq!(
        done.status,
        Status::Done("fetch all remotes cancelled".to_owned())
    );
    assert!(commands.contains(&Command::ReadRefs), "{commands:?}");
}

/// Progress names the write and the brief; under 100 columns the brief
/// drops its counts.
#[test]
fn progress_reaches_the_line_and_narrows() {
    let mut state = with_facts(vec![remote("origin")], Some("refs/heads/main"), None);
    state.size.columns = 120;
    let (running, _) = press(&state, plain('f'));
    let id = running.operating.expect("f launched the fetch");
    let mut wide = running.clone();
    remote_progress(&mut wide, id, "receiving objects 45% (450/1000)");
    assert_eq!(
        wide.status,
        Status::Busy(
            "fetch all remotes · receiving objects 45% (450/1000) · esc cancels".to_owned()
        )
    );
    let mut narrow_state = running.clone();
    narrow_state.size.columns = 80;
    remote_progress(&mut narrow_state, id, "receiving objects 45%");
    assert_eq!(
        narrow_state.status,
        Status::Busy("fetch all remotes · receiving 45% · esc cancels".to_owned())
    );
    let mut other = running;
    remote_progress(&mut other, JobId::new(999), "receiving objects 45%");
    assert_eq!(
        other.status,
        Status::Busy("fetch all remotes · esc cancels".to_owned()),
        "another job's progress is dropped"
    );
}

/// A non-fast-forward push is remembered for the lease box; a cancel is
/// `Done`, not sticky; anything else is the `ops` line, sticky.
#[test]
fn failures_are_worded_and_remembered() {
    let state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let params = push_params(&state).expect("an upstream");
    let op = Operation::Push(params.clone());
    let mut after = state.clone();
    let rejected = "push rejected: origin/main has moved · p pull first · P force with lease";
    assert_eq!(
        remote_failed(&mut after, &op, &RemoteFailure::NonFastForward, rejected),
        Status::Sticky {
            reason: "push rejected: origin/main has moved".to_owned(),
            keys: Some("p pull first · P force with lease".to_owned()),
        }
    );
    assert_eq!(after.remote.last_rejected_push, Some(params));
    assert_eq!(
        remote_failed(
            &mut after,
            &op,
            &RemoteFailure::Cancelled,
            "push main → origin/main cancelled"
        ),
        Status::Done("push main → origin/main cancelled".to_owned())
    );
    // A push that lands forgets the rejection.
    assert_eq!(
        remote_done(&mut after, &op),
        Some("pushed main → origin/main".to_owned())
    );
    assert_eq!(after.remote.last_rejected_push, None);
}

/// `Esc` **cancels** `u` and `U` while they are in flight
/// (m5-history-and-ghost-preview/06, scope item 5).
///
/// This used to be the opposite assertion. `graft_ops::restore::execute`
/// checked its stop token once, before the first step, so a cancel mid-plan
/// left the steps that had already run applied and nothing put them back;
/// the app refused the cancel and said so (`UNDO_CANNOT_STOP`). Group 06
/// added the between-step check and the roll-back, so the refusal has no
/// reason left and the key does what `Esc` does everywhere else: one
/// `CancelJob` for the running job, `cancelling` latched so a second `Esc`
/// does not ask twice.
#[test]
fn esc_cancels_an_undo_in_flight() {
    for (key, words) in [(plain('u'), "undo"), (plain('U'), "redo")] {
        let (running, commands) = press(&AppState::default(), key);
        let id = running.operating.expect("the undo launched");
        assert_eq!(
            commands.len(),
            1,
            "{words}: the launch is the only command: {commands:?}"
        );
        assert_eq!(
            running.status,
            Status::Busy(format!("{words}{ESC_HINT}")),
            "{words}: the busy line offers the cancel"
        );

        let (pressed, commands) = press(&running, KeyPress::plain(KeyCode::Escape));
        assert!(
            matches!(commands.as_slice(), [Command::CancelJob(asked)] if *asked == id),
            "{words}: Esc must stop the job it is on: {commands:?}"
        );
        assert_eq!(
            pressed.operating,
            Some(id),
            "{words}: still running until the job answers"
        );
        assert_eq!(pressed.remote.cancelling, Some(id), "{words}");
        assert_eq!(
            pressed.status,
            Status::Busy(format!("{words} \u{b7} cancelling")),
            "{words}"
        );

        // A second `Esc` asks for nothing more: the first is still going.
        let (again, commands) = press(&pressed, KeyPress::plain(KeyCode::Escape));
        assert!(commands.is_empty(), "{words}: asked twice: {commands:?}");
        assert_eq!(again.remote.cancelling, Some(id), "{words}");
    }
}

/// A history operation is the opposite: `Esc` does **not** stop it, and the
/// line says why (m5-history-and-ghost-preview/08, piece 10).
///
/// An undo can now be cancelled because `restore::execute` rolls its own
/// steps back. A merge or a rebase cannot: it is not one git write, and a
/// SIGTERM mid-rebase leaves `rebase-merge/` on disk with a `Pending`
/// journal entry that `graft_ops` does not model as `Aborted`. So the app
/// refuses the cancel rather than offering one it cannot honour, and points
/// at the thing that *can* stop it — the banner's `a`.
#[test]
fn history_ops_launch_unstoppable() {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let op = graft_ops::operation::Operation::Merge(graft_ops::operation::MergeParams {
        source: graft_ops::operation::Source::Branch("feature".to_owned()),
        into: graft_ops::operation::Tip {
            name: "refs/heads/main".to_owned(),
            oid: oid(THEIRS),
        },
        source_tip: oid(THEIRS),
        ff: graft_ops::operation::FastForward::Default,
    });
    let description = op.describe();
    crate::update::operate::launch_unstoppable(
        &mut state,
        &mut commands,
        op,
        crate::update::preview::HISTORY_CANNOT_STOP,
        Some("m"),
        &mut || JobId::new(7),
    );
    assert_eq!(state.operating, Some(JobId::new(7)));

    let (pressed, commands) = press(&state, KeyPress::plain(KeyCode::Escape));
    assert!(
        commands.is_empty(),
        "a merge must not be killed mid-way: {commands:?}"
    );
    assert_eq!(pressed.remote.cancelling, None);
    assert_eq!(
        pressed.status,
        Status::Busy(format!(
            "{description} {}",
            crate::update::preview::HISTORY_CANNOT_STOP
        )),
        "the refusal names what does stop it"
    );
}

/// A `p` that cannot fast-forward names the two keys that *can* do it, and
/// remembers the upstream they aim at (m5-history-and-ghost-preview/08,
/// piece 9).
///
/// `graft_ops`' own line for this sends the user to the shell
/// (`git pull --rebase`). M5 put both verbs inside Graft, so the app words
/// it as a state with keys — and `r` then previews a rebase onto the
/// *tracking ref*, whatever row the cursor is on, because the sentence on
/// screen is what the user is answering.
/// `main` at `OURS`, `origin/main` at `THEIRS`, the remote facts landed.
fn diverged() -> AppState {
    let mut state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let ours = oid(OURS);
    let main = graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a ref name");
    state.refs = Some(Arc::new(
        crate::refs::badges(
            &[(main.clone(), ours)],
            &graft_git::repo::HeadState::Branch(main),
            &std::collections::HashMap::new(),
            &[],
            |_, _| {
                Ok(graft_git::history::AheadBehind {
                    ahead: 1,
                    behind: 1,
                })
            },
        )
        .expect("no ahead/behind walk can fail here"),
    ));
    state
}

/// Our own tip, for the rebase the diverged line offers.
const OURS: &str = "1111111111111111111111111111111111111111";

#[test]
fn diverged_pull_sets_m_r_source() {
    let mut state = diverged();
    let op = Operation::Pull(PullParams {
        branch: "refs/heads/main".to_owned(),
        upstream: upstream(Some(THEIRS)),
    });
    let status = remote_failed(
        &mut state,
        &op,
        &graft_git::remote::RemoteFailure::NotFastForward,
        "pull stopped: main and origin/main diverged \u{2014} merge or rebase",
    );
    assert_eq!(
        status,
        Status::Sticky {
            reason: "main and origin/main have diverged".to_owned(),
            keys: Some("m merge it  r rebase onto it".to_owned()),
        }
    );
    assert_eq!(
        state.remote.diverged_upstream,
        Some(("refs/remotes/origin/main".to_owned(), oid(THEIRS)))
    );
    state.status = status;

    // `r` previews a rebase onto that tracking ref, pinned to the tip the
    // refusal recorded — not onto whatever the cursor is over.
    let (next, _) = press(&state, plain('r'));
    let preview = next.preview.as_ref().expect("r opened a preview");
    let crate::update::preview::PreviewSubject::Operation { op, .. } = &preview.subject else {
        panic!("not an operation: {:?}", preview.subject);
    };
    assert_eq!(
        **op,
        Operation::Rebase(graft_ops::operation::RebaseParams {
            branch: graft_ops::operation::Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(OURS),
            },
            onto: graft_ops::operation::Source::Branch("origin/main".to_owned()),
            onto_tip: oid(THEIRS),
        })
    );
}

/// `Esc` dismisses the diverged line, and the keys it offered go with it:
/// the remembered tip means nothing without the sentence that explained it.
#[test]
fn dismissing_the_diverged_line_takes_its_keys_with_it() {
    let mut state = diverged();
    state.remote.diverged_upstream = Some(("refs/remotes/origin/main".to_owned(), oid(THEIRS)));
    state.status = Status::Sticky {
        reason: "main and origin/main have diverged".to_owned(),
        keys: Some("m merge it  r rebase onto it".to_owned()),
    };
    let (dismissed, _) = press(&state, KeyPress::plain(KeyCode::Escape));
    assert_eq!(dismissed.remote.diverged_upstream, None);
    assert_eq!(dismissed.status, Status::Ready);
}

/// Review 08 Minor 9. `Esc` was not the only way the diverged line left the
/// screen: a job's `Done`, or any later refusal, replaced it and left the
/// remembered tip behind — so the next `m` silently merged the upstream
/// from whatever row the cursor was on, with nothing on screen saying so.
#[test]
fn the_remembered_upstream_dies_with_the_line_that_explained_it() {
    let mut state = diverged();
    state.remote.diverged_upstream = Some(("refs/remotes/origin/main".to_owned(), oid(THEIRS)));
    state.status = Status::Sticky {
        reason: "main and origin/main have diverged".to_owned(),
        keys: Some(crate::update::remote::DIVERGED_KEYS.to_owned()),
    };

    // Any event at all, with the line replaced by something else first.
    state.status = Status::Done("fetched origin".to_owned());
    let (after, _) = press(&state, plain('k'));

    assert_eq!(
        after.remote.diverged_upstream, None,
        "the tip goes with the words"
    );
    let (next, _) = press(&after, plain('r'));
    assert!(
        next.preview.is_none() || {
            let preview = next.preview.as_ref().expect("checked");
            !matches!(
                preview.subject.operation(),
                Some(Operation::Rebase(params)) if params.onto_tip == oid(THEIRS)
            )
        },
        "and `r` no longer aims at it"
    );
}
