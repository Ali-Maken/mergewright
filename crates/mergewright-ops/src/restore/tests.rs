use mergewright_core::journal::HeadRef;

use super::refresh_after;
use crate::apply::Refresh;

fn branch(name: &str) -> HeadRef {
    HeadRef::Branch(format!("refs/heads/{name}"))
}

/// The scope rule as a table (m6-conflicts/79): any ref, a `SetHead`
/// step, a recorded `HEAD` move, or an unread `HEAD` re-reads the graph;
/// only a restore that provably left refs and `HEAD` alone does not.
#[test]
fn refresh_covers_head_whenever_the_restore_moves_it() {
    let main = branch("main");
    let feature = branch("feature");
    let rows: [(bool, bool, Option<&HeadRef>, Refresh, &str); 6] = [
        (
            false,
            false,
            Some(&main),
            Refresh::WorkingTree,
            "nothing moved",
        ),
        (true, false, Some(&main), Refresh::Everything, "a ref moved"),
        (
            false,
            true,
            Some(&main),
            Refresh::Everything,
            "the plan sets HEAD",
        ),
        (
            false,
            false,
            Some(&feature),
            Refresh::Everything,
            "HEAD recorded moved",
        ),
        (false, false, None, Refresh::Everything, "HEAD after unread"),
        (true, true, Some(&feature), Refresh::Everything, "all of it"),
    ];
    for (refs, sets_head, after, want, why) in rows {
        assert_eq!(refresh_after(refs, sets_head, &main, after), want, "{why}");
    }
}
