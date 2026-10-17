use super::{LaneRole, Route, made_on};
use crate::identity::BranchIdentity;
use crate::layout::tests::commit;
use crate::row::Row;

fn id(name: &str) -> BranchIdentity {
    BranchIdentity::of_name(name.as_bytes())
}

/// `main ← wallet-release ← sara/fix-toc`, cursor on `sara/fix-toc`.
#[test]
fn role_table() {
    let main = id("main");
    let wallet = id("wallet-release");
    let sara = id("sara/fix-toc");
    let stranger = id("ops/ci");

    let route = Route::new(main, &[wallet], Some(sara));
    let cases = [
        (sara, LaneRole::This),
        (main, LaneRole::Trunk),
        (wallet, LaneRole::Under),
        (stranger, LaneRole::Other),
    ];
    for (branch, want) in cases {
        assert_eq!(route.role(branch), want, "{branch:?}");
    }
    assert_eq!(route.focus(), Some(sara));
    assert_eq!(route.trunk(), main);
    assert_eq!(route.chain(), &[wallet]);
}

/// On the trunk itself, the trunk is the focused branch: `This`, not
/// `Trunk`.
#[test]
fn focus_on_the_trunk_is_this() {
    let main = id("main");
    let route = Route::new(main, &[], Some(main));
    assert_eq!(route.role(main), LaneRole::This);
    assert_eq!(route.role(id("ops/ci")), LaneRole::Other);
}

/// No focus (the cursor row's lane is unnamed): only the trunk stands out.
#[test]
fn no_focus() {
    let main = id("main");
    let wallet = id("wallet-release");
    let route = Route::unread(main, None);
    assert_eq!(route.focus(), None);
    assert!(route.chain().is_empty());
    assert_eq!(route.role(main), LaneRole::Trunk);
    assert_eq!(route.role(wallet), LaneRole::Other);

    let read = Route::new(main, &[wallet], None);
    assert_eq!(read.role(wallet), LaneRole::Under);
}

/// Before the route is read, a base is still `Other`; the focus and trunk
/// already have their roles.
#[test]
fn unread_has_no_chain() {
    let main = id("main");
    let sara = id("sara/fix-toc");
    let route = Route::unread(main, Some(sara));
    assert_eq!(route.role(sara), LaneRole::This);
    assert_eq!(route.role(main), LaneRole::Trunk);
    assert_eq!(route.role(id("wallet-release")), LaneRole::Other);
}

/// A trunk passed in `built_on` stays `Trunk` and is not in the chain.
#[test]
fn trunk_in_built_on_is_dropped() {
    let main = id("main");
    let wallet = id("wallet-release");
    let route = Route::new(main, &[wallet, main], Some(id("sara/fix-toc")));
    assert_eq!(route.chain(), &[wallet]);
    assert_eq!(route.role(main), LaneRole::Trunk);
}

fn row(lane: usize, lanes: Vec<Option<BranchIdentity>>) -> Row {
    Row {
        commit: commit(1, &[], 0, b"one"),
        drawn_parents: Vec::new(),
        lane,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes,
        marker: None,
    }
}

#[test]
fn made_on_reads_the_rows_own_lane() {
    let main = id("main");
    let sara = id("sara/fix-toc");
    assert_eq!(made_on(&row(1, vec![Some(main), Some(sara)])), Some(sara));
    assert_eq!(made_on(&row(0, vec![Some(main), Some(sara)])), Some(main));
}

#[test]
fn made_on_an_unnamed_lane_is_none() {
    assert_eq!(made_on(&row(0, vec![None, Some(id("main"))])), None);
}

#[test]
fn made_on_out_of_range_is_none() {
    // Trailing `None`s are trimmed, so a lane past the end is legal input.
    assert_eq!(made_on(&row(2, vec![Some(id("main"))])), None);
    assert_eq!(made_on(&row(0, Vec::new())), None);
}
