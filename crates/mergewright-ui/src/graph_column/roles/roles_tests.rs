//! Lanes by role (ease-pass 07, G1, G2): the route decides each lane's
//! shape and colour token, the joining branch's role its connector.

use mergewright_core::capabilities::ColorDepth;
use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::layout::layout;
use mergewright_graph::row::Row;
use mergewright_graph::seeds::{BranchSeed, Seeds};
use mergewright_graph::{LaneRole, Route};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::graph_column::{GraphColumn, GraphLine};
use crate::paint::color;
use crate::test_support::terminal;
use crate::theme::{Theme, ThemeKind, Token};

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

fn commit(id: u8, parents: &[u8], subject: &str) -> Commit {
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: i64::from(id),
            offset_seconds: 0,
        },
    };
    Commit {
        id: oid(id),
        parents: parents.iter().map(|&byte| oid(byte)).collect(),
        author: who.clone(),
        committer: who,
        message: subject.as_bytes().to_vec(),
    }
}

fn name(branch: &str) -> BranchIdentity {
    BranchIdentity::of_name(branch.as_bytes())
}

/// `main ← wallet-release ← sara/fix-toc`, and `ops/ci` beside them. Top
/// to bottom: 0 main merges ops/ci, 1 main merges wallet-release, 2
/// wallet-release merges sara, 3 sara, 4 ops/ci (sara's lane runs past),
/// 5 sara, 6 wallet-release, 7 root.
fn chain_rows() -> Vec<Row> {
    layout(
        &[
            commit(10, &[9, 4], "Merge branch 'ops/ci'"),
            commit(9, &[1, 8], "Merge branch 'wallet-release'"),
            commit(
                8,
                &[6, 7],
                "Merge branch 'sara/fix-toc' into wallet-release",
            ),
            commit(7, &[5], "sara: fix the toc"),
            commit(4, &[1], "ops: ci"),
            commit(5, &[6], "sara: start the toc"),
            commit(6, &[1], "wallet: release work"),
            commit(1, &[], "root"),
        ],
        Seeds {
            default_branch: Some(BranchSeed {
                tip: oid(10),
                identity: name("main"),
                follows: None,
            }),
            ..Seeds::default()
        },
    )
}

fn roles(row: &Row, route: &Route) -> Vec<LaneRole> {
    row.lanes
        .iter()
        .map(|lane| lane.map_or(LaneRole::Other, |identity| route.role(identity)))
        .collect()
}

fn dark() -> Theme {
    Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false))
}

fn draw(rows: &[Row], route: &Route, ascii: bool) -> Buffer {
    let lines: Vec<GraphLine<'_>> = rows
        .iter()
        .map(|row| {
            let mut line = GraphLine::plain(row);
            line.roles = Some(roles(row, route));
            line
        })
        .collect();
    let area = Rect::new(0, 0, 10, u16::try_from(lines.len()).expect("few rows"));
    let mut buf = Buffer::empty(area);
    GraphColumn::new(&lines, &dark())
        .ascii(ascii)
        .render(area, &mut buf);
    buf
}

fn text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
}

fn all(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height).map(|y| text(buf, y)).collect()
}

/// The column (in cells) the lane of `branch` takes on `row`.
fn cell_of(row: &Row, branch: &str) -> u16 {
    let column = row
        .lanes
        .iter()
        .position(|lane| *lane == Some(name(branch)))
        .unwrap_or_else(|| panic!("{branch} has a lane on row {:?}", row.commit.id));
    u16::try_from(column * 2).expect("few lanes")
}

#[test]
fn each_lane_takes_its_roles_shape_and_colour() {
    let rows = chain_rows();
    let sara = Route::new(
        name("main"),
        &[name("wallet-release")],
        Some(name("sara/fix-toc")),
    );
    let buf = draw(&rows, &sara, false);
    let theme = dark();
    let drawn = all(&buf);

    // Row 4 is ops/ci's commit: main, wallet-release and sara run past it.
    let row = &rows[4];
    let at = |branch: &str| &buf[(cell_of(row, branch), 4)];
    assert_eq!(at("sara/fix-toc").symbol(), "┃", "this\n{drawn:#?}");
    assert_eq!(at("wallet-release").symbol(), "│", "built on\n{drawn:#?}");
    assert_eq!(at("main").symbol(), "║", "trunk\n{drawn:#?}");
    assert_eq!(at("sara/fix-toc").fg, color(theme.token(Token::RoleThis)));
    assert_eq!(
        at("wallet-release").fg,
        color(theme.token(Token::RoleUnder))
    );
    assert_eq!(at("main").fg, color(theme.token(Token::RoleTrunk)));

    // Row 3 is sara's commit: ops/ci runs past it.
    let ops = &buf[(cell_of(&rows[3], "ops/ci"), 3)];
    assert_eq!(ops.symbol(), "┊", "other\n{drawn:#?}");
    assert_eq!(ops.fg, color(theme.token(Token::RoleOther)));
}

#[test]
fn focus_moved_to_another_branch_redraws_the_lanes() {
    let rows = chain_rows();
    let ops = Route::unread(name("main"), Some(name("ops/ci")));
    let buf = draw(&rows, &ops, false);
    let row = &rows[3];
    let drawn = all(&buf);
    assert_eq!(buf[(cell_of(row, "ops/ci"), 3)].symbol(), "┃", "{drawn:#?}");
    assert_eq!(
        buf[(cell_of(row, "wallet-release"), 3)].symbol(),
        "┊",
        "{drawn:#?}"
    );
}

#[test]
fn a_connector_runs_in_the_joining_branchs_role() {
    let rows = chain_rows();
    let sara = Route::new(
        name("main"),
        &[name("wallet-release")],
        Some(name("sara/fix-toc")),
    );
    let ops = Route::unread(name("main"), Some(name("ops/ci")));

    // Row 2 is the merge of sara into wallet-release.
    let joined = text(&draw(&rows, &sara, false), 2);
    assert!(joined.contains('━'), "sara is this: {joined:?}");
    let joined = text(&draw(&rows, &ops, false), 2);
    assert!(joined.contains('┄'), "sara is other: {joined:?}");
}

#[test]
fn the_ascii_tier_says_role_with_bar_and_colon_only() {
    let rows = chain_rows();
    let sara = Route::new(
        name("main"),
        &[name("wallet-release")],
        Some(name("sara/fix-toc")),
    );
    for line in all(&draw(&rows, &sara, true)) {
        assert!(line.is_ascii(), "{line:?}");
        assert!(
            !line.contains(['┃', '║', '┊', '━', '═', '┄']),
            "no role shape survives ASCII: {line:?}"
        );
    }
    let drawn = all(&draw(
        &rows,
        &Route::unread(name("main"), Some(name("ops/ci"))),
        true,
    ));
    assert!(drawn.iter().any(|line| line.contains(':')), "{drawn:#?}");
}
