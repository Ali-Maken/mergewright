//! The merge box's capture (`docs/shots/m6/fix-preview-verb-120x40.txt`),
//! apart from `preview.rs` so that file stays under its line count.

use std::path::Path;

use mergewright::session::Clock;

use super::{FORK_NOW, forked, frame_at, preview_opened_by};
use crate::app_harness::App;

/// `m` on `main`'s row: the merge box (ease-pass 10) has keys, not buttons,
/// and its footer names `y` once, no `m merge` alias. Regenerate on
/// purpose with `MWR_UPDATE_SHOTS=1`.
#[test]
fn capture_fix_preview_verb_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open_with_clock(
        scratch.path(),
        Clock::Fixed(mergewright_fixtures::PINNED_NOW),
    );
    let state = preview_opened_by(&app, state, 'm');
    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    app.close();
    let text = drawn.join("\n");
    assert!(text.contains("╰ y merge  esc cancel  ⏎ see the "), "{text}");
    assert!(!text.contains("[ Cancel ]"), "{text}");
    // Ease-pass 06u: the column carrying the merge's second parent down to
    // main's commit holds no commit, but both its ends are in view, so it
    // is a connector and draws as lines, not stubs, and nothing is left out.
    let row_of = |subject: &str| -> (&str, &str) {
        let line = drawn
            .iter()
            .find(|line| line.contains(subject))
            .unwrap_or_else(|| panic!("no row `{subject}`:\n{text}"));
        let at = line.find(subject).expect("found above");
        (line.as_str(), &line[..at])
    };
    let (merge, merge_graph) = row_of("[◆ feature] Merge branch 'main' into feature");
    let (lower, lower_graph) = row_of("[main] main moves again");
    assert!(merge_graph.contains('╮'), "merge row:\n{text}");
    assert!(lower_graph.contains('╯'), "main's row:\n{text}");
    for row in [merge, lower] {
        assert!(
            !row.contains('▾') && !row.contains('▴'),
            "a stub in `{row}`"
        );
    }
    assert!(!drawn[0].contains("lanes drawn"), "{}", drawn[0]);
    let mut out = String::from(
        "# fix-preview-verb-120x40.txt: 120x40, drawn by `view::view` from a real\n\
         # AppState over a real repository (`main` three commits, `feature` forked off\n\
         # the first with two of its own, `feature` checked out); cursor on `main`'s\n\
         # row, `m` pressed through `update`, the real plan landed\n\
         # (crates/mergewright/tests/it/preview/capture.rs, capture_fix_preview_verb_for_docs_shots).\n\
         # TestBackend character grid: colour ships unverified (CLAUDE.md rule 5).\n\
         #\n\
         # What right looks like:\n\
         #  - The merge box (ease-pass 10); its bottom border reads\n\
         #    `y merge  esc cancel  ⏎ see the N commits`.\n\
         #  - No buttons row, no `m merge` on the footer.\n\
         #  - The merge's second parent runs to main's row as a line (ease-pass\n\
         #    06u): `◎┈╮` on the merge, `●─│─╯` on `main moves again`, no\n\
         #    `▾`/`▴`, and no `lanes drawn` on the top line.\n\
         #\n\n",
    );
    for line in &drawn {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/m6/fix-preview-verb-120x40.txt");
    let checked = mergewright_fixtures::check(&path, &out);
    assert!(checked.is_ok(), "{}", checked.expect_err("it drifted"));
}
