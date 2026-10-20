//! m7-power-tools/11: `o` takes a commit onto the branch (moved out of
//! `picker.rs` for the file-size ratchet).

use super::*;

/// The line a landed take leaves.
fn took(subject: &str, also: Option<&str>) -> Status {
    Status::Done {
        what: format!("took \"{subject}\" onto main"),
        also: also.map(str::to_owned),
        keys: Some("u undoes".to_owned()),
    }
}

/// `cherry-pick-in-view`: `(motion) o y` over an unsaved edit — `login-page`'s
/// commit lands on `main` (one commit, its file there), the edit back, the
/// cursor on the new commit. Captures the preview at 120×40 on the way. Then
/// `u`: `main` as before, the edit still there.
#[test]
fn cherry_pick_in_view() {
    let scratch = picker_repo();
    let path = scratch.path();
    std::fs::write(path.join("base.txt"), EDIT).expect("the scratch repo is writable");
    let pick = rev(path, "login-page");
    let main_before = rev(path, "main");
    let (app, state) = App::open(path);
    let state = crate::context_menu::settled(&app, app.resize(&ready(&app, state), 120, 40));
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = reach(&mut walk, state, &pick);
    let state = walk.press(&state, KeyCode::Char('o'), "o");
    let state = planned(&app, state);

    let title = format!("Preview: take {} \"fix the bug\" onto main", &pick[..7]);
    let text = drawn(&state, 120, 40);
    assert!(text.contains(&title), "`{title}`:\n{text}");
    assert!(text.contains("y confirm"), "{text}");
    assert!(text.contains("esc cancel"), "{text}");
    check_shot(
        "m7/take-preview-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "take-preview-120x40.txt",
                "cherry_pick_in_view",
                "`o` on login-page's commit, main checked out, base.txt edited —\n\
                 # the title `Preview: take <sha7> \"fix the bug\" onto main`; the\n\
                 # footer `y confirm … esc cancel`; the set-aside line (the edit is\n\
                 # put aside and back)."
            )
        ),
    );

    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the take"));

    // The graph opens on its newest row, which here is the very commit
    // wanted: reaching it costs nothing, so the walk is `o y` (2), under the
    // note's `motion o y` (3). `cherry_pick_other_branch` walks a real motion.
    assert_eq!(walk.keys, ["o", "y"]);
    assert!(walk.keys.len() <= noted_keys("cherry-pick-in-view"));
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(rev(path, "main~1"), main_before, "one commit on main");
    assert!(
        path.join("fix.txt").exists(),
        "the commit's change is on HEAD"
    );
    assert_eq!(read(path, "base.txt"), EDIT, "the edit is back");
    y_takes_it_and_lands_on_the_new_commit(&state, path);

    let state = undo(&mut walk, &state);
    assert_eq!(rev(path, "main"), main_before, "`u` removed it");
    assert!(!path.join("fix.txt").exists(), "and its change");
    assert_eq!(read(path, "base.txt"), EDIT, "the edit survives the undo");
    drop(state);
    app.close();
}

/// The ticket's `y_takes_it_and_lands_on_the_new_commit`, walked inside
/// [`cherry_pick_in_view`]: the status line exact, the cursor on the new
/// `HEAD`.
fn y_takes_it_and_lands_on_the_new_commit(state: &AppState, path: &std::path::Path) {
    assert_eq!(
        state.status,
        took("fix the bug", Some("your edits are back"))
    );
    assert_eq!(
        cursor(state),
        Some(rev(path, "main")),
        "the cursor on the new commit"
    );
    let text = drawn(state, 120, 40);
    let line = "\u{2713} took \"fix the bug\" onto main \u{b7} your edits are back  u undoes";
    assert!(text.contains(line), "`{line}`:\n{text}");
}

/// `cherry-pick-other-branch`: `C <type> ⇥ motion o y` — the picker puts the
/// cursor on `login-page`'s tip, one row down is the commit wanted (not the
/// tip), `o y` takes it onto `main`. Then `u`.
#[test]
fn cherry_pick_other_branch() {
    let scratch = picker_repo();
    let path = scratch.path();
    git(path, &["switch", "login-page"]);
    dated_commit(path, "typo.txt", "typo\n", "fix the typo", 4);
    git(path, &["switch", "main"]);
    let pick = rev(path, "login-page~1");
    let main_before = rev(path, "main");
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.open(&state);
    let state = walk.type_in(&state, "login");
    let state = walk.press(&state, KeyCode::Tab, "⇥");
    assert!(state.picker.is_none(), "⇥ closed the picker");
    assert_eq!(
        cursor(&state),
        Some(rev(path, "login-page")),
        "⇥ put the cursor on the tip"
    );
    let state = reach(&mut walk, state, &pick);
    let state = walk.press(&state, KeyCode::Char('o'), "o");
    let state = planned(&app, state);
    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the take"));

    assert_eq!(walk.keys, ["C", "<type>", "⇥", "motion", "o", "y"]);
    assert!(walk.keys.len() <= noted_keys("cherry-pick-other-branch"));
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(rev(path, "main~1"), main_before, "one commit on main");
    assert!(
        path.join("fix.txt").exists(),
        "the commit's change is on HEAD"
    );
    assert!(!path.join("typo.txt").exists(), "and only that commit's");
    assert_eq!(state.status, took("fix the bug", None));
    assert_eq!(cursor(&state), Some(rev(path, "main")));

    let state = undo(&mut walk, &state);
    assert_eq!(rev(path, "main"), main_before, "`u` removed it");
    drop(state);
    app.close();
}

/// `o` on a merge: refused on the status line in `check_commit`'s words, no
/// preview box. Captured at 120×40.
#[test]
fn take_refused_on_a_merge() {
    let scratch = picker_repo();
    let path = scratch.path();
    git(path, &["switch", "-c", "merged", "feature-a"]);
    let when = format!("@{} +0000", crate::preview::FORK_EPOCH + 5 * 60);
    mergewright_fixtures::git::run_at(
        path,
        &["merge", "--no-ff", "-m", "merge login-page", "login-page"],
        &when,
    )
    .unwrap_or_else(|why| panic!("merging: {why}"));
    git(path, &["switch", "main"]);
    let merge = rev(path, "merged");
    let (app, state) = App::open(path);
    let state = crate::context_menu::settled(&app, ready(&app, state));
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = reach(&mut walk, state, &merge);
    let state = walk.press(&state, KeyCode::Char('o'), "o");
    assert!(state.preview.is_none(), "no preview box");
    let words = format!(
        "{} is a merge \u{b7} taking a merge isn't in Mergewright yet",
        &merge[..7]
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: words.clone(),
            keys: None
        }
    );
    let text = drawn(&state, 120, 40);
    assert!(text.contains(&words), "`{words}`:\n{text}");
    check_shot(
        "m7/take-refused-merge-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "take-refused-merge-120x40.txt",
                "take_refused_on_a_merge",
                "`o` on a merge commit — the status line `<sha7> is a merge ·\n\
                 # taking a merge isn't in Mergewright yet`; no preview box."
            )
        ),
    );
    drop(state);
    app.close();
}
