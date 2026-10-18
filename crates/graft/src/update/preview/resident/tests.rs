use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_ops::operation::{Operation, RebaseParams, Source, Tip};
use graft_ops::preview::Target;
use graft_ui::graph_column::NodeKind;

use crate::update::keys::view_change::open_merge;
use crate::update::preview::tests::a_plan;
use crate::update::preview::{Planned, PreviewSubject, open_preview, preview_produced};
use crate::update::{AppState, Command};
use crate::view::node::tests::{every_node, unicode};
use crate::view::tests::oid;

/// Opens a preview rebasing `feature` (3) onto 2: the slice ids its plan
/// job was asked about.
fn open_a_rebase(state: &mut AppState) -> Vec<HashSet<Oid>> {
    let mut commands = Vec::new();
    let mut next = 0;
    let subject = PreviewSubject::Operation {
        op: Box::new(Operation::Rebase(RebaseParams::new(
            Tip {
                name: "refs/heads/feature".to_owned(),
                oid: oid(3),
            },
            Source::Branch("main".to_owned()),
            oid(2),
        ))),
        opened_by: "r",
    };
    assert!(
        open_preview(state, &mut commands, subject, false, &mut || {
            next += 1;
            JobId::new(next)
        }),
        "nothing is running, so the preview opens"
    );
    commands
        .into_iter()
        .filter_map(|command| match command {
            Command::PlanPreview(_, request) => Some(request.resident),
            _ => None,
        })
        .collect()
}

/// ease-pass 06r m1: a Ghost Preview whose slice holds a folded merge (9)
/// and a merge in `1` (7) draws them `◉`, as the graph under it does; an
/// older stash entry (10) stays plain. The plan job still gets ids only.
#[test]
fn a_folded_merge_in_a_preview_draws_as_a_merge() {
    let mut state = every_node(8);
    let window: HashSet<Oid> = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| window.rows.iter().map(|row| row.commit.id).collect())
        .expect("a window");
    let asked = open_a_rebase(&mut state);
    assert_eq!(
        asked,
        std::slice::from_ref(&window),
        "the job gets the slice's ids"
    );
    let drawn = rebase_planned(&mut state);
    assert!(
        drawn.len() > window.len(),
        "the picture, its ghost included, is what the graph draws: {drawn:?}"
    );
    let drawn: HashMap<Oid, NodeKind> = drawn.into_iter().collect();
    for (id, kind) in [
        (9, NodeKind::Merge),
        (8, NodeKind::Merge),
        (7, NodeKind::Merge),
        (10, NodeKind::Commit),
        (5, NodeKind::Commit),
    ] {
        assert_eq!(drawn.get(&oid(id)), Some(&kind), "commit {id}");
    }
}

/// ease-pass 06c: a Ghost Preview is laid out from the rows the walk sent,
/// never from the view's fold. The same rows under a view with every merge
/// folded and one with every merge opened give the same slice and the same
/// picture.
#[test]
fn a_preview_slice_is_the_same_with_every_merge_folded_or_opened() {
    let folded_view = crate::history::View::folded();
    let mut opened_view = folded_view.clone();
    let mut merges = 0;
    let rows = every_node(8)
        .history
        .and_then(|h| h.window)
        .expect("a window")
        .rows;
    for row in rows.iter().filter(|row| row.commit.parents.len() > 1) {
        opened_view = open_merge(&opened_view, row.commit.id).expect("each opens once");
        merges += 1;
    }
    assert!(merges >= 2, "the history has merges to open: {merges}");
    let previewed = |view: &crate::history::View| {
        let mut state = every_node(8);
        state.history.as_mut().expect("a walk").view = view.clone();
        let asked = open_a_rebase(&mut state);
        (asked, rebase_planned(&mut state))
    };

    let (folded, opened) = (previewed(&folded_view), previewed(&opened_view));

    assert!(!folded.1.is_empty(), "a picture is drawn");
    assert_eq!(folded, opened);
}

/// `a_plan`'s one ghost, replaying 3 onto 2 of [`every_node`]'s history,
/// handed to the preview `open_a_rebase` opened: the Unicode tier's node
/// per drawn commit, top to bottom.
fn rebase_planned(state: &mut AppState) -> Vec<(Oid, NodeKind)> {
    let mut plan = a_plan();
    plan.ghosts[0].parents = vec![Target::Real(oid(2))];
    plan.ghosts[0].replays = Some(oid(3));
    plan.ref_moves[0].from = Some(oid(3));
    plan.against.clear();
    let job = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        state,
        job,
        Ok(Arc::new(Planned {
            plan,
            anchors: Vec::new(),
            runs: None,
        })),
    );
    unicode(state)
}
