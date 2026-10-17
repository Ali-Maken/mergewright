//! A folded merge is still a merge to everything but the drawing
//! (ease-pass 06 D1): the evil probe, the menu's revert, the detail
//! pane. A stash entry is not one.

use std::collections::HashMap;

use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;

use super::*;

/// [`merge_on_screen`] with the merge folded: it draws its first parent
/// only, as the fold (or the first-parent view) trims it; the repository
/// still has both.
fn folded_merge_on_screen(id: JobId, cursor: usize) -> (AppState, Oid) {
    let (mut state, merge) = merge_on_screen(id, cursor);
    let window = state
        .history
        .as_mut()
        .and_then(|history| history.window.as_mut())
        .expect("a window");
    let mut rows = window.rows.to_vec();
    rows[1].drawn_parents.truncate(1);
    assert_eq!(rows[1].commit.parents.len(), 2, "the commit keeps both");
    window.rows = rows.into();
    (state, merge)
}

/// The refs: `HEAD` on `main` at `tip`, and `stash` as `stash@{0}`.
fn refs_at(state: &mut AppState, tip: Oid, stash: Option<Oid>) {
    let main = RefName::new("refs/heads/main").expect("a valid name");
    let stashes: Vec<(Oid, usize)> = stash.into_iter().map(|entry| (entry, 0)).collect();
    let refs = crate::refs::badges(
        &[(main.clone(), tip)],
        &HeadState::Branch(main),
        &HashMap::<RefName, RefName>::new(),
        &stashes,
        |_, _| unreachable!("no upstreams to count"),
    )
    .expect("nothing to count");
    state.refs = Some(Arc::new(refs));
}

fn plain(state: &AppState) -> Oid {
    state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| window.rows[0].commit.id)
        .expect("row 0")
}

fn evil_probed(commands: &[Command], merge: Oid) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, Command::EvilProbe(_, oid) if *oid == merge))
}

#[test]
fn a_folded_merge_is_probed_as_a_merge() {
    let (state, merge) = folded_merge_on_screen(JobId::new(61), 0);
    let (_, commands) = update(&state, key(KeyCode::Char('j')));
    assert!(evil_probed(&commands, merge), "{commands:?}");
}

#[test]
fn a_stash_entry_is_not_probed_as_a_merge() {
    let (mut state, merge) = merge_on_screen(JobId::new(62), 0);
    let tip = plain(&state);
    refs_at(&mut state, tip, Some(merge));
    let (_, commands) = update(&state, key(KeyCode::Char('j')));
    assert!(!evil_probed(&commands, merge), "{commands:?}");
}

/// The revert the menu offers on a folded merge says `-m 1`: without it,
/// git refuses ("is a merge but no -m option was given").
#[test]
fn a_folded_merge_menu_reverts_with_a_mainline() {
    let (mut state, _) = folded_merge_on_screen(JobId::new(63), 1);
    let tip = plain(&state);
    refs_at(&mut state, tip, None);
    crate::update::context_menu::open_for_history_row(&mut state, 1);
    let open = state.context_menu.expect("the menu opened");
    let labels: Vec<&str> = open
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(
        labels
            .iter()
            .any(|label| label.starts_with("Revert") && label.ends_with("-m 1")),
        "{labels:?}"
    );
}

#[test]
fn a_folded_merge_shows_both_parents_in_the_detail_pane() {
    let (mut state, merge) = folded_merge_on_screen(JobId::new(64), 1);
    state.detail.spec = Some(DetailSpec::Commit(merge));
    let view = crate::view::detail_view(&state);
    assert_eq!(view.header.parents.len(), 2);
}

/// The radar treats it as a merge: the evil-merge line is offered.
#[test]
fn a_folded_merge_gets_the_evil_merge_radar_line() {
    let (mut state, merge) = folded_merge_on_screen(JobId::new(66), 1);
    state.detail.spec = Some(DetailSpec::Commit(merge));
    let view = crate::view::detail_view(&state);
    assert!(
        view.header
            .radar
            .iter()
            .any(|line| matches!(line, graft_ui::detail::RadarLine::EvilUnchecked)),
        "{:?}",
        view.header.radar
    );
}

/// `o` on a folded merge is refused as a merge, not offered as a pick.
#[test]
fn o_on_a_folded_merge_is_refused_as_a_merge() {
    let (mut state, merge) = folded_merge_on_screen(JobId::new(67), 1);
    let tip = plain(&state);
    refs_at(&mut state, tip, None);
    let (next, commands) = update(&state, key(KeyCode::Char('o')));
    assert!(next.preview.is_none(), "{commands:?}");
    let short = merge.to_string()[..7].to_owned();
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. }
            if reason.starts_with(&format!("{short} is a merge"))),
        "{:?}",
        next.status
    );
}

/// A stash entry keeps today's output: its first parent only.
#[test]
fn a_stash_entry_shows_its_first_parent_in_the_detail_pane() {
    let (mut state, merge) = merge_on_screen(JobId::new(65), 1);
    let tip = plain(&state);
    refs_at(&mut state, tip, Some(merge));
    state.detail.spec = Some(DetailSpec::Commit(merge));
    let view = crate::view::detail_view(&state);
    assert_eq!(view.header.parents.len(), 1);
}
