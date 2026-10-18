//! The merge review as the mock reads it, and where its `y` lands
//! (ease-pass/12 M15-M19), against a real merge of `sara/fix-toc` into
//! `wallet-release`: `report.md` two clashes (`o` then `t`), `notes.md` one
//! (`o`), two files git combined on its own. What landed is read back from
//! git. Colour unverified (rule 5); the look is the user's (rule 6).

use super::*;

/// Fixed commit times, so ids and date order are the same on every run.
const EPOCH: i64 = 1_760_000_000;

fn commit(repo: &Path, subject: &str, minute: i64) {
    git(repo, &["add", "--all"]);
    let when = format!("@{} +0000", EPOCH + minute * 60);
    graft_fixtures::git::run_at(repo, &["commit", "-q", "-m", subject], &when)
        .unwrap_or_else(|why| panic!("committing {subject:?}: {why}"));
}

const REPORT: &str = "# Report\n\nintro\n\n## Contents\nold toc\n\nmiddle\nmiddle\nmiddle\nmiddle\n\n## End\nold end\n";

/// `wallet-release` checked out, `sara/fix-toc` two commits ahead of the
/// base, both sides having changed `report.md` (two places) and `notes.md`;
/// `a.md` and `b.md` changed on her side only.
fn wallet_and_sara() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    git(repo, &["config", "user.name", "Graft Fixture"]);
    git(repo, &["config", "user.email", "fixture@example.invalid"]);
    git(repo, &["switch", "--quiet", "-c", "wallet-release"]);
    write(repo, "report.md", REPORT);
    write(repo, "notes.md", "the plan\n");
    write(repo, "a.md", "a\n");
    write(repo, "b.md", "b\n");
    commit(repo, "the report", 1);
    git(repo, &["switch", "--quiet", "-c", "sara/fix-toc"]);
    write(
        repo,
        "report.md",
        &REPORT
            .replace("old toc", "sara's toc")
            .replace("old end", "sara's end"),
    );
    write(repo, "notes.md", "sara's plan\n");
    commit(repo, "fix the toc", 2);
    write(repo, "a.md", "a, fixed\n");
    write(repo, "b.md", "b, fixed\n");
    commit(repo, "fix the links", 3);
    git(repo, &["switch", "--quiet", "wallet-release"]);
    write(
        repo,
        "report.md",
        &REPORT
            .replace("old toc", "release toc")
            .replace("old end", "release end"),
    );
    write(repo, "notes.md", "the release plan\n");
    commit(repo, "release notes", 4);
    scratch
}

/// Is the cursor's row carrying `name`'s branch badge?
fn on_branch(state: &AppState, name: &[u8]) -> bool {
    let (Some(refs), Some(row)) = (
        state.refs.as_deref(),
        state
            .history
            .as_ref()
            .and_then(graft::history::History::selected),
    ) else {
        return false;
    };
    refs.on(row.commit.id)
        .iter()
        .any(|label| matches!(label, graft::refs::RefLabel::Branch { name: n, .. } if n == name))
}

/// `m` on `sara/fix-toc`'s row, `y` in the merge box, and the resolver
/// landed on its first file.
fn merge_stopped(app: &App, state: AppState) -> AppState {
    let mut state = app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    });
    for _ in 0..20 {
        if on_branch(&state, b"sara/fix-toc") {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_branch(&state, b"sara/fix-toc"), "never found sara's row");
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the merge preview's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(preview.plan, graft::update::detail::Loading::Pending(_))
        })
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the merge", |state| state.operating.is_none());
    app.pump(state, "the landed resolver's first file", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.job.is_none() && (open.file.is_some() || open.whole_file.is_some())
        })
    })
}

/// The resolver on `path`, its read landed, nothing in flight.
fn on_file(app: &App, state: AppState, path: &str) -> AppState {
    app.pump(state, path, |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new(path) && open.job.is_none() && open.file.is_some()
            })
    })
}

/// `notes.md` `o`, `n`; `report.md` `o` then `t`; then `y`: the review, read.
fn the_review(app: &App, state: AppState) -> AppState {
    let state = on_file(app, merge_stopped(app, state), "notes.md");
    let state = crate::resolver::settled(app, app.key(&state, KeyCode::Char('o')));
    // A solved file stays on screen: `n` goes to the next (its own line).
    let state = app.key(&state, KeyCode::Char('n'));
    let state = on_file(app, state, "report.md");
    let state = crate::resolver::settled(app, app.key(&state, KeyCode::Char('o')));
    let state = crate::resolver::settled(app, app.key(&state, KeyCode::Char('t')));
    let state = app.key(&state, KeyCode::Char('y'));
    let state = super::review_landed(app, state);
    assert!(
        state.finish.is_some(),
        "y opens the review: {:?}",
        state.status
    );
    state
}

fn unmerged(repo: &Path) -> String {
    git(repo, &["ls-files", "-u", "--format=%(path)"])
}

/// The review at 120 reads the mock's lines in order (its files in git's
/// order), and nothing is staged by reading it.
#[test]
fn the_review_reads_the_mock_lines_and_stages_nothing() {
    let scratch = wallet_and_sara();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = the_review(&app, state);
    let subject = std::fs::read_to_string(repo.join(".git/MERGE_MSG"))
        .expect("git wrote it")
        .lines()
        .next()
        .expect("a first line")
        .to_owned();
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    let want = [
        "All 3 clashes solved. Nothing committed yet.".to_owned(),
        "This merge will put into wallet-release".to_owned(),
        "2 commits from sara/fix-toc".to_owned(),
        // In git's file order, which is the read's.
        "✓ notes.md   you chose: ◀ yours".to_owned(),
        "✓ report.md  you chose: ◀ yours (1) · ▶ theirs (1)".to_owned(),
        "✓ 2 files git combined on its own".to_owned(),
        format!("Message  {subject}"),
    ];
    let mut from = 0;
    for line in &want {
        let at = frame[from..]
            .iter()
            .position(|row| row.contains(line.as_str()))
            .unwrap_or_else(|| panic!("no {line:?} after row {from}:\n{body}"));
        from += at + 1;
    }
    let still = unmerged(repo);
    assert!(
        still.contains("report.md") && still.contains("notes.md"),
        "the review staged nothing: {still:?}"
    );
    crate::captures::check_shot("ease-pass/merge-review-120.txt", &format!("{body}\n"));
    let narrow = crate::captures::frame(&state, 90, 40).join("\n");
    crate::captures::check_shot("ease-pass/merge-review-90.txt", &format!("{narrow}\n"));
    drop(state);
    app.close();
}

/// The landed frame at 120, checked: the bar at 120 and 200 leads
/// `u undo the merge · P push` and has `space show`; one `✓`, the done
/// line's; the merge's fold reads `2 commits from sara/fix-toc`.
fn the_landed_frame(state: &AppState) -> String {
    for columns in [120, 200] {
        let frame = crate::captures::frame(state, columns, 40);
        let bar = frame.last().expect("a bar");
        assert!(
            bar.trim_start().starts_with("u undo the merge · P push") && bar.contains("space show"),
            "{columns}: {bar}"
        );
    }
    let body = crate::captures::frame(state, 120, 40).join("\n");
    assert!(
        body.contains("✓ Merged sara/fix-toc into wallet-release."),
        "{body}"
    );
    assert_eq!(body.matches('✓').count(), 1, "one tick:\n{body}");
    assert!(
        body.contains("2 commits from sara/fix-toc"),
        "the fold:\n{body}"
    );
    body
}

/// `y` lands on the merge commit with `✓ Merged sara/fix-toc into
/// wallet-release.`; once its count is known the row's bar is
/// `JustMerged`; `u` takes the merge back.
#[test]
fn y_lands_on_the_merge_and_u_takes_it_back() {
    let scratch = wallet_and_sara();
    let repo = scratch.path();
    let before = git(repo, &["rev-parse", "HEAD"]);
    let (app, state) = App::open(repo);
    let state = the_review(&app, state);
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the continue", |state| state.operating.is_none());
    let landed_at = std::time::Instant::now();
    let state = app.pump(state, "the landing the finish owes", |state| {
        state.in_progress.is_none() && !state.land_on_head
    });
    assert_eq!(
        state.status,
        Status::done("Merged sara/fix-toc into wallet-release."),
    );
    let head = git(repo, &["rev-parse", "HEAD"]);
    let parents = git(repo, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(parents.split_whitespace().count(), 3, "a merge: {parents}");
    assert_eq!(
        state.last_merge.map(|oid| oid.to_string()).as_deref(),
        Some(head.trim())
    );
    // The row's bar turns `JustMerged` once the count is known. Its
    // `space show the 2 commits inside` is drawn at its short rung,
    // `space show`, at 120 and at 200 alike (the row bar's fitter keeps more
    // hints over longer words; ticket 12 NOTES).
    let state = app.pump(state, "the merge's count and its bar", |state| {
        crate::captures::frame(state, 120, 40)
            .last()
            .is_some_and(|bar| bar.contains("u undo the merge"))
    });
    eprintln!(
        "ease-pass/12: JustMerged after {:?} on the fixture",
        landed_at.elapsed()
    );
    let body = the_landed_frame(&state);
    // The merge is made at run time: its id is zeroed, width kept.
    let shot = crate::captures::without_invented_id(&format!("{body}\n"), head.trim());
    crate::captures::check_shot("ease-pass/merge-landed-120.txt", &shot);

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the undo", |state| state.operating.is_none());
    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]),
        before,
        "back before the merge"
    );
    drop(state);
    app.close();
}

/// A fast-forward makes no merge commit, so its landing has no
/// `JustMerged` bar and no fold (ease-pass/12 M18; 05s's fixture): the
/// review never ran, `last_merge` stays empty.
#[test]
fn a_fast_forward_lands_with_no_just_merged() {
    let scratch = crate::merge_preview::fast_forward::behind();
    let repo = scratch.path();
    let tip = git(repo, &["rev-parse", "sara/fix-toc"]);
    let (app, state) = App::open(repo);
    let mut state = app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    });
    for _ in 0..20 {
        if on_branch(&state, b"sara/fix-toc") {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_branch(&state, b"sara/fix-toc"), "never found sara's row");
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the merge preview's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(preview.plan, graft::update::detail::Loading::Pending(_))
        })
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the fast-forward", |state| {
        state.operating.is_none() && !state.land_on_head
    });
    let state = crate::captures::at_rest(&app, state);
    assert_eq!(git(repo, &["rev-parse", "HEAD"]), tip, "moved up");
    assert_eq!(state.last_merge, None, "no merge was made");
    // Recorded in ticket 12's NOTES (its words were not specified).
    eprintln!("ease-pass/12: fast-forward status {:?}", state.status);
    let frame = crate::captures::frame(&state, 200, 40);
    let body = frame.join("\n");
    let bar = frame.last().expect("a bar");
    assert!(!bar.contains("undo the merge"), "{bar}");
    assert!(
        !body.contains("commits from sara/fix-toc"),
        "no fold:\n{body}"
    );
    drop(state);
    app.close();
}
