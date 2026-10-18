//! A fast-forward's merge box (ease-pass 05s, Minor 1): git makes no merge
//! commit and combines nothing, so the box says so — and `m y` only moves
//! the branch up.

use mergewright_core::event::KeyCode;

use super::{REPORT, authored, head, journal_len, merge_preview, open, screen};
use crate::app_harness::{git, pin_identity};
use crate::preview::dated_commit;

/// `wallet-release` checked out, with no commits of its own past where
/// `sara/fix-toc` forked: `sara/fix-toc` is three commits ahead
/// (Sara, Omar, Sara), adding `toc.md`, `notes.md` and `links.md`.
pub fn behind() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let path = scratch.path();
    pin_identity(path);
    git(path, &["switch", "-q", "-c", "wallet-release"]);
    dated_commit(path, "report.md", REPORT, "Add the report", 1);
    git(path, &["switch", "-q", "-c", "sara/fix-toc"]);
    authored(
        path,
        "toc.md",
        "toc\n",
        "Add a table of contents",
        "Sara Ahmed",
        3,
    );
    authored(path, "notes.md", "notes\n", "Add notes", "Omar Khan", 4);
    authored(
        path,
        "links.md",
        "links\n",
        "Check every link",
        "Sara Ahmed",
        5,
    );
    git(path, &["switch", "-q", "wallet-release"]);
    scratch
}

#[test]
fn a_fast_forward_says_it_only_moves_up_and_m_y_moves_it() {
    let scratch = behind();
    let repo = scratch.path();
    let tip = git(repo, &["rev-parse", "sara/fix-toc"]).trim().to_owned();
    let journal_before = journal_len(repo);
    let (app, state) = open(repo);
    let state = merge_preview(&app, state, "sara/fix-toc");
    let text = screen(&state).join("\n");
    for line in [
        "Merge sara/fix-toc into wallet-release",
        "│ Nothing has changed yet.",
        "│ Comes in      3 commits from sara/fix-toc (Sara, Omar)",
        "│ Changes       3 files",
        "│ Git combines  nothing: no merge commit is made",
        "│ You decide    nothing: no file clashes",
        "│ After y wallet-release moves up to sara/fix-toc.",
        "╰ y merge  esc cancel  ⏎ see the 3 commits ─",
    ] {
        assert!(text.contains(line), "{line:?} in\n{text}");
    }
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the fast-forward");
    assert!(state.preview.is_none(), "the box closed");
    assert_eq!(head(repo), tip, "HEAD is sara/fix-toc's tip");
    let parents = git(repo, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        2,
        "HEAD has one parent: {parents}"
    );
    assert_eq!(
        git(repo, &["symbolic-ref", "--short", "HEAD"]).trim(),
        "wallet-release",
        "still on wallet-release"
    );
    assert_eq!(journal_len(repo), journal_before + 1, "one journal entry");
    app.close();
}
