//! `d` on a done file, in a merge Mergewright began: offered where ops can start
//! the file over, and answered in ops' words where it cannot
//! (m6-conflicts/28) — and the captures the user judges by eye.

use super::*;

mod mergewright_begun;
mod shell_begun;
use mergewright_begun::capture_can;
use shell_begun::capture_cannot;

/// `gone.txt` kept with `o`, the resolver on to `shared.rs`, then
/// `gone.txt` clicked open read-only.
fn gone_kept_and_opened(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('o'));
    let state = app.pump(state, "the keep, and shared.rs", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new("shared.rs") && open.job.is_none() && open.file.is_some()
            })
            && rows(state)
                .iter()
                .any(|(path, row)| path == "gone.txt" && *row == RowState::Done)
    });
    let state = done_open(app, &state, "gone.txt");
    // The status line (`✓ resolved gone.txt (ours)`) holds the bar's row
    // until the next key; `PgUp` at the top scrolls nowhere and gives it
    // back (`k` steps files since m6-conflicts/17).
    app.key(&state, KeyCode::PageUp)
}

/// `shared.rs` done ([`shared_done`]), then clicked open read-only.
fn shared_done_and_opened(app: &App, state: &AppState) -> AppState {
    let state = shared_done(app, state);
    let state = done_open(app, &state, "shared.rs");
    app.key(&state, KeyCode::PageUp)
}

/// What right looks like — can: `▸ shared.rs · done ✓ · d starts it over`
/// over the file, bar `d start over  e edit  esc close`. Cannot: `▸
/// gone.txt · done ✓` and nothing after it, bar `e edit  esc close`; after
/// `d`, the status row says why. At 200 columns the same identity row
/// beside the list column, one card.
#[test]
fn done_start_over_capture() {
    capture_can();
    capture_cannot();
}

/// `git ls-files -u -- <name>`: each stage's mode, oid and number.
fn stages(repo: &Path, name: &str) -> Vec<String> {
    crate::app_harness::git(repo, &["ls-files", "-u", "--", name])
        .lines()
        .map(|line| line.split('\t').next().unwrap_or(line).to_owned())
        .collect()
}
