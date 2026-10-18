//! `include_of` over every way in, amend flag and touch (08s M2).

use graft_ops::operation::{Chosen, FileChoice, Include, OutWhy, Part};

use super::*;

fn choice(path: &str, goes_in: bool, part: Part) -> FileChoice {
    FileChoice {
        path: path.into(),
        goes_in,
        part,
        why_out: (!goes_in).then_some(OutWhy::NotStaged),
    }
}

/// Every way in × amend × touched: the index as it is only for `a`, still
/// amending, untouched; the rows going in every other time (08s M2).
#[test]
fn include_of_table() {
    let rows = [
        choice("a.rs", true, Part::StagedPart),
        choice("b.rs", false, Part::Whole),
    ];
    let shown = Include::Chosen(vec![Chosen {
        path: "a.rs".into(),
        part: Part::StagedPart,
    }]);
    // (way in, amend on, rows touched, the index as it is)
    let table = [
        (WayIn::Commit, false, false, false),
        (WayIn::Commit, false, true, false),
        (WayIn::Commit, true, false, false),
        (WayIn::Commit, true, true, false),
        (WayIn::Amend, false, false, false),
        (WayIn::Amend, false, true, false),
        (WayIn::Amend, true, false, true),
        (WayIn::Amend, true, true, false),
    ];
    for (way_in, amend, touched, index) in table {
        let expected = if index {
            Include::Staged
        } else {
            shown.clone()
        };
        assert_eq!(
            include_of(&rows, way_in, amend, touched),
            expected,
            "{way_in:?} amend {amend} touched {touched}"
        );
    }
}
