//! J7 (m8-polish-and-ship/11): a rebase stopped on a conflict he has
//! resolved and added; `?` puts `IN PROGRESS` first with `c` Continue on
//! the cursor, and `⏎` asks for what `c` does there. Real `git`, the real pool.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::preview::dated_commit;

/// `feature` and `main` both edit `shared.txt`: rebasing `feature` onto
/// `main` stops on its one commit.
fn stopped_and_resolved() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, "shared.txt", "base\n", "add shared", 1);
    git(path, &["branch", "feature"]);
    dated_commit(path, "shared.txt", "main's line\n", "main edits shared", 2);
    git(path, &["switch", "feature"]);
    dated_commit(
        path,
        "shared.txt",
        "feature's line\n",
        "feature edits shared",
        3,
    );
    let stop = std::process::Command::new("git")
        .args(["rebase", "main"])
        .current_dir(path)
        .output()
        .expect("git runs");
    assert!(!stop.status.success(), "the rebase stops on the conflict");
    std::fs::write(path.join("shared.txt"), "both lines\n").expect("write the resolution");
    git(path, &["add", "shared.txt"]);
    scratch
}

fn rebasing(path: &std::path::Path) -> bool {
    let dir = git(path, &["rev-parse", "--git-dir"]);
    let dir = path.join(dir.trim());
    dir.join("rebase-merge").exists() || dir.join("rebase-apply").exists()
}

#[test]
fn j7_help_mid_rebase_puts_continue_first_and_enter_asks_what_c_asks() {
    let scratch = stopped_and_resolved();
    let path = scratch.path();
    assert!(rebasing(path));
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 36);
    let state = crate::conflict_interim::launched(&app, state);
    let state = crate::captures::settled_frame(&app, state);
    let state = app.key(&state, KeyCode::Char('?'));
    let frame = crate::captures::frame(&state, 120, 36);
    let shot = frame.join("\n");
    let headers: Vec<&String> = frame
        .iter()
        .filter(|row| {
            ["IN PROGRESS", "HERE", "EVERYWHERE"]
                .iter()
                .any(|h| row.contains(&format!("│{h}")) || row.contains(&format!("│ {h}")))
        })
        .collect();
    assert!(
        headers
            .first()
            .is_some_and(|row| row.contains("IN PROGRESS")),
        "IN PROGRESS first\n{shot}"
    );
    assert!(
        frame
            .iter()
            .any(|row| row.contains("▸c") && row.contains("Continue")),
        "the cursor on `c` Continue\n{shot}"
    );
    // `⏎` asks for what `c` asks for at this stop: the terminal handed to
    // `git rebase --continue` (`Command::ContinueInGit`). The handover is
    // the loop's, never the job pool's, so this harness cannot carry it
    // out: git's answer is not observed here, the command is.
    let (after, by_palette) = crate::app_harness::press_for_commands(
        &state,
        graft_core::event::KeyPress::plain(KeyCode::Enter),
    );
    assert!(after.finish.is_none() && after.operating.is_none());
    let closed = crate::app_harness::press_for_commands(
        &state,
        graft_core::event::KeyPress::plain(KeyCode::Escape),
    )
    .0;
    let (_, by_c) = crate::app_harness::press_for_commands(
        &closed,
        graft_core::event::KeyPress::plain(KeyCode::Char('c')),
    );
    assert_eq!(by_palette, by_c);
    assert!(
        matches!(
            by_palette.as_slice(),
            [graft::update::Command::ContinueInGit { .. }]
        ),
        "{by_palette:?}"
    );
    assert!(rebasing(path), "nothing ran yet");
    app.close();
}
