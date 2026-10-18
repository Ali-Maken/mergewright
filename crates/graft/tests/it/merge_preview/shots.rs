//! The merge preview's captures, beside the merge mock
//! (`prototypes/ease-mocks/mergewright-merge-screen.html`): frames of the
//! real app over real git, for the user to judge by eye (rule 6). No colour
//! recorded (rule 5): the `You decide` line's clash colour is unverified.

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::KeyCode;

use super::{NOW, fix_toc, merge_preview, open, with_a_stray};

/// File name (width appended), what right looks like, width, height.
const SHOTS: [(&str, &str, u16, u16); 5] = [
    (
        "merge-preview",
        "`m` on sara/fix-toc from wallet-release: title `Merge sara/fix-toc into wallet-release`; `Nothing has changed yet.`; `Comes in 3 commits … (Sara, Omar)`, `Changes 3 files`, `Git combines 2 files on its own`, `You decide 1 file clashes: report.md (2 places)`; `After y you go straight to the first clash.`; footer `y merge  esc cancel  ⏎ see the 3 commits`; no buttons; the ghost merge row above the box.",
        120,
        40,
    ),
    (
        "merge-incoming",
        "`⏎` from the box: `The 3 commits that come in from sara/fix-toc`, newest first, `▸` on the first, who and age right-aligned; footer `esc back to the preview  y merge`.",
        120,
        40,
    ),
    (
        "merge-preview",
        "the same box at 90 columns: the lines whole where they fit, the footer's widest rung that fits.",
        90,
        30,
    ),
    (
        "merge-clean",
        "a merge with no clash: `You decide nothing: no file clashes`; `After y the merge is committed.`",
        120,
        40,
    ),
    (
        "merge-unrelated",
        "`m` on `stray`, which shares no history: `Comes in` stays; one line `Can't merge: stray and wallet-release share no history.`; no Changes/You decide/After lines; footer `esc cancel  ⏎ see the 1 commit` (no `y`).",
        120,
        40,
    ),
];

#[test]
fn capture_the_merge_preview() {
    let clash = fix_toc(true);
    let (app, state) = open(clash.path());
    let boxed = merge_preview(&app, state, "sara/fix-toc");
    let listed = app.key(&boxed, KeyCode::Enter);
    app.close();
    let clean = fix_toc(false);
    let (app, state) = open(clean.path());
    let clean_box = merge_preview(&app, state, "sara/fix-toc");
    app.close();
    let stray = with_a_stray();
    let (app, state) = open(stray.path());
    let unrelated = merge_preview(&app, state, "stray");
    app.close();
    let states: [&AppState; 5] = [&boxed, &listed, &boxed, &clean_box, &unrelated];

    let mut captures = graft_fixtures::Captures::new();
    for ((name, judge, columns, rows), state) in SHOTS.into_iter().zip(states) {
        let frame = crate::captures::frame_at(state, columns, rows, Some(Clock::Fixed(NOW)));
        let body = format!(
            "# TestBackend capture (not a pty), no colour recorded (rule 5).\n\
             # Fixture: capture_the_merge_preview in\n\
             # crates/graft/tests/it/merge_preview/shots.rs, real git.\n\
             # Judge by eye: {judge}\n\
             === {columns}x{rows} ===\n{}\n",
            frame.join("\n")
        );
        captures.check(
            format!(
                "{}/../../docs/shots/ease-pass/{name}-{columns}.txt",
                env!("CARGO_MANIFEST_DIR"),
            ),
            body,
        );
    }
    captures.finish();
}
