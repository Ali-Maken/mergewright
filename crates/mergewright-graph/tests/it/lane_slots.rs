//! Lane-colour probe for pre-publish-cleanup/12 (see `lane_slot_probe`).

use std::collections::HashMap;
use std::path::Path;

use crate::corpus::{for_each_shape, open_with_seeds, walk_everything};
use mergewright_fixtures::Shape;
use mergewright_graph::layout::layout;

/// Palette size `mergewright-ui` draws lanes with (`theme::LANE_COUNT`).
/// `graph` cannot depend on `ui`, so the probe repeats it.
const PALETTE_LEN: usize = 12;

/// One-off probe (pre-publish-cleanup/12): for every corpus shape, each row
/// keyed by (subject, nth occurrence) and the palette slot of every lane it
/// draws. Run before and after a fixture-identity change and diff the two
/// files: a changed slot is a changed lane colour. Writes to the path in
/// `MWR_LANE_SLOTS_OUT`; run with `--run-ignored only`.
#[test]
#[ignore = "probe: writes the file named by MWR_LANE_SLOTS_OUT"]
fn lane_slot_probe() {
    let out = std::env::var("MWR_LANE_SLOTS_OUT").expect("set MWR_LANE_SLOTS_OUT");
    let mut text = String::new();
    let mut visit = |shape: Shape, dir: &Path| {
        let (handle, seeds) = open_with_seeds(shape, dir);
        let (commits, _) = walk_everything(&handle, shape);
        let rows = layout(&commits, seeds);
        let mut seen: HashMap<Vec<u8>, usize> = HashMap::new();
        for row in &rows {
            let subject = row.commit.subject().to_vec();
            let nth = seen.entry(subject.clone()).or_insert(0);
            *nth += 1;
            let slots: Vec<String> = row
                .lanes
                .iter()
                .enumerate()
                .filter_map(|(column, lane)| {
                    lane.map(|identity| format!("{column}:{}", identity.slot(PALETTE_LEN)))
                })
                .collect();
            text.push_str(&format!(
                "{shape}\t{}#{nth}\t{}\n",
                String::from_utf8_lossy(&subject),
                slots.join(",")
            ));
        }
    };
    for_each_shape(&mut visit);
    visit(
        Shape::ManyLanes,
        &mergewright_fixtures::borrow(Shape::ManyLanes),
    );
    std::fs::write(&out, text).unwrap_or_else(|why| panic!("writing {out}: {why}"));
}
