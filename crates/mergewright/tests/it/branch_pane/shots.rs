//! The branch pane's captures next to the main mock (ease-pass 09): his own
//! branch and a teammate's at 120, the teammate's at 100. Character grids:
//! colour unverified (CLAUDE.md rule 5); the look is the user's to judge.

use super::{MORE_RELEASE, TOC, cursor_on, drawn, opened, read, with_sibling};

const NOTE: &str = "ease-pass 09, drawn by `view::view` from a real AppState over the job \
     pool (crates/mergewright/tests/it/branch_pane/shots.rs): `main ← wallet-release (HEAD) \
     ← sara/fix-toc`, `ali/x` cut where sara cut hers, then a commit on wallet-release. \
     Judge beside prototypes/ease-mocks/mergewright-main-screen.html. Not a pty capture; \
     colour unverified (rule 5).";

/// One capture: the cursor moved onto `subject` with `j`, the read landed.
fn shot(name: &str, subject: &str, columns: u16, right: &str) {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), columns);
    let state = cursor_on(&app, state, subject);
    let state = read(&app, state, columns);
    let state = crate::captures::at_rest(&app, state);
    crate::captures::check_shot(
        &format!("ease-pass/{name}"),
        &format!(
            "{NOTE}\n{columns}x30. What right looks like: {right}\n\n{}\n",
            drawn(&state, columns).join("\n")
        ),
    );
    app.close();
}

#[test]
fn capture_the_branch_pane_reading() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = super::rest_with(&app, state, None);
    crate::captures::check_shot(
        "ease-pass/main-pane-reading-120.txt",
        &format!(
            "{NOTE}\n120x30. The pane's read withheld by the test harness \
             (`branch_pane::rest_with`): the moment before the answer lands. What right \
             looks like: the cursor on sara/fix-toc's row; the pane heads `sara/fix-toc`, \
             then four lines each `reading…`; no `!`, `✓` or `·`, no `built on` yet.\n\n{}\n",
            drawn(&state, 120).join("\n")
        ),
    );
    app.close();
}

#[test]
fn capture_the_branch_pane_own_teammate_and_narrow() {
    shot(
        "main-pane-own-120.txt",
        MORE_RELEASE,
        120,
        "the cursor on HEAD's row; right of a `│` rule the pane heads `wallet-release  \
         (you're here)`, four answers each led by `!`, `✓` or `·`, the who line naming the \
         author (`· <fixture author>, …`: this fixture sets no `user.email`, so not `you`), then `built \
         on` and its route `main ─◆ wallet-release`; no subject cut that would fit its column.",
    );
    shot(
        "main-pane-teammate-120.txt",
        TOC,
        120,
        "the cursor on sara/fix-toc's row, moved there with `j` alone; the pane heads \
         `sara/fix-toc`, four answers, and `built on` reads `main ─◆ sara/fix-toc`: \
         wallet-release has a commit after sara's cut, so it is not her base (01r R1); \
         never `ali/x`, whose lane draws `┊`.",
    );
    shot(
        "main-pane-100.txt",
        TOC,
        100,
        "the same at 100: the pane 30 wide, its words wrapped or cut at its edge, the rows \
         narrower beside it.",
    );
}
