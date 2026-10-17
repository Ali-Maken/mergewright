//! The resolver's file list on real stops (m6-conflicts/13): a file done
//! stays listed, the list comes back after a relaunch, a rebase lists only
//! its own step's files — and the captures the user judges by eye.

use graft::update::resolver::{RowState, listed_files};

use super::*;

/// A merge of `feature` into `main` stopped on two files: `gone.txt`
/// (changed on `main`, deleted on `feature` — a whole-file card) and
/// `notes.txt` (two one-line blocks, lines 2 and 7).
pub(crate) fn gone_and_notes_merge() -> Scratch {
    gone_and_notes_merge_with(false)
}

/// [`gone_and_notes_merge`], and `feature` also changes one line of
/// `README.md`, which git merges on its own: the finish summary's
/// `COMBINED BY GIT ON ITS OWN` row (m6-conflicts/19, built by 35). A
/// sibling rather than a change to the fixture above, whose captures the
/// user has already judged.
pub(crate) fn gone_notes_and_readme_merge() -> Scratch {
    gone_and_notes_merge_with(true)
}

fn gone_and_notes_merge_with(readme: bool) -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let write = |name: &str, text: &str| std::fs::write(repo.join(name), text).expect("writable");
    if readme {
        // The merge commit is the app's `git merge --continue`: the
        // fixture's own name, so no machine's identity decides whether it
        // can commit.
        git(repo, &["config", "user.name", "Graft Fixture"]);
        git(repo, &["config", "user.email", "fixture@example.invalid"]);
        write("README.md", "# Project\n\nline one\nline two\nline three\n");
    }
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
    if readme {
        write("README.md", "# Project\n\nline one\nline TWO\nline three\n");
    }
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
pub(crate) fn rows(state: &AppState) -> Vec<(String, RowState)> {
    listed_files(state)
        .into_iter()
        .map(|file| (file.path.display().to_string(), file.state))
        .collect()
}

/// The resolver on `notes.txt` with `gone.txt` done: `o` on the card, then
/// the list's set and the status both landed.
fn gone_kept(app: &App, state: AppState) -> AppState {
    let state = resolver_on_whole_file(app, state, "gone.txt");
    keep_gone(app, &state)
}

/// [`gone_kept`] from the resolver already on `gone.txt`'s card.
fn keep_gone(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('o'));
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

/// The column `gone.txt` starts at on the list's row (row 1).
fn gone_column(frame: &[String]) -> usize {
    gone_like_column(frame, "gone.txt")
}

#[test]
fn a_done_file_stays_listed() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on_whole_file(&app, state, "gone.txt");
    let before = gone_column(&crate::captures::frame(&state, 120, 40));
    let state = keep_gone(&app, &state);
    assert_eq!(
        rows(&state),
        // git's path order, whatever became done (m6-conflicts/25): the
        // spec's order, not `ConflictSet::paths`' unmerged-first one.
        vec![
            ("gone.txt".to_owned(), RowState::Done),
            ("notes.txt".to_owned(), RowState::Open { open: 2, of: 2 }),
        ]
    );
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(
        frame[1].contains("gone.txt · done ✓    ▸ notes.txt · 2 of 2 open"),
        "{body}"
    );
    assert_eq!(
        gone_column(&frame),
        before,
        "gone.txt kept its cell: {body}"
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
        // git's path order (m6-conflicts/25).
        vec![
            ("gone.txt".to_owned(), RowState::Done),
            ("notes.txt".to_owned(), RowState::Open { open: 2, of: 2 }),
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

/// A left click on `name` in the list's row (row 1), at the app's own size.
pub(crate) fn click_file(app: &App, state: &AppState, name: &str) -> AppState {
    let frame = crate::captures::frame(state, state.size.columns, state.size.rows);
    let column = u16::try_from(gone_like_column(&frame, name)).expect("a narrow frame");
    app.mouse_down(state, graft_core::event::MouseButton::Left, column, 1)
}

/// The column `name`'s cell starts at on the list's row: its `▸` when it is
/// the open file, else its name (the list draws no placeholder for `▸`).
fn gone_like_column(frame: &[String], name: &str) -> usize {
    frame[1]
        .find(&format!("▸ {name}"))
        .or_else(|| frame[1].find(name))
        .map_or_else(
            || panic!("no {name} on the list row:\n{}", frame.join("\n")),
            |byte| frame[1][..byte].chars().count(),
        )
}

/// The listed paths, in the list's order.
fn listed_names(state: &AppState) -> Vec<String> {
    rows(state).into_iter().map(|(path, _)| path).collect()
}

/// The resolver open read-only on `name`, its done read landed.
fn done_open(app: &App, state: &AppState, name: &str) -> AppState {
    let state = click_file(app, state, name);
    app.pump(state, "the done file's read", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new(name) && open.job.is_none() && open.done.is_some()
        })
    })
}

/// `git status --porcelain`'s two-letter code for `name`.
fn porcelain(repo: &Path, name: &str) -> String {
    crate::app_harness::git(repo, &["status", "--porcelain", "--", name])
        .get(..2)
        .unwrap_or_default()
        .to_owned()
}

/// A done file of a merge **begun in the shell**: nothing kept its stages,
/// so ops cannot start it over (`NoStagesKept`) — and `d` is not offered.
/// Pressed anyway, it answers ops' words with no box and no write
/// (m6-conflicts/28; before, the box promised the markers and ops refused
/// after the confirm).
#[test]
fn a_shell_merge_done_file_answers_d_in_words() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    let order = listed_names(&state);
    let state = done_open(&app, &state, "gone.txt");
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert_eq!(frame[3].trim_end(), "▸ gone.txt · done ✓", "{body}");
    assert_eq!(listed_names(&state), order, "the row order is unchanged");
    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_none(), "no box");
    assert!(state.operating.is_none(), "no write");
    // Kept as ours, which is `HEAD`'s: no status line at all, and above all
    // not an unmerged one.
    assert_eq!(porcelain(scratch.path(), "gone.txt"), "");
    let open_now = open(&state);
    assert_eq!(open_now.path, Path::new("gone.txt"));
    assert!(open_now.done.is_some());
    assert_eq!(open_now.restarting, None, "never set");
    assert!(
        matches!(&state.status, Status::Sticky { reason, .. } if reason == "git can't bring back the conflict in gone.txt: the conflict's sides were not kept"),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// Graft's own merge stopped on `gone.txt` (modify/delete, a card) and
/// `shared.rs` (one block), with the banner's `⏎` landed on `gone.txt`'s
/// card and the list read.
pub(crate) fn on_gone_card(scratch: &Scratch) -> (App, AppState) {
    let (app, state) = App::open(scratch.path());
    let state = crate::conflict_interim::stopped_on_the_conflict(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the card and the set", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new("gone.txt")
                && open.whole_file.is_some()
                && open.set_job.is_none()
        })
    });
    (app, state)
}

/// From [`on_gone_card`]: `shared.rs` taken and marked done with `⏎`, the
/// resolver landed back on `gone.txt`'s card.
pub(crate) fn shared_done(app: &App, state: &AppState) -> AppState {
    let state = click_file(app, state, "shared.rs");
    let state = app.pump(state, "shared.rs", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.path == Path::new("shared.rs") && open.file.is_some())
    });
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(app, state);
    assert_eq!(open(&state).file.as_ref().expect("a document").open(), 0);
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "shared.rs staged, on to gone.txt", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.path == Path::new("gone.txt") && open.whole_file.is_some())
            && rows(state)
                .iter()
                .any(|(path, row)| path == "shared.rs" && *row == RowState::Done)
    })
}

/// The journey `d` exists for: a marker file taken, marked done, opened
/// read-only from the list, and started over. In a merge **Graft began**,
/// whose backup chain keeps the conflict's stages — in a shell-begun merge
/// there are none, and `graft_ops` refuses (`NoStagesKept`, above).
#[test]
fn a_done_marker_file_starts_over_in_place() {
    let scratch = super::restart::app_merge_with_a_whole_file_conflict();
    let repo = scratch.path().to_owned();
    let (app, state) = on_gone_card(&scratch);
    let state = shared_done(&app, &state);
    let before = listed_names(&state);
    assert_eq!(before, ["gone.txt", "shared.rs"]);
    // The journey from the landing after `⏎` (design note J11,
    // `walk-back-a-done-file`): click row · `d` · `→` · `⏎` = 4 inputs.
    // Every input the user gives goes through `input`, which counts it; the
    // pumps between them are the app answering, not the user.
    let mut inputs = 0;
    let mut input = |state: &AppState, step: &dyn Fn(&AppState) -> AppState| {
        inputs += 1;
        step(state)
    };
    let state = input(&state, &|state| click_file(&app, state, "shared.rs"));
    let state = app.pump(state, "the done file's read", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new("shared.rs") && open.job.is_none() && open.done.is_some()
        })
    });
    let cell = gone_like_column(&crate::captures::frame(&state, 120, 40), "shared.rs");
    assert!(!porcelain(&repo, "shared.rs").contains('U'));
    let state = input(&state, &|state| app.key(state, KeyCode::Char('d')));
    assert!(state.confirm.is_some(), "d's box: {:?}", state.status);
    let state = input(&state, &|state| app.key(state, KeyCode::Right));
    let state = input(&state, &|state| app.key(state, KeyCode::Enter));
    assert!(state.operating.is_some(), "{:?}", state.status);
    assert_eq!(inputs, 4, "the note's walk-back-a-done-file count");
    let state = app.pump(state, "the restart", |state| state.operating.is_none());
    let state = app.pump(state, "shared.rs read again, and the status", |state| {
        state.status_read.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new("shared.rs") && open.job.is_none() && open.file.is_some()
            })
            && rows(state)
                .iter()
                .any(|(path, row)| path == "shared.rs" && *row != RowState::Done)
    });
    assert_eq!(porcelain(&repo, "shared.rs"), "UU");
    assert_eq!(open(&state).done, None);
    assert_eq!(open(&state).cursor_block, Some(0), "the cursor on block 0");
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(frame[1].contains("▸ shared.rs · 1 of 1 open"), "{body}");
    assert_eq!(
        gone_like_column(&frame, "shared.rs"),
        cell,
        "the same cell: {body}"
    );
    // Its place in the list, not its column: the strip packs its cells, and
    // `gone.txt`'s cell is two narrower once its `▸` has gone.
    assert_eq!(
        listed_names(&state),
        before,
        "shared.rs kept its place: {body}"
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// A done file opened read-only: the list with `▸` on the done row, its
/// identity row, its plain lines, and the done bar. The merge was begun in
/// the shell, so nothing kept `gone.txt`'s stages: no `d` on the row or the
/// bar (m6-conflicts/28).
#[test]
fn done_open_capture() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    let state = done_open(&app, &state, "gone.txt");
    // The status line (`resolved gone.txt (ours) ✓`) holds the bar's row
    // until the next key; `PgUp` at the top scrolls nowhere and gives it
    // back (`k` steps files since m6-conflicts/17).
    let state = app.key(&state, KeyCode::PageUp);
    for (columns, rows) in [(120u16, 40u16), (200, 50), (187, 50)] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        assert!(body.contains("▸ gone.txt · done ✓"), "{body}");
        assert!(!body.contains("d starts it over"), "{body}");
        // `o` kept ours: the file on disk is main's plan.
        assert!(body.contains("our plan"), "{body}");
        // A done file has no sides left: one document at every width, never
        // the three panes, whose `theirs · feature` would put the file on
        // disk under the side that deleted it.
        assert!(!body.contains("theirs · feature"), "{body}");
        assert!(
            frame
                .last()
                .is_some_and(|bar| bar.trim() == "e edit  esc close"),
            "{body}"
        );
        if columns == 120 {
            assert!(
                frame[1].contains("▸ gone.txt · done ✓    notes.txt · 2 of 2 open"),
                "{body}"
            );
        }
        crate::captures::check_shot(
            &format!("m6/fix-file-list-done-open-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}

/// A click acknowledges a finished write's status line the way a key does
/// (m6-conflicts/42): after the list click that opens the done file, the
/// bottom row is the bar, not `resolved gone.txt (ours) ✓`.
#[test]
fn a_click_clears_the_done_status_like_a_key() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    assert!(
        matches!(state.status, graft::update::Status::Done { .. }),
        "{:?}",
        state.status
    );
    let state = done_open(&app, &state, "gone.txt");
    let frame = crate::captures::frame(&state, 120, 40);
    assert_eq!(
        frame.last().map(|bar| bar.trim()),
        Some("e edit  esc close"),
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
            // git's path order (m6-conflicts/25): `gone.txt` first.
            80 => assert!(
                frame[1].contains("✓ gone.txt   ▸ notes.txt 2/2 open"),
                "{body}"
            ),
            200 | 187 => {
                // The list's column only: left of its `│`.
                let at = |name: &str| {
                    frame.iter().position(|row| {
                        row.split('│')
                            .next()
                            .is_some_and(|list| list.contains(name))
                    })
                };
                assert!(
                    frame[1].starts_with("FILES · 2 conflicted")
                        && at("gone.txt").is_some()
                        && at("gone.txt") < at("▸ notes.txt"),
                    "{body}"
                );
            }
            _ => assert!(
                frame[1].contains("gone.txt · done ✓    ▸ notes.txt · 2 of 2 open"),
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
pub(super) fn at_step_two(app: &App, state: AppState) -> AppState {
    // m6-conflicts/18: the stop landed in the resolver on step 1's file.
    let state = app.pump(state, "the landed resolver on a.txt", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.path == Path::new("a.txt") && open.file.is_some())
    });
    let state = take_all_and_mark(app, state);
    // a.txt was step 1's only file: the resolver stays on it, done, and
    // `c` works from there (m6-conflicts/31).
    assert!(open(&state).done.is_some(), "a.txt was step 1's only file");
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
    // A rebase's `c` closes the resolver to the banner as it sends, so the
    // next stop opens it fresh (P5).
    assert!(state.resolver.is_none() && state.banner_focus);
    let state = app.pump(state, "the continue", |state| state.operating.is_none());
    let state = app.pump(state, "the continue stopping at step 2", |state| {
        state.operating.is_none()
            && state.working_tree.as_ref().is_some_and(|tree| {
                tree.entries
                    .iter()
                    .any(|entry| entry.unmerged && entry.path == Path::new("b.txt"))
            })
    });
    // m6-conflicts/18: the second stop lands in the resolver too.
    assert!(state.resolver.is_some(), "the stop opened the resolver");
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
pub(super) fn on_branch(state: &AppState, name: &str) -> bool {
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
pub(super) fn graft_rebase_at_step_one() -> (Scratch, App, AppState) {
    use crate::app_harness::git;
    let scratch = graft_fixtures::copy_into_temp(Shape::TwoStopRebase);
    git(scratch.path(), &["rebase", "--abort"]);
    let (app, state) = opened_on(&scratch);
    let state = rebased_onto_main_by_graft(&app, state, "a.txt");
    (scratch, app, state)
}

/// The branch checked out rebased onto `main` **by Graft** (`r` on `main`'s
/// row, confirmed), waited on until it stops with `first` unmerged.
pub(super) fn rebased_onto_main_by_graft(app: &App, state: AppState, first: &str) -> AppState {
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
    app.pump(state, "the rebase stopping at step 1", |state| {
        state.operating.is_none()
            && state.in_progress.is_some()
            && state.working_tree.as_ref().is_some_and(|tree| {
                tree.entries
                    .iter()
                    .any(|entry| entry.unmerged && entry.path == Path::new(first))
            })
    })
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

    // `⏎` stages the step's last file: the resolver stays on it, read back
    // as done (m6-conflicts/31); no summary for a rebase, the finish row
    // says `c` goes on instead of the notice (m6-conflicts/60).
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state
                .resolver
                .as_ref()
                .is_none_or(|open| open.done.is_some() && open.job.is_none())
    });
    assert_eq!(open(&state).notice, None);
    assert!(state.finish.is_none(), "no summary for a rebase");
    let body = crate::captures::frame(&state, 120, 40).join("\n");
    assert!(
        body.contains("✓ every file done — c continue the rebase"),
        "{body}"
    );
    drop(state);
    app.close();
    drop(scratch);
}

mod start_over;
