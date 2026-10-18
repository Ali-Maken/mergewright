use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_ops::operation::{Operation, RebaseParams, Source, Tip};
use graft_ops::preview::Target;
use graft_ui::graph_column::NodeKind;

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

    // `a_plan`'s one ghost, replaying 3 onto 2 of this history.
    let mut plan = a_plan();
    plan.ghosts[0].parents = vec![Target::Real(oid(2))];
    plan.ghosts[0].replays = Some(oid(3));
    plan.ref_moves[0].from = Some(oid(3));
    plan.against.clear();
    let job = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        &mut state,
        job,
        Ok(Arc::new(Planned {
            plan,
            anchors: Vec::new(),
            runs: None,
        })),
    );
    let drawn: HashMap<Oid, NodeKind> = unicode(&state).into_iter().collect();
    assert!(
        drawn.len() > window.len(),
        "the picture, its ghost included, is what the graph draws: {drawn:?}"
    );
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
