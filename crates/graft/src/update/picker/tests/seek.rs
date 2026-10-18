//! `Tab` on a tip the window has not loaded seeks it; on a merged branch's
//! tip, which the default fold hides, the merge opens and the cursor lands
//! on the tip (ease-pass 06c). Its own file so `picker/tests.rs` stays
//! under its size row.

use super::*;
use crate::update::tests::reveal::{find_for, on, revealed};

const WALK: JobId = JobId::new(4242);

#[test]
fn tab_on_an_unloaded_tip_seeks() {
    let mut state = on_row(&opened(&fixture()), "login-page");
    state.awaiting = Some(WALK);
    let (_, commands) = press(&state, KeyCode::Tab);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ShowRows(walk, _) if *walk == WALK)),
        "{commands:?}"
    );
}

/// `login-page` was merged into `main` at 0x02: its tip 0x09 sits behind
/// that merge, which the default fold keeps closed.
#[test]
fn tab_on_a_merged_branchs_tip_opens_its_merge_and_lands_on_it() {
    let mut state = on_row(&opened(&fixture()), "login-page");
    state.awaiting = Some(WALK);
    let (state, commands) = press(&state, KeyCode::Tab);
    let find = find_for(&commands, oid(9));

    let (merge, tip) = (oid(2), oid(9));
    let (state, _) = revealed(
        &state,
        WALK,
        find,
        (merge, tip),
        (&[3, 2, 1], &[3, 2, 9, 1]),
    );

    assert_eq!(on(&state), Some(tip), "the cursor on login-page's tip");
    assert!(state.picker.is_none(), "Tab closed the picker");
}
