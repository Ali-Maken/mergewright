//! The clash screen's captures, beside the merge mock
//! (`prototypes/ease-mocks/mergewright-merge-screen.html`, M6–M9): frames
//! of the app over `clash_look`'s hand-built stop, for the user to judge by
//! eye (rule 6). No colour recorded (rule 5): the outline's open colour,
//! the marks' hues and the muted pick words are unverified.

use mergewright_git::conflict::Take;
use mergewright_git::state::InProgress;

use super::clash_look::{merge, stopped, taken};
use super::frame;
use crate::update::AppState;
use crate::update::resolver::RowState;

/// `state` with `notes.md` read and its one clash taken.
fn notes_solved(mut state: AppState) -> AppState {
    let open = state.resolver.as_mut().expect("open");
    open.seen
        .insert(std::path::PathBuf::from("notes.md"), RowState::AllTaken);
    state
}

/// One capture: its file's name, what right looks like, the state, and the
/// width it is drawn at.
type Shot = (&'static str, &'static str, AppState, u16);

/// Every capture, in the order the merge mock walks them.
fn shots() -> Vec<Shot> {
    let both = [(0, Take::Ours), (1, Take::Theirs)];
    vec![
        (
            "merge-clash-open-120",
            "the top row `merging sara/fix-toc into wallet-release`; the list a column 30 wide headed `Clashing files`: `? notes.md  1 open`, `▏? report.md  2 open` (the open file), then a blank row, `Combined by git (2)`, `✓ refund.rs, locale.rs`; the first clash outlined, `? yours · wallet-release` left, `theirs · sara/fix-toc` right, its lines unnumbered, the plain lines numbered; the bar starts `o take yours (left)`.",
            merge(),
            120,
        ),
        (
            "merge-clash-taken-120",
            "`o` on the first clash: it shrinks to `let tax = round_cents(total * rate);`, numbered 3, marked `◀`, ending `kept yours`; the second clash still open below; the list `report.md  1 open`; the bar `n next open clash · u undo pick`.",
            taken(merge(), &[(0, Take::Ours)]),
            120,
        ),
        (
            "merge-file-solved-120",
            "both clashes taken (`◀ kept yours`, `▶ took theirs`): under the file `✓ Every clash in report.md is solved.  n next file, 1 clash left`; the list `✓ report.md  done`; the bar names `n next file: notes.md`.",
            taken(merge(), &both),
            120,
        ),
        (
            "merge-all-solved-120",
            "every clash in both files taken: the list `✓ notes.md  done`, `✓ report.md  done`, then `✓ Review the merge  y`, a blank row, `Combined by git (2)`, `✓ refund.rs, locale.rs`; the solved line ends `y review the merge`; no `every file done` words on the notice row; the bar `y review the merge`.",
            notes_solved(taken(merge(), &both)),
            120,
        ),
        (
            "merge-clash-200",
            "200 columns (three panes retired): the same screen as at 120 — the 30-column list with `Combined by git (2)`, the clash's two sides side by side across the wider document, no `result` pane.",
            merge(),
            200,
        ),
        (
            "merge-clash-100",
            "100 columns, the narrowest with the list column: the clash still two columns side by side.",
            merge(),
            100,
        ),
        (
            "merge-clash-90",
            "90 columns: the list a strip over the file; the clash's sides stacked, each under its `?`-marked name.",
            merge(),
            90,
        ),
        (
            "rebase-clash-120",
            "a rebase stopped on the same file: the same screen, sides by the rebase's names (`upstream · …` left, `yours · …` right); the bar `o take upstream (left)`.",
            stopped(InProgress::Rebase),
            120,
        ),
    ]
}

#[test]
fn capture_the_clash_screen() {
    let mut captures = mergewright_fixtures::Captures::new();
    for (name, judge, state, columns) in shots() {
        let rows = 40;
        let body = format!(
            "# TestBackend capture (not a pty), no colour recorded (rule 5).\n\
             # Fixture: capture_the_clash_screen in\n\
             # crates/mergewright/src/view/resolver/tests/clash_shots.rs, a hand-built stop.\n\
             # Judge by eye: {judge}\n\
             === {columns}x{rows} ===\n{}\n",
            frame(&state, columns, rows)
        );
        captures.check(
            format!(
                "{}/../../docs/shots/ease-pass/{name}.txt",
                env!("CARGO_MANIFEST_DIR"),
            ),
            body,
        );
    }
    captures.finish();
}
