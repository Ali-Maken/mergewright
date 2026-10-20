//! The lane-identity property: a lane stays in its column for its whole
//! life, and keeps its identity until it reaches another branch's tip
//! (ADR-0064). Split from `properties.rs` to keep that file under its row.

use proptest::prelude::*;

use super::*;
use crate::layout::tests::extend_rows;

/// What one row may do to one column of open lanes: nothing, unless the lane
/// there was waiting for this very commit — in which case it closes, or (on
/// the row's own lane only) continues in place with its identity intact.
fn check_column(
    column: usize,
    before: Option<&Lane>,
    after: Option<&Lane>,
    row: &Row,
    handles: &Handles,
    seeds: &Seeds,
) -> Result<(), TestCaseError> {
    let Some(previous) = before else {
        return Ok(());
    };
    if previous.waiting_for != handles.handle_of(&row.commit.id).expect("interned") {
        prop_assert_eq!(
            after,
            Some(previous),
            "row {} touched column {}, which was waiting for {:?}",
            row.commit.id,
            column,
            previous.waiting_for
        );
        return Ok(());
    }
    let Some(now) = after else {
        return Ok(());
    };
    prop_assert_eq!(column, row.lane, "only the row's own lane continues");
    // ADR-0064: a lane keeps its identity until it reaches another branch's
    // tip, where it takes that branch's name (`renamed_at_tip`).
    let tip = seeds
        .known_branches
        .iter()
        .find(|seed| seed.tip == row.commit.id);
    let expected =
        crate::layout::assign_row::renamed_at_tip(previous, tip).unwrap_or(previous.identity);
    prop_assert_eq!(
        now.identity,
        expected,
        "a continuing lane keeps its identity, bar a rename at a tip"
    );
    prop_assert_eq!(now.reserved, previous.reserved);
    prop_assert_eq!(
        Some(now.waiting_for),
        row.commit
            .parents
            .first()
            .map(|&parent| handles.handle_of(&parent).expect("interned"))
    );
    Ok(())
}

/// A lane never changes column mid-life: a lane not connected to the row
/// being laid out is untouched by it, and a lane that is connected either
/// closes or continues in place with its identity intact.
pub(super) fn check_lanes_stay_put(commits: &[Commit], seeds: &Seeds) -> Result<(), TestCaseError> {
    let handles = Handles::of(commits);
    let mut cursor = LayoutCursor::new(seeds.clone(), &handles);
    let mut order = EmittedOrder::new();
    for commit in commits {
        let before = cursor.active_lanes().to_vec();
        let row = extend_rows(
            &mut cursor,
            std::slice::from_ref(commit),
            commits,
            &handles,
            &mut order,
        )
        .remove(0);
        let after = cursor.active_lanes();

        for (column, lane) in after.iter().enumerate() {
            if let Some(lane) = lane {
                prop_assert_eq!(lane.index, column, "a lane's index is its column");
            }
        }
        for (column, previous) in before.iter().enumerate() {
            check_column(
                column,
                previous.as_ref(),
                after[column].as_ref(),
                &row,
                &handles,
                seeds,
            )?;
        }
    }
    Ok(())
}
