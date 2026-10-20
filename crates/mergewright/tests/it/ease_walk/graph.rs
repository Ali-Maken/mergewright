//! G1–G8 (G9–G14 in `bar_and_pane.rs`): the main mock's behaviours on the mock-shaped repository, one
//! test per id. Shapes and words only; every colour claim is the buffer's
//! attribute at most, never what a terminal shows (rule 5).

use super::fixture::{self, mock};
use super::screen::{
    buffer, cell, cursor_on, drawn, key_at_rest, line_with, node_column, open, rows_side, shown,
};

/// G1: the lanes' shapes by role against the cursor row's branch — `┃` its
/// own, `║` the trunk, `┊` every other — and they move with `j`/`k`.
#[test]
fn g1_lane_shape_is_the_role_against_the_cursor_branch() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let on_sara = cursor_on(&app, state, fixture::TOC_TWO);
    let frame = drawn(&on_sara);
    let sara = node_column(&frame, fixture::TOC_TWO);
    let integration = node_column(&frame, fixture::INTEGRATION);
    let main = node_column(&frame, fixture::BUMP);
    assert_eq!(cell(&frame, fixture::START_RELEASE, sara), '●');
    assert_eq!(
        cell(&frame, fixture::TIDY, sara),
        '┃',
        "sara's lane: {}",
        shown(&frame)
    );
    assert_eq!(
        cell(&frame, fixture::CI_MORE, integration),
        '┊',
        "{}",
        shown(&frame)
    );
    assert_eq!(
        cell(&frame, "Pin the CI image", main),
        '║',
        "trunk: {}",
        shown(&frame)
    );

    let on_integration = cursor_on(&app, on_sara, fixture::INTEGRATION);
    let frame = drawn(&on_integration);
    assert_eq!(
        cell(&frame, fixture::CI_MORE, integration),
        '┃',
        "{}",
        shown(&frame)
    );
    assert_eq!(cell(&frame, fixture::TIDY, sara), '┊', "{}", shown(&frame));
    app.close();
}

/// G1/G2, built on: on 01r's team repository (`wallet-release` moved),
/// sara's route is `main ─◆ develop ─◆ sara/fix-toc`; `develop` joins the
/// trunk with the thin `─` of a branch she is built on, and `ali/x`'s
/// lane, off her route, reads `┊`.
#[test]
fn g1_built_on_branch_draws_thin() {
    let scratch = fixture::team(true);
    let (app, state) = super::team::open_every_branch(scratch.path());
    let state = cursor_on(&app, state, fixture::TOC_TWO);
    let frame = drawn(&state);
    let readme = line_with(&frame, fixture::README);
    assert!(
        readme.contains("◇─╯"),
        "develop joins main thin: {readme:?}"
    );
    let ali = node_column(&frame, fixture::ALI);
    assert_eq!(
        cell(&frame, fixture::TOC_TWO, ali),
        '┊',
        "{}",
        shown(&frame)
    );
    app.close();
}

/// G2: connectors `╯` `╮` with `━` (the cursor's branch), `┄` (other) and
/// `═` (trunk) on the row where the lanes join main; `━` moves with the
/// cursor.
#[test]
fn g2_connectors_take_the_joining_branch_role() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let state = cursor_on(&app, state, fixture::TOC_TWO);
    let frame = drawn(&state);
    let join = line_with(&frame, fixture::BUMP);
    for glyph in ['╯', '┄', '═', '━'] {
        assert!(join.contains(glyph), "{glyph} on {join:?}");
    }
    assert!(line_with(&frame, fixture::SYNC_MERGE).contains('╮'));
    let first_heavy = |line: &str| line.chars().position(|c| c == '━');
    let on_sara = first_heavy(join);
    let state = cursor_on(&app, state, fixture::INTEGRATION);
    let frame = drawn(&state);
    let on_integration = first_heavy(line_with(&frame, fixture::BUMP));
    assert_ne!(on_sara, on_integration, "the heavy join follows the cursor");
    app.close();
}

/// G3: `●` a commit, `◉` a merge.
#[test]
fn g3_commit_dot_and_merge_ring() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let frame = drawn(&state);
    assert_eq!(
        cell(
            &frame,
            fixture::PR_MERGE,
            node_column(&frame, fixture::PR_MERGE)
        ),
        '◉'
    );
    assert_eq!(
        cell(
            &frame,
            fixture::SYNC_MERGE,
            node_column(&frame, fixture::SYNC_MERGE)
        ),
        '◉'
    );
    assert_eq!(
        cell(
            &frame,
            fixture::INTEGRATION,
            node_column(&frame, fixture::INTEGRATION)
        ),
        '●'
    );
    app.close();
}

/// G4: merged branches folded by default with their tail; `space` opens
/// `▾`, the rows under it dim with the branch's last path segment; `space`
/// on one of them closes it and puts the cursor on the merge.
#[test]
fn g4_merged_branches_fold_and_space_opens_and_closes() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let frame = drawn(&state);
    assert!(
        !frame.iter().any(|line| line.contains(fixture::RECEIPTS)),
        "{}",
        shown(&frame)
    );
    let merge = line_with(&frame, fixture::NAMED_MERGE);
    assert!(merge.contains("▸ 1 commit from dana/receipts"), "{merge:?}");

    let state = cursor_on(&app, state, fixture::NAMED_MERGE);
    let state = key_at_rest(&app, &state, ' ');
    let frame = drawn(&state);
    assert!(line_with(&frame, fixture::NAMED_MERGE).contains("▾ 1 commit from dana/receipts"));
    let inside = line_with(&frame, fixture::RECEIPTS);
    assert!(
        inside.contains(" receipts "),
        "last path segment in the branch column: {inside:?}"
    );

    let state = key_at_rest(&app, &state, 'j');
    assert!(super::screen::selected_subject(&state).starts_with(fixture::RECEIPTS));
    let state = key_at_rest(&app, &state, ' ');
    let frame = drawn(&state);
    assert!(!frame.iter().any(|line| line.contains(fixture::RECEIPTS)));
    assert!(super::screen::selected_subject(&state).starts_with(fixture::NAMED_MERGE));
    app.close();
}

/// G5: the branch column names a change of branch once, `(you)` on
/// `HEAD`'s. Checked once his commit lands on `wallet-release`: before it,
/// `wallet-release`'s rows share sara's lane and the column never names it
/// (`KNOWN_GAPS.md` `ease/07c-wallet-no-column`, confirmed by this walk).
#[test]
fn g5_branch_column_once_per_change_with_you_on_head() {
    let scratch = mock();
    fixture::with_edits(scratch.path());
    let (app, state) = open(scratch.path(), 120);
    let state = super::commit::committed(&app, &state, "fix toc");
    let frame = rows_side(&drawn(&state), 120);
    let head = line_with(&frame, "fix toc");
    assert!(head.contains("wallet-release (you)"), "{head:?}");
    let named = |name: &str| frame.iter().filter(|line| line.contains(name)).count();
    assert_eq!(named("integration "), 1, "{}", shown(&frame));
    assert_eq!(named("pay-307 "), 1, "{}", shown(&frame));
    app.close();
}

/// G6: the message on an `Other` row is drawn fainter than on the cursor's
/// own: its cells' style differs (buffer attribute; how faint is for his
/// eye, colour unverified).
#[test]
fn g6_other_rows_draw_their_message_fainter() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let state = cursor_on(&app, state, fixture::TOC_TWO);
    let frame = drawn(&state);
    let cells = buffer(&state);
    let style_of = |subject: &str| {
        let y = super::screen::row_of(&frame, subject);
        let x = line_with(&frame, subject)
            .find(subject)
            .map(|bytes| line_with(&frame, subject)[..bytes].chars().count());
        let x = u16::try_from(x.unwrap_or(0)).unwrap_or(0);
        let cell = &cells[(x + 1, u16::try_from(y).unwrap_or(0))];
        (cell.fg, cell.modifier)
    };
    assert_ne!(style_of(fixture::INTEGRATION), style_of(fixture::TOC));
    app.close();
}

/// G7: muted labels after the message: `↑1 to push`, `not merged`,
/// `<branch> is here`, `tag v1.2`; a branch and its upstream level are one
/// name (no `origin/main is here`).
#[test]
fn g7_labels_after_the_message() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let frame = rows_side(&drawn(&state), 200);
    assert!(
        line_with(&frame, fixture::TOC_TWO)
            .contains("toc: number the headings  ↑1 to push  not merged")
    );
    assert!(line_with(&frame, fixture::RELEASE_REPORT).contains("wallet-release is here"));
    assert!(line_with(&frame, fixture::README).contains("Add the readme  tag v1.2"));
    assert!(
        !frame.iter().any(|line| line.contains("origin/main")),
        "{}",
        shown(&frame)
    );
    assert!(
        !frame
            .iter()
            .any(|line| line.contains("origin/wallet-release"))
    );
    app.close();
}

/// G8: who is a first name, his own `you`; age `27h`; no SHA column, no
/// `[…]` badge.
#[test]
fn g8_who_first_name_age_and_no_sha_or_badge() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let frame = rows_side(&drawn(&state), 120);
    let trimmed = |subject: &str| line_with(&frame, subject).trim_end().to_owned();
    assert!(
        trimmed(fixture::INTEGRATION).ends_with("Bob  27h"),
        "{}",
        trimmed(fixture::INTEGRATION)
    );
    assert!(trimmed(fixture::TIDY).ends_with("you  27h"));
    assert!(trimmed(fixture::CI).ends_with("Omar  28h"));
    let hex7 = |word: &str| word.len() == 7 && word.chars().all(|c| c.is_ascii_hexdigit());
    for line in frame
        .iter()
        .filter(|line| crate::row_reads::is_commit_row(line))
    {
        assert!(!line.contains('['), "no badge: {line:?}");
        assert!(!line.split_whitespace().any(hex7), "no SHA: {line:?}");
    }
    app.close();
}
