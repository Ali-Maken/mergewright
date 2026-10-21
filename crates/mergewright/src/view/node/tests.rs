//! ease-pass 06r M1, M2: a history row's node is one answer in both tiers.
//! Read through the frame builders (`graph_view`, `pixel_rows`), not
//! `Node::of` alone, so a tier that stops asking it fails here.

use std::collections::HashMap;
use std::sync::Arc;

use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::HeadState;
use mergewright_graph::layout::layout;
use mergewright_graph::raster::Ring;
use mergewright_graph::raster::cache::STRIP_ROWS;
use mergewright_graph::row::Row;
use mergewright_graph::seeds::Seeds;
use mergewright_ui::graph_column::NodeKind;

use crate::history::{History, Phase, Window};
use crate::session::for_tests::session;
use crate::update::{AppState, Size};
use crate::view::graph_view;
use crate::view::strips::{StripPaint, pixel_rows};
use crate::view::tests::oid;

/// Each row of [`every_node`], newest first: its commit and the shape its
/// node draws as.
pub(crate) const EVERY_NODE: [(u8, &str); 12] = [
    (12, "head"),
    (11, "stash"),  // stash@{0}
    (10, "commit"), // stash@{1}, with untracked files: three parents
    (9, "merge"),   // folded: one parent drawn
    (8, "merge"),   // opened: both drawn
    (7, "merge"),   // in the first-parent view: one drawn
    (6, "commit"),  // 8's side
    (5, "commit"),
    (4, "commit"),
    (3, "commit"),
    (2, "commit"),
    (1, "root"),
];

fn commit(id: u8, parents: &[u8]) -> Commit {
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
        parents: parents.iter().map(|&parent| oid(parent)).collect(),
        author: who.clone(),
        committer: who,
        message: format!("Commit {id}").into_bytes(),
    }
}

/// [`EVERY_NODE`]'s rows, laid out from the parents each view draws and
/// then given git's own (06 D1): the stash entries their index and
/// untracked parents, the folded merge and the merge in `1` their second.
fn every_row() -> Vec<Row> {
    let drawn = [
        commit(12, &[11]),
        commit(11, &[10]),
        commit(10, &[9]),
        commit(9, &[8]),
        commit(8, &[7, 6]),
        commit(7, &[5]),
        commit(6, &[5]),
        commit(5, &[4]),
        commit(4, &[3]),
        commit(3, &[2]),
        commit(2, &[1]),
        commit(1, &[]),
    ];
    let mut rows = layout(&drawn, Seeds::default());
    let hidden: HashMap<Oid, &[u8]> = HashMap::from([
        (oid(11), &[0xe0][..]),
        (oid(10), &[0xe1, 0xe2][..]),
        (oid(9), &[0xe3][..]),
        (oid(7), &[0xe4][..]),
    ]);
    for row in &mut rows {
        if let Some(more) = hidden.get(&row.commit.id) {
            row.commit
                .parents
                .extend(more.iter().map(|&parent| oid(parent)));
        }
    }
    let ids: Vec<Oid> = rows.iter().map(|row| row.commit.id).collect();
    assert_eq!(ids, EVERY_NODE.map(|(id, _)| oid(id)), "the order assumed");
    rows
}

/// [`every_row`] as the app holds it: `HEAD` detached at 12; `stash@{0}`
/// is 11, `stash@{1}` 10.
pub(crate) fn every_node(cursor: usize) -> AppState {
    let rows = every_row();
    let refs = crate::refs::badges(
        &[],
        &HeadState::Detached(oid(12)),
        &HashMap::<RefName, RefName>::new(),
        &[(oid(11), 0), (oid(10), 1)],
        |_, _| unreachable!("no branches to count"),
    )
    .expect("nothing to count");
    let count = rows.len();
    AppState {
        history: Some(History {
            window: Some(Window {
                request: 0,
                first: 0,
                laid_out: count,
                walked: count,
                hidden: 0,
                done: true,
                rows: rows.into(),
                promoted: false,
                fold: [].into(),
                sought: None,
            }),
            rows: count,
            phase: Phase::Complete,
            cursor,
            reanchor: None,
            ..History::started(crate::config::MergedBranches::Folded)
        }),
        refs: Some(Arc::new(refs)),
        size: Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    }
}

/// The Unicode tier's node per drawn commit, top to bottom.
pub(crate) fn unicode(state: &AppState) -> Vec<(Oid, NodeKind)> {
    graph_view(state, 0)
        .rows
        .iter()
        .map(|entry| (entry.line.row.commit.id, entry.line.node))
        .collect()
}

/// The pixel tier's ring per drawn commit, top to bottom.
fn pixel(state: &AppState) -> Vec<(Oid, Ring)> {
    let frame = pixel_rows(state, 12, STRIP_ROWS);
    let paint = StripPaint::of(&session());
    frame
        .strips
        .iter()
        .flat_map(|strip| {
            strip.with_layers(&frame, &paint, |plain, _| {
                plain
                    .rows
                    .iter()
                    .map(|row| (row.row.commit.id, row.ring))
                    .collect::<Vec<_>>()
            })
        })
        .collect()
}

/// A Unicode node's shape, in [`EVERY_NODE`]'s words.
fn kind_shape(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Head => "head",
        NodeKind::Stash => "stash",
        NodeKind::Merge => "merge",
        NodeKind::Root => "root",
        NodeKind::Commit => "commit",
        other => panic!("no history row here draws {other:?}"),
    }
}

/// A pixel ring's shape, in [`EVERY_NODE`]'s words.
fn ring_shape(ring: Ring) -> &'static str {
    match ring {
        Ring::Head => "head",
        Ring::Stash => "stash",
        Ring::Merge => "merge",
        Ring::Root => "root",
        Ring::Commit => "commit",
        Ring::Uncommitted => panic!("no uncommitted row here"),
    }
}

/// M1: an older stash entry (two or three of git's parents since 06 D1)
/// draws `●`, never `◉`; the newest draws the stash's `■`; a folded merge
/// still draws `◉`.
#[test]
fn an_older_stash_entry_never_draws_as_a_merge() {
    let drawn: HashMap<Oid, NodeKind> = unicode(&every_node(0)).into_iter().collect();
    for (id, kind) in [
        (11, NodeKind::Stash),
        (10, NodeKind::Commit),
        (9, NodeKind::Merge),
        (5, NodeKind::Commit),
    ] {
        assert_eq!(drawn.get(&oid(id)), Some(&kind), "commit {id}");
    }
}

/// M2: the pixel ring and the Unicode node name the same shape, row for
/// row: `HEAD`, both stash entries, a folded merge, an opened one, a merge
/// in `1`, plain commits and the root.
#[test]
fn the_pixel_ring_and_the_unicode_node_agree_on_every_row() {
    let state = every_node(0);
    let want: Vec<(Oid, &str)> = EVERY_NODE
        .iter()
        .map(|&(id, shape)| (oid(id), shape))
        .collect();
    let unicode: Vec<(Oid, &str)> = unicode(&state)
        .into_iter()
        .map(|(id, kind)| (id, kind_shape(kind)))
        .collect();
    let pixel: Vec<(Oid, &str)> = pixel(&state)
        .into_iter()
        .map(|(id, ring)| (id, ring_shape(ring)))
        .collect();
    assert_eq!(unicode, want, "the Unicode tier");
    assert_eq!(pixel, want, "the pixel tier");
}
