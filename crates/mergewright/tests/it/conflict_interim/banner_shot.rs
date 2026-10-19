//! The banner captures for `docs/shots/` (moved out of `conflict_interim.rs`
//! for the file-size ratchet).

use super::*;

/// Writes the banner captures for `docs/shots/`.
///
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it conflict_interim::banner_shot::capture_banner_for_docs_shots`.
#[test]
fn capture_banner_for_docs_shots() {
    let branch_pane_settled = mergewright::update::branch_pane::settled;
    // Enough behind the frame that the banner's two rows land on content:
    // 30 commits of ordinary work for the graph and 16 conflicted files for
    // the changes pane. The capture is taken to show what those rows cost,
    // and it cannot show a cost against blank space (m5/20 item 6).
    let conflicts = 16;
    let scratch = about_to_conflict_over(conflicts - 2, 30);
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    // The branch pane's read draws `not merged` on a row (ease-pass 09b):
    // a frame taken before it lands differs from the next run's.
    let state = app.pump(state, "the branch pane's read", branch_pane_settled);

    // Two screens, not one. Under 100 columns the tab strip shows one side
    // at a time, and the narrow capture stands on the **graph**: the
    // changes pane's file list is capped at 40% of the pane, so the Detail
    // tab at 24 rows is six rows of list over a body showing one file's
    // conflict — a fine screen, but not one that can show what the banner's
    // rows cost, which is what item 6 was filed about. The graph fills.
    //
    // Cloned before the pane is opened, because the banner keeps the
    // keyboard through all of it (`detail.focus` is what the tab strip
    // reads, and it is still `Graph` here).
    let on_the_graph = state.clone();
    // The focused banner is modal: `Tab` out of it first, or the `k`s that
    // walk to the changes row are the banner's to swallow.
    let state = app.key(&state, KeyCode::Tab);
    let state = open_changes_pane(&app, state);
    let state = tab_back_to_the_banner(&app, state);
    let state = app.pump(state, "the branch pane's read", branch_pane_settled);
    let home = repo.display().to_string();

    for (label, columns, rows, graph_tab) in [
        ("80x24", 80u16, 24u16, true),
        ("120x40", 120, 40, false),
        ("160x48", 160, 48, false),
    ] {
        let shot = if graph_tab { &on_the_graph } else { &state };
        // The path is shortened **before** drawing, not after. The scratch
        // repository lives under a ~50-character temporary path, and the
        // header lays itself out against it: replacing the text afterwards
        // produced a capture that showed `/code/api` beside a banner cut to
        // `MERG…`, and the reader had no way to know the width had been
        // spent on a path they could not see. Shortening the state makes
        // the capture the frame a user with `~/code/api` actually gets
        // (m5/16). Done to a copy, per shot, so the state the app is still
        // driving keeps the path git is reachable by.
        let mut shot = shot.clone();
        if let Some(facts) = shot.repository.as_mut() {
            facts.path = std::path::PathBuf::from("/code/api");
        }
        // Resized before drawing: the layout, the graph's row budget and
        // the hint bar's room all come off `state.size`, so a state sized
        // for one screen photographed at another is a picture of neither.
        let sized = app.resize(&shot, columns, rows);
        let drawn = frame_at(&sized, columns, rows, Some(Clock::Fixed(CONFLICT_NOW)));
        let body = drawn.join("\n");
        assert!(
            body.contains("merging "),
            "every banner capture is of a stopped merge: {label}"
        );
        if graph_tab {
            assert!(
                drawn.iter().all(|line| !line.trim().is_empty()),
                "a frame with blank rows in it cannot show what a row costs \
                 (m5/20 item 6): {label}\n{body}"
            );
            assert!(
                drawn[0].contains("/code/api") && drawn[0].contains("merging "),
                "on the graph tab the banner shares the header row: {label}\n{body}"
            );
        } else {
            assert!(
                body.contains(&format!("CONFLICTS ({conflicts})")),
                "the wide captures show the changes pane's own count: {label}"
            );
            assert!(
                !body.contains("reading\u{2026}"),
                "the shutter waits for the pane to settle (m5/20 item 5): {label}"
            );
        }
        let mut out = capture_header(label, columns, rows, conflicts, graph_tab);
        for line in &drawn {
            out.push_str(&line.replace(&home, "/code/api"));
            out.push('\n');
        }
        check_shot(&format!("m5-ui-banner-conflict-{label}.txt"), &out);
    }
    drop(state);
    drop(on_the_graph);
    app.close();
}
