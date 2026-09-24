//! The resolver's file list on real stops (m6-conflicts/13): a file done
//! stays listed, the list comes back after a relaunch, a rebase lists only
//! its own step's files — and the captures the user judges by eye.

use graft::update::resolver::{RowState, listed_files};

use super::*;

/// A merge of `feature` into `main` stopped on two files: `gone.txt`
/// (changed on `main`, deleted on `feature` — a whole-file card) and
/// `notes.txt` (two one-line blocks, lines 2 and 7).
fn gone_and_notes_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let write = |name: &str, text: &str| std::fs::write(repo.join(name), text).expect("writable");
    write("notes.txt", "one\ntwo\nthree\nfour\nfive\nsix\nseven\n");
    write("gone.txt", "the plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "notes and a plan"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write(
        "notes.txt",
        "one\nTWO from feature\nthree\nfour\nfive\nsix\nSEVEN from feature\n",
    );
    git(repo, &["rm", "--quiet", "gone.txt"]);
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "shout, drop the plan"]);
    git(repo, &["checkout", "-q", "main"]);
    write(
        "notes.txt",
        "one\ntwo from main\nthree\nfour\nfive\nsix\nseven from main\n",
    );
    write("gone.txt", "our plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "annotate, keep the plan"]);
    git_stops(repo, &["merge", "feature"]);
    scratch
}

/// Each listed file's path and state, as the list derives them.
fn rows(state: &AppState) -> Vec<(String, RowState)> {
    listed_files(state)
        .into_iter()
        .map(|file| (file.path.display().to_string(), file.state))
        .collect()
}

/// The resolver on `notes.txt` with `gone.txt` done: `o` on the card, then
/// the list's set and the status both landed.
fn gone_kept(app: &App, state: AppState) -> AppState {
    let state = resolver_on_whole_file(app, state, "gone.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    assert!(state.operating.is_some(), "{:?}", state.status);
    let state = app.pump(state, "the keep and the next file", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new("notes.txt") && open.job.is_none() && open.file.is_some()
            })
    });
    // The status refresh and the set re-read land after the resolver has
    // moved on; wait for both rather than read the list mid-way.
    app.pump(state, "gone.txt listed as done", |state| {
        rows(state)
            .iter()
            .any(|(path, row)| path == "gone.txt" && *row == RowState::Done)
            && open(state).set_job.is_none()
    })
}

#[test]
fn a_done_file_stays_listed() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    assert_eq!(
        rows(&state),
        // `ConflictSet::paths` order: unmerged now first, then the ones
        // git's record names (graft-ops `conflict/set.rs:28`).
        vec![
            ("notes.txt".to_owned(), RowState::Open { open: 2, of: 2 }),
            ("gone.txt".to_owned(), RowState::Done),
        ]
    );
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(
        frame[1].contains("gone.txt · done ✓") && frame[1].contains("▸ notes.txt · 2 of 2 open"),
        "{body}"
    );
    assert!(
        frame[3].starts_with("▸ notes.txt · 2 conflicts · 2 open"),
        "{body}"
    );
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn the_list_survives_a_relaunch() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    drop(state);
    app.close();

    // A new app on the same stop: git's own record of the merge's set
    // still names `gone.txt`, now resolved.
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "notes.txt");
    let state = app.pump(state, "the set read", |state| {
        matches!(open(state).files, graft::update::detail::Loading::Ready(_))
    });
    assert_eq!(
        rows(&state),
        // `ConflictSet::paths` order: unmerged now first, then the ones
        // git's record names (graft-ops `conflict/set.rs:28`).
        vec![
            ("notes.txt".to_owned(), RowState::Open { open: 2, of: 2 }),
            ("gone.txt".to_owned(), RowState::Done),
        ]
    );
    let frame = crate::captures::frame(&state, 120, 40);
    assert!(
        frame[1].contains("gone.txt · done ✓"),
        "{}",
        frame.join("\n")
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// ADR-0043's list at every width the ticket names, and the user's own
/// 187×50.
#[test]
fn file_list_capture() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    for (columns, rows) in [(120u16, 40u16), (160, 48), (80, 24), (200, 50), (187, 50)] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        match columns {
            80 => assert!(
                frame[1].contains("✓ gone.txt") && frame[1].contains("▸ notes.txt 2/2 open"),
                "{body}"
            ),
            200 | 187 => assert!(
                frame[1].starts_with("FILES · 2 conflicted")
                    && frame.iter().any(|row| row.contains("▸ notes.txt")),
                "{body}"
            ),
            _ => assert!(
                frame[1].contains("gone.txt · done ✓")
                    && frame[1].contains("▸ notes.txt · 2 of 2 open"),
                "{body}"
            ),
        }
        crate::captures::check_shot(
            &format!("m6/fix-file-list-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}

/// Twelve files stopped in one merge, each a one-line block.
fn twelve_file_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let names: Vec<String> = (1..=12)
        .map(|i| format!("src/handlers/handler_{i:02}.rs"))
        .collect();
    std::fs::create_dir_all(repo.join("src/handlers")).expect("writable");
    let write_all = |line: &str| {
        for name in &names {
            std::fs::write(repo.join(name), format!("{line}\n")).expect("writable");
        }
    };
    write_all("fn handle() {}");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "twelve handlers"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write_all("fn handle() { theirs() }");
    git_as_graft(repo, &["commit", "-qam", "theirs"]);
    git(repo, &["checkout", "-q", "main"]);
    write_all("fn handle() { ours() }");
    git_as_graft(repo, &["commit", "-qam", "ours"]);
    git_stops(repo, &["merge", "feature"]);
    scratch
}

#[test]
fn file_list_many_capture() {
    let scratch = twelve_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "handler_01.rs");
    let state = app.pump(state, "the set read", |state| {
        matches!(open(state).files, graft::update::detail::Loading::Ready(_))
    });
    assert_eq!(rows(&state).len(), 12);
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(frame[3].trim_end().ends_with("more"), "{body}");
    assert!(
        frame[4].starts_with('─'),
        "three grid rows, then the rule: {body}"
    );
    crate::captures::check_shot("m6/fix-file-list-many-120x40.txt", &format!("{body}\n"));
    drop(state);
    app.close();
    drop(scratch);
}

/// Stops at step 2 of `TwoStopRebase` with the resolver open on `b.txt`.
fn at_step_two(app: &App, state: AppState) -> AppState {
    let state = resolver_on(app, state, "a.txt");
    let state = take_all_and_mark(app, state);
    assert!(state.resolver.is_none(), "a.txt was step 1's only file");
    assert!(state.banner_focus, "the banner has the keyboard for `c`");
    // The status refresh after the stage lands on its own; `c` counts
    // conflicts from it.
    let state = app.pump(state, "the status after the stage", |state| {
        state
            .working_tree
            .as_ref()
            .is_some_and(|tree| tree.entries.iter().all(|entry| !entry.unmerged))
    });
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(
        state.operating.is_some(),
        "`c` ran nothing: {:?}",
        state.status
    );
    let state = app.pump(state, "the continue", |state| state.operating.is_none());
    let state = app.pump(state, "the continue stopping at step 2", |state| {
        state.operating.is_none()
            && state.working_tree.as_ref().is_some_and(|tree| {
                tree.entries
                    .iter()
                    .any(|entry| entry.unmerged && entry.path == Path::new("b.txt"))
            })
    });
    // The stop may or may not have opened the resolver itself; cope with
    // either.
    let state = if state.resolver.is_some() {
        state
    } else {
        let state = on_the_banner(app, state);
        app.key(&state, KeyCode::Enter)
    };
    app.pump(state, "b.txt and its set", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new("b.txt")
                && open.file.is_some()
                && open.job.is_none()
                && matches!(open.files, graft::update::detail::Loading::Ready(_))
        })
    })
}

/// The selected graph row carries the branch badge `name`.
fn on_branch(state: &AppState, name: &str) -> bool {
    let (Some(refs), Some(row)) = (
        state.refs.as_deref(),
        state
            .history
            .as_ref()
            .and_then(graft::history::History::selected),
    ) else {
        return false;
    };
    refs.on(row.commit.id).iter().any(
        |label| matches!(label, graft::refs::RefLabel::Branch { name: found, .. } if found == name.as_bytes()),
    )
}

/// `TwoStopRebase`'s history with the corpus's shell rebase aborted, then
/// rebased again **by Graft** (`r` on `main`, confirmed), so `c` is
/// Graft's to run and the set has Graft's backup chain behind it — the
/// chain is where step 1's `a.txt` could leak into step 2.
fn graft_rebase_at_step_one() -> (Scratch, App, AppState) {
    use crate::app_harness::git;
    let scratch = graft_fixtures::copy_into_temp(Shape::TwoStopRebase);
    git(scratch.path(), &["rebase", "--abort"]);
    let (app, state) = opened_on(&scratch);
    let state = app.pump(state, "the graph and its badges", |state| {
        state.refs.is_some() && state.history.is_some()
    });
    let mut state = app.key(&state, KeyCode::Char('g'));
    for _ in 0..40 {
        if on_branch(&state, "main") {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_branch(&state, "main"), "no row carries `main`");
    let state = app.key(&state, KeyCode::Char('r'));
    let state = app.pump(state, "the rebase preview's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Pending(_) | graft::update::detail::Loading::Idle
            )
        })
    });
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the rebase stopping at step 1", |state| {
        state.operating.is_none()
            && state.in_progress.is_some()
            && state.working_tree.as_ref().is_some_and(|tree| {
                tree.entries
                    .iter()
                    .any(|entry| entry.unmerged && entry.path == Path::new("a.txt"))
            })
    });
    (scratch, app, state)
}

#[test]
fn a_rebase_lists_only_this_steps_files() {
    let (scratch, app, state) = graft_rebase_at_step_one();
    let state = at_step_two(&app, state);
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert_eq!(
        rows(&state)
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>(),
        vec!["b.txt".to_owned()],
        "{body}"
    );
    assert!(frame[1].contains("▸ b.txt"), "{body}");
    assert!(!body.contains("a.txt"), "{body}");
    crate::captures::check_shot(
        "m6/fix-file-list-rebase-step2-120x40.txt",
        &format!("{body}\n"),
    );

    // Every block taken: the row says so, and still no `a.txt`.
    let mut state = state;
    while open(&state).file.as_ref().expect("a document").open() > 0 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(&app, state);
    }
    assert_eq!(rows(&state), vec![("b.txt".to_owned(), RowState::AllTaken)]);
    let body = crate::captures::frame(&state, 120, 40).join("\n");
    assert!(!body.contains("a.txt"), "{body}");

    // `⏎` stages the step's last file: the resolver closes to the banner,
    // which says `c` goes on.
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage", |state| {
        state.operating.is_none() && state.resolver.is_none()
    });
    assert!(
        matches!(&state.status, Status::Done(line)
            if line == "every file done ✓ · c continues the rebase"),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
    drop(scratch);
}
