//! `update` tests: resolver screens. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// `state` in the operation whose sides go by `roles`: stash pop is the
/// resolver open with no operation at all (`update::resolver::side_names`).
/// [`screen_for`]'s resolver: open on a two-block conflict, where every live
/// key in `resolver_list` has something to do; its whole-file card; or the
/// same document now done, read-only (m6-conflicts/25).
fn resolver_screen_for(
    walk: JobId,
    whole_file: bool,
    done: bool,
    roles: graft_ui::conflict_resolver::SideRoles,
) -> AppState {
    if whole_file {
        return in_roles(resolver_card_on_screen(walk), roles);
    }
    let mut state = in_roles(resolver_on_screen(walk), roles);
    if done {
        let open = state.resolver.as_mut().expect("open");
        open.done = Some(resolver::DoneOpen {
            shape: resolver::DoneShape::Text { unstaged: false },
            start_over: crate::jobs::conflict::StartOver::Can {
                kind: graft_ops::operation::RestartKind::Markers,
            },
        });
        open.cursor_block = None;
    }
    state
}

/// [`screen_for`]'s resolver screen for a `Resolver` context.
pub(super) fn resolver_for(walk: JobId, context: &HintContext) -> AppState {
    let HintContext::Resolver {
        whole_file,
        done,
        roles,
        finishable,
        ..
    } = *context
    else {
        panic!("not a resolver context: {context:?}");
    };
    let mut state = resolver_screen_for(walk, whole_file, done, roles);
    // Only a done file is ever finishable in the app (see
    // `classified_form`): a document or a card open is a file still
    // conflicted, where `c` refuses in words — still an answer — and the
    // fixture keeps the second file `j`/`n` step to.
    if finishable && done {
        every_file_staged(&mut state);
    }
    state
}

/// Nothing unmerged any more: every path the fixture listed is staged, so
/// `c` acts (m6-conflicts/31).
fn every_file_staged(state: &mut AppState) {
    let entries = state
        .working_tree
        .as_ref()
        .expect("the resolver fixtures list their files")
        .entries
        .iter()
        .map(|entry| status_entry(&entry.path.display().to_string(), false))
        .collect();
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus { entries }));
}

fn in_roles(mut state: AppState, roles: graft_ui::conflict_resolver::SideRoles) -> AppState {
    use graft_git::state::InProgress;
    use graft_ui::conflict_resolver::SideRoles;
    let kind = match roles {
        SideRoles::Merge => Some(InProgress::Merge),
        SideRoles::Rebase => Some(InProgress::Rebase),
        SideRoles::CherryPick => Some(InProgress::CherryPick),
        SideRoles::Revert => Some(InProgress::Revert),
        SideRoles::StashPop | SideRoles::SetAside => None,
    };
    state.in_progress = kind.map(|kind| crate::update::resolver::fixtures::operation(kind, None));
    // A set-aside's clash is the resolver with no operation and a recorded
    // clash whose `why` is `SetAside` (m6-conflicts/74).
    if roles == SideRoles::SetAside {
        state.clash = Some(crate::update::state::ClashState {
            clash: graft_ops::clash::StashClash {
                entry: graft_core::journal::EntryId::new(7, [7; 10]),
                stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
                index: Some(0),
                why: graft_core::journal::PutBackWhy::SetAside,
            },
            origin: crate::update::LiveOrigin::ThisRun,
        });
    }
    state
}

/// [`screen_for`]'s clash screen for `bar` (ease-pass/11): the two-block
/// document in `bar.roles`' operation, picked to where `bar.at` stands —
/// `Picked` its first clash taken, `FileSolved` both (`later.rs`, unread,
/// is next), `AllSolved` both and `later.rs` solved too.
pub(super) fn clash_for(walk: JobId, bar: &graft_ui::graph_screen::hints::ClashBar) -> AppState {
    use graft_git::conflict::Take;
    use graft_ui::graph_screen::hints::ClashAt;
    let mut state = resolver_screen_for(walk, false, false, bar.roles);
    let open = state.resolver.as_mut().expect("open");
    let file = Arc::make_mut(open.file.as_mut().expect("a document"));
    match bar.at {
        ClashAt::Open => {}
        ClashAt::Picked => file.take(0, Take::Ours).expect("block 0"),
        ClashAt::FileSolved { .. } => {
            file.take_all(Take::Ours);
        }
        ClashAt::AllSolved => {
            file.take_all(Take::Ours);
            open.seen
                .insert("later.rs".into(), resolver::RowState::AllTaken);
        }
    }
    state
}
