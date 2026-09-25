//! The footer's other rungs under a modal (m6-conflicts/78, Minor 3 and
//! Minor 6b): the commit-graph offer and the search line name keys the
//! modal swallows, so they are blanked while it owns the row.

use graft_git::version::{GitVersion, SystemGit};

use super::*;
use crate::update::{CommitGraphOffer, GitCheck};

const OFFER: &str = "B build one";

fn offered(mut state: AppState) -> AppState {
    state.commit_graph = CommitGraphOffer::Offered;
    state.system_git = Some(GitCheck::Ran(SystemGit::Supported(GitVersion::MINIMUM)));
    state
}

/// The failing test first: `no commit-graph file · B build one · X never
/// here` stood above the lease box's keys and over the credential prompt,
/// where `B` is swallowed or typed into the password.
#[test]
fn the_commit_graph_offer_hides_under_a_box() {
    for (columns, rows) in SIZES {
        let open = offered(rejected_push(columns, rows));
        let screen = draw(&open, columns, rows);
        assert!(
            screen.contains(OFFER),
            "{columns}x{rows}: offered\n{screen}"
        );

        let lease = press(&open, KeyCode::Char('P'));
        assert!(lease.confirm.is_some(), "P opened the lease box");
        let screen = draw(&lease, columns, rows);
        assert!(
            !screen.contains(OFFER),
            "{columns}x{rows}: lease box\n{screen}"
        );

        let mut credential = offered(credential_state(
            columns,
            rows,
            "Password for 'http://user@127.0.0.1:8443': ",
        ));
        credential.detail.shown = false;
        let screen = draw(&credential, columns, rows);
        assert!(
            !screen.contains(OFFER),
            "{columns}x{rows}: credential\n{screen}"
        );

        let closed = press(&lease, KeyCode::Escape);
        let screen = draw(&closed, columns, rows);
        assert!(
            screen.contains(OFFER),
            "{columns}x{rows}: back after esc\n{screen}"
        );
    }
}

/// Minor 6b: a search standing on the graph, then a Ghost Preview opened
/// over it. The preview's own row is blank, and the search line with it —
/// the one modal where it would show, since a box's bar covers the footer.
#[test]
fn the_search_rung_is_blank_under_the_preview() {
    for (columns, rows) in SIZES {
        let mut state = changes_pane_state(columns, rows);
        state.detail.shown = false;
        let query = crate::search::Query::parse("Commit").expect("a non-empty query");
        let mut search = crate::search::Search::new(query, 0, (0, 0));
        search.state = crate::search::SearchState::Done { hits: 2 };
        let line = search.line();
        state.search = Some(search);
        assert!(
            crate::view::graph_view(&state, 0).search.is_some(),
            "{columns}x{rows}: the search rung stands"
        );

        let (_, mut preview) = modals(columns, rows)
            .into_iter()
            .find(|(modal, _)| *modal == "preview")
            .expect("the preview is one of the modals");
        preview.search = state.search.clone();
        assert!(
            crate::view::graph_view(&preview, 0).search.is_none(),
            "{columns}x{rows}: blank under the preview"
        );
        let screen = draw(&preview, columns, rows);
        assert!(
            !screen.contains(&line),
            "{columns}x{rows}: {line}\n{screen}"
        );
        assert_eq!(bottom_row(&preview, columns, rows), "", "{columns}x{rows}");
    }
}
