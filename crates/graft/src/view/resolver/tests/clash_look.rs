//! The clash screen as the merge mock draws it (ease-pass/11): the list
//! column, the open clash's two sides, the marks a pick leaves, the solved
//! line. Over one fixture: `sara/fix-toc` merged into `wallet-release`,
//! `report.md` with two clashes (open here), `notes.md` with one.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::ref_name::RefName;
use graft_git::conflict::Take;
use graft_git::repo::HeadState;
use graft_git::state::InProgress;
use graft_ops::conflict::ConflictSet;

use super::{frame, open_on};
use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::resolver::RowState;
use crate::update::resolver::fixtures::{commit, oid, operation};

/// Two clashes, three plain runs around them.
pub(super) const REPORT: &[u8] = b"# Report\nintro\n\
<<<<<<< HEAD\nlet tax = round_cents(total * rate);\n=======\nlet tax = (total * rate).round_to(Cents);\n>>>>>>> sara/fix-toc\n\
middle\n\
<<<<<<< HEAD\nfooter.push_str(&refund_line(r));\n=======\nfooter.push_str(&format_refund(r, locale));\n>>>>>>> sara/fix-toc\n\
end\n";

/// `wallet-release` (checked out) at `oid(2)`, `sara/fix-toc` at `oid(3)`.
fn refs() -> Arc<crate::refs::RefBadges> {
    let name = |full: &str| RefName::new(full.as_bytes().to_vec()).expect("a valid ref name");
    let badges = crate::refs::badges(
        &[
            (name("refs/heads/wallet-release"), oid(2)),
            (name("refs/heads/sara/fix-toc"), oid(3)),
        ],
        &HeadState::Branch(name("refs/heads/wallet-release")),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    Arc::new(badges)
}

/// The fixture as `kind` stopped on it: `report.md` open, cursor on its
/// first clash; `notes.md` seen with its one clash open.
pub(super) fn stopped(kind: InProgress) -> AppState {
    stopped_with(kind, "notes.md")
}

/// [`stopped`], the second file at `notes` (a path, `docs/notes.md`).
pub(super) fn stopped_with(kind: InProgress, notes: &str) -> AppState {
    let mut state = open_on(REPORT, "report.md");
    let entry = |path: &str| graft_git::status::StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: Some(graft_git::status::Change::Modified),
        untracked: false,
        unmerged: true,
        unmerged_how: Some(graft_git::status::UnmergedHow::BothModified),
        submodule: None,
    };
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![entry(notes), entry("report.md")],
    }));
    state.refs = Some(refs());
    state.in_progress = Some(operation(kind, Some(commit(oid(3), "Fix the TOC"))));
    let open = state.resolver.as_mut().expect("open");
    open.files = Loading::Ready(Arc::new(ConflictSet {
        paths: vec![PathBuf::from(notes), PathBuf::from("report.md")],
        recorded: true,
        solved: Vec::new(),
    }));
    open.seen
        .insert(PathBuf::from(notes), RowState::Open { open: 1, of: 1 });
    state
}

/// The merge.
pub(super) fn merge() -> AppState {
    stopped(InProgress::Merge)
}

/// The row of `drawn` holding `needle`, or a panic showing the frame.
pub(super) fn row_with<'a>(drawn: &'a str, needle: &str) -> &'a str {
    drawn
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}:\n{drawn}"))
}

/// The cells of `line` from `from` (0-based), up to `to`.
fn cells(line: &str, from: usize, to: usize) -> String {
    line.chars().skip(from).take(to - from).collect()
}

/// M6: at 120 columns the list is a column 30 wide headed `Clashing
/// files`, each file `?` and its open clashes.
#[test]
fn at_120_the_list_is_a_column_of_clashing_files_with_their_open_count() {
    let drawn = frame(&merge(), 120, 30);
    let heading = row_with(&drawn, "Clashing files");
    assert!(heading.starts_with("Clashing files"), "{drawn}");
    // The column's `│` sits right after its 30 cells, on every list row.
    let report = row_with(&drawn, "report.md  ");
    assert_eq!(report.chars().nth(30), Some('│'), "{drawn}");
    let list = cells(report, 0, 30);
    assert!(list.contains("? report.md"), "{list:?}");
    assert!(list.trim_end().ends_with("2 open"), "{list:?}");
    let notes = cells(row_with(&drawn, "notes.md"), 0, 30);
    assert!(notes.contains("? notes.md"), "{notes:?}");
    assert!(notes.trim_end().ends_with("1 open"), "{notes:?}");
}

/// The document's part of `line`: the cells right of the list's `│`.
fn doc_part(line: &str) -> String {
    line.chars().skip(31).collect()
}

/// `state` with `takes` made on `report.md`, as the picks leave it.
pub(super) fn taken(mut state: AppState, takes: &[(usize, Take)]) -> AppState {
    let open = state.resolver.as_mut().expect("open");
    let file = Arc::make_mut(open.file.as_mut().expect("a document"));
    for (block, take) in takes {
        file.take(*block, *take).expect("the block is there");
    }
    state
}

/// M7: the clash under the cursor is two columns, each headed by its
/// side's one name, outlined; its lines carry no numbers, the plain lines
/// around it do.
#[test]
fn an_open_clash_is_two_named_columns_outlined_and_unnumbered() {
    let drawn = frame(&merge(), 120, 30);
    let labels = row_with(&drawn, "yours · wallet-release");
    assert!(labels.contains("theirs · sara/fix-toc"), "{drawn}");
    assert!(doc_part(labels).contains('?'), "{labels:?}");
    let pair = doc_part(row_with(&drawn, "round_cents"));
    assert!(
        pair.contains("(total * rate).round_to("),
        "side by side: {pair:?}"
    );
    let inside = pair.trim_start_matches([' ', '│', '?']);
    assert!(inside.starts_with("let tax"), "no number inside: {pair:?}");
    assert!(drawn.contains('┌') && drawn.contains('└'), "{drawn}");
    let intro = doc_part(row_with(&drawn, "intro"));
    let words: Vec<&str> = intro.split_whitespace().collect();
    assert_eq!(&words[..2], ["2", "intro"], "{intro:?}");
}

/// Width 100 still has the column and the two-column clash; 99 has the
/// strip, the sides stacked.
#[test]
fn at_100_the_clash_is_side_by_side_and_at_99_it_stacks() {
    let at_100 = frame(&merge(), 100, 30);
    assert!(row_with(&at_100, "Clashing files").starts_with("Clashing files"));
    let labels = row_with(&at_100, "yours · wallet-release");
    assert!(labels.contains("theirs · sara/fix-toc"), "{at_100}");
    let at_99 = frame(&merge(), 99, 30);
    assert!(!at_99.contains("Clashing files"), "{at_99}");
    let ours = row_with(&at_99, "yours · wallet-release");
    assert!(!ours.contains("theirs · sara/fix-toc"), "{at_99}");
    row_with(&at_99, "theirs · sara/fix-toc");
}

/// M8: a pick shrinks the clash to what it wrote, marked, numbered as the
/// plain lines are.
#[test]
fn a_taken_clash_is_its_kept_lines_marked_with_the_pick() {
    let ours = frame(&taken(merge(), &[(0, Take::Ours)]), 120, 30);
    let kept = doc_part(row_with(&ours, "round_cents"));
    assert!(
        kept.contains('◀') && kept.contains("kept yours"),
        "{kept:?}"
    );
    assert!(kept.split_whitespace().any(|word| word == "3"), "{kept:?}");
    assert!(!ours.contains("round_to(Cents)"), "{ours}");

    let theirs = frame(&taken(merge(), &[(0, Take::Theirs)]), 120, 30);
    let took = row_with(&theirs, "round_to(Cents)");
    assert!(
        took.contains('▶') && took.contains("took theirs"),
        "{took:?}"
    );
    assert!(!theirs.contains("round_cents"), "{theirs}");

    let both = frame(&taken(merge(), &[(0, Take::Both)]), 120, 30);
    assert!(row_with(&both, "round_cents").contains('◆'), "{both}");
    assert!(
        row_with(&both, "round_to(Cents)").contains("kept both"),
        "{both}"
    );
}

/// Q16 a: a re-take replaces the pick; the screen says what it is now.
#[test]
fn a_re_take_redraws_with_its_new_mark() {
    let state = taken(merge(), &[(0, Take::Ours), (0, Take::Theirs)]);
    let drawn = frame(&state, 120, 30);
    let took = row_with(&drawn, "round_to(Cents)");
    assert!(
        took.contains('▶') && took.contains("took theirs"),
        "{took:?}"
    );
    assert!(!drawn.contains("kept yours") && !drawn.contains("already taken"));
}

/// M9: a solved file stays, says so, and points on.
#[test]
fn a_solved_file_stays_and_points_to_the_next_file() {
    let state = taken(merge(), &[(0, Take::Ours), (1, Take::Theirs)]);
    let drawn = frame(&state, 120, 30);
    let solved = row_with(&drawn, "Every clash in report.md is solved.");
    assert!(solved.contains('✓'), "{solved:?}");
    assert!(solved.contains("n next file, 1 clash left"), "{solved:?}");
    let report = cells(row_with(&drawn, "report.md  "), 0, 30);
    assert!(report.contains("✓ report.md"), "{report:?}");
    assert!(report.trim_end().ends_with("done"), "{report:?}");
}

/// M6, M9: every clash solved — the list ends `✓ Review the merge  y`, the
/// solved line says `y`, and the notice row names no finish.
#[test]
fn all_solved_ends_the_list_with_the_review() {
    let mut state = taken(merge(), &[(0, Take::Ours), (1, Take::Ours)]);
    let open = state.resolver.as_mut().expect("open");
    open.seen
        .insert(PathBuf::from("notes.md"), RowState::AllTaken);
    let drawn = frame(&state, 120, 30);
    let tail = cells(row_with(&drawn, "Review the merge"), 0, 30);
    assert_eq!(tail.trim_end(), "✓ Review the merge  y", "{drawn}");
    let notes = cells(row_with(&drawn, "notes.md"), 0, 30);
    assert!(notes.trim_end().ends_with("done"), "{notes:?}");
    assert!(row_with(&drawn, "is solved.").contains("y review the merge"));
    assert!(!drawn.contains("every file done"), "{drawn}");
}

/// Q12: a rebase's clash is the same screen, its sides by the rebase's
/// names and its marks in the rebase's words.
#[test]
fn a_rebase_clash_draws_the_same_marks_in_its_words() {
    let state = stopped(InProgress::Rebase);
    let names = crate::update::resolver::side_names(&state);
    // Today's rebase names (`names.rs`): the pick is `yours · <sha7> …`, not
    // the ticket's `picked · <sha7> …` — a question in its NOTES.
    assert!(
        names.theirs.as_str().starts_with("yours · 0303030 "),
        "{names:?}"
    );
    let drawn = frame(&state, 120, 30);
    let labels = row_with(&drawn, names.ours.as_str());
    assert!(labels.contains(names.theirs.as_str()), "{drawn}");
    let kept = frame(&taken(state, &[(0, Take::Ours)]), 120, 30);
    assert!(row_with(&kept, "round_cents").contains('◀'), "{kept}");
    row_with(&kept, "kept upstream");
}
