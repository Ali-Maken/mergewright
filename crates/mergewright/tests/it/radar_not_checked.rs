//! The radar's context failed to read (m7-power-tools/33): the detail pane
//! says the radar did not run, and why, instead of drawing "no findings".
//! Real `git`, the real job pool, the real `view::view`; the failure itself is
//! set on the state, since a real one needs a shallow clone (20f).

use mergewright::update::detail::{DetailSpec, Loading};
use mergewright_core::event::KeyCode;

use crate::app_harness::App;
use crate::preview::{FORK_NOW, dated_commit, opened};

/// The first radar line the pane draws while the context is failed.
const NOT_CHECKED: &str = "radar: not checked \u{2014} commit b32274b is not in the repository \u{b7} ctrl+r to try again";

/// `docs/shots/m7/radar-not-checked-120x40.txt`. What right looks like: the
/// detail pane's radar block starts `radar: not checked — commit b32274b is
/// not in the repository · ctrl+r to try again`; the graph's marker column
/// shows no marker. Colour unverified (rule 5).
#[test]
fn capture_radar_not_checked() {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    dated_commit(repo, "notes.txt", "a\n", "add notes", 1);
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, 40);
    let state = opened(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the pane on a commit, the radar answered", |state| {
        matches!(state.detail.spec, Some(DetailSpec::Commit(_)))
            && matches!(state.radar.context, Loading::Ready(_))
    });
    let mut state = crate::captures::settled_frame(&app, state);
    state.radar.context = Loading::Failed("commit b32274b is not in the repository".to_owned());
    let frame = crate::captures::frame_at(
        &state,
        120,
        40,
        Some(mergewright::session::Clock::Fixed(FORK_NOW)),
    );
    let lines: Vec<&String> = frame.iter().filter(|row| row.contains("radar:")).collect();
    assert_eq!(lines.len(), 1, "{}", frame.join("\n"));
    assert!(lines[0].contains(NOT_CHECKED), "{}", lines[0]);
    let body = format!("{}\n", frame.join("\n"));
    crate::captures::check_shot("m7/radar-not-checked-120x40.txt", &body);
    drop(state);
    app.close();
}

/// The evil-merge line in the pane (mergewright-ui `detail_pane::header`).
const EVIL_LINE: &str = "\u{b7} ctrl+r to try again \u{b7} evil merge";

/// `docs/shots/m7/radar-not-checked-evil-120x40.txt` (m7-power-tools/33c).
/// What right looks like: the detail pane's radar block starts `radar: not
/// checked — commit b32274b is not in the repository · ctrl+r to try again`,
/// then, after ` · ` on the same row (the pane joins its radar lines), `evil
/// merge`, and the graph's marker column
/// shows the evil marker on the merge's row only. Colour unverified (rule 5).
#[test]
fn capture_radar_not_checked_with_an_evil_merge() {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::EvilMerge);
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, 40);
    let state = opened(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(
        state,
        "the merge probed evil, the radar answered",
        |state| {
            matches!(
                state.detail.evil,
                Loading::Ready(mergewright_ops::mistake_radar::probes::EvilVerdict::Evil { .. })
            ) && matches!(state.radar.context, Loading::Ready(_))
        },
    );
    let mut state = crate::captures::settled_frame(&app, state);
    state.radar.context = Loading::Failed("commit b32274b is not in the repository".to_owned());
    let frame = crate::captures::frame_at(
        &state,
        120,
        40,
        Some(mergewright::session::Clock::Fixed(FORK_NOW)),
    );
    let shot = frame.join("\n");
    // The pane joins its radar lines with ` · `: not-checked first, then
    // the evil finding.
    let radar: Vec<&String> = frame.iter().filter(|row| row.contains("radar:")).collect();
    assert_eq!(radar.len(), 1, "{shot}");
    assert!(radar[0].contains(EVIL_LINE), "{shot}");
    // The marker column: `⚠` on the merge's row and on no other.
    let marked: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains('\u{26a0}'))
        .collect();
    assert_eq!(marked.len(), 1, "{shot}");
    assert!(marked[0].contains("Merge branch 'alpha'"), "{shot}");
    let body = format!("{}\n", frame.join("\n"));
    crate::captures::check_shot("m7/radar-not-checked-evil-120x40.txt", &body);
    drop(state);
    app.close();
}
