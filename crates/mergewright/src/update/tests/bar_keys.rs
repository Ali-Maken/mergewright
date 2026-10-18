//! `update` tests: bar keys. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// **Every key the bar draws, in any context, at any width, is answered by
/// the handler that has focus in that state** (m5/31). The invariant three
/// of this milestone's defects fell through, in one sweep.
///
/// It grew out of `every_hinted_key_is_bound`, whose narrower question —
/// is this `(key, verb)` in `keys::bound_actions`? — is still asked in the
/// same loop, because the table is what ties a key to the **verb** the hint
/// names: `M` is bound, but to the Map view, so a hint reading
/// `M mergetool` is as dead as one on an unbound key (m5-03 review,
/// Critical 2). Its earlier version carved out `m`/`r`/`b`/`t`/`s`/`a` and
/// compared keys only; there are no exemptions now, because a key the
/// design names but no group has built is `HintState::Planned` and is never
/// drawn.
///
/// What that table cannot say is **where** a binding is live, and `⏎ open`
/// went straight through the hole: `Enter` is bound, and with the detail
/// pane open on its diff it was consumed and did nothing while the bar
/// still advertised it (m5 review, Q2). So every hint is also **pressed**,
/// into a state that really is the screen its bar is drawn on.
///
/// **"Answered", without a second table of effects.** Each key is pressed
/// twice over: once as itself, once as [`INERT`] — a key nothing binds.
/// Whatever the state does on any event at all (the evil-merge probe
/// re-asking for rows, a job taking an id) happens under both, so a key is
/// answered exactly when what comes back differs from the inert press.
/// Nothing here states what a key *should* do, so there is no second table
/// to drift out of step with the first.
///
/// **Enumerated, not sampled.** The contexts come from
/// `graph_screen::every_context`, the one list every sweep now reads, and
/// [`state_for`] matches it exhaustively — a context added to the enum
/// cannot compile until it has been given a state here.
/// [`every_swept_state_is_the_context_it_claims`] then makes the app's own
/// classifier confirm each state, so a fixture cannot quietly be a
/// different screen from the one whose bar is being checked.
///
/// **A hint naming two keys** (`j/k`, `<>`) is answered when *either* acts:
/// `k` on the top row has nowhere to go, and a boundary is not a dead key.
///
/// **At the width the bar is drawn for** (m5/33, review M6). The fixture is
/// built at each of [`SWEPT_WIDTHS`], not at 160 for all six, so the
/// handlers see the layout the bar belongs to — the sub-100-column `Tab`
/// layout is where tickets 29 and 31's defects lived, and the sweep was
/// pressing its bars into a wide side-by-side state. All six widths settle.
/// The `Divider` the bar is asked for comes from the state through
/// `view::rows::divider_shown` rather than from the loop, for the same
/// reason: a bar the app never draws is not a bar to check. Doing this
/// found one dead key straight away — `<> resize` under 100 columns, where
/// the pane is a tab strip with no divider to drag (fixed in
/// `mergewright_ui::graph_screen::GraphView::divider_shown`).
#[test]
fn every_key_the_bar_draws_is_answered() {
    use mergewright_ui::graph_screen::{Divider, every_context};

    let bound = super::super::keys::bound_actions();
    // Collected rather than asserted one at a time: when the bar and the
    // handlers disagree they tend to disagree in several places at once,
    // and a run that names one of them hides the shape of it.
    let mut dead: Vec<String> = Vec::new();
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in SWEPT_WIDTHS {
                dead.extend(dead_keys_on(&context, divider, columns, bound));
            }
        }
    }
    assert!(
        dead.is_empty(),
        "the bar draws {} key(s) the handler with focus does nothing with:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}

/// **No key a live-operation bar draws refuses because the operation is
/// live** (m8/15 A1). [`every_key_the_bar_draws_is_answered`] counts a
/// refusal as an answer — the status line moves — so it cannot see this;
/// here every drawn key of every `OperationLive` form (an `edit` stop
/// included, `LiveC::ContinueAmends`) is pressed into its screen and the
/// status it leaves must not be the live refusal (`undo::live_refusal_words`)
/// nor `m`'s detached-`HEAD` one (`preview/history.rs:112`). Holds
/// `mergewright_ui`'s `REFUSED_WHILE_LIVE` against the keys that really refuse.
#[test]
fn no_key_a_live_operation_bar_draws_refuses_while_live() {
    use mergewright_ui::graph_screen::every_context;

    let mut refused: Vec<String> = Vec::new();
    let mut pressed = 0;
    for context in every_context() {
        if !matches!(context, HintContext::OperationLive { .. }) {
            continue;
        }
        for columns in [80, 120, 160] {
            for state in states_for(&context, Divider::Hidden, columns) {
                let state = settled(state);
                let banner = crate::update::banner::banner_view(&state);
                let drawn = crate::view::rows::footer_context(&state, &[], banner.as_ref());
                for hint in bar_hints(&state, &drawn, Divider::Hidden, columns) {
                    for press in presses_for(&hint) {
                        pressed += 1;
                        let status = format!("{:?}", outcome(&state, press).0.status);
                        if status.contains("finish or abort the")
                            || status.contains("needs a branch checked out")
                        {
                            refused.push(format!(
                                "{context:?} at {columns}: `{} {}` → {status}",
                                hint.key(),
                                hint.verb
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(pressed > 50, "only {pressed} presses swept");
    assert!(
        refused.is_empty(),
        "a live bar offers {} key(s) that refuse:\n  {}",
        refused.len(),
        refused.join("\n  ")
    );
}

/// **A stash's row draws the stash bar, and no key on it refuses for being
/// a stash** (m8/15 A4): the drawn frame classifies the row `StashRow`
/// (bare, and inside a live operation's wrap), and every drawn key pressed
/// there leaves no `a stash isn't a branch` (`preview/history.rs:185`).
#[test]
fn a_stash_row_is_classified_and_no_key_on_its_bar_refuses_a_stash() {
    let mut refused: Vec<String> = Vec::new();
    for columns in [80, 120, 160] {
        for state in states_for(&HintContext::StashRow, Divider::Hidden, columns) {
            let state = settled(state);
            let view = crate::view::rows::graph_view(&state, 0);
            assert_eq!(view.hint_context, HintContext::StashRow, "at {columns}");
            for hint in bar_hints(&state, &view.hint_context, Divider::Hidden, columns) {
                for press in presses_for(&hint) {
                    let status = format!("{:?}", outcome(&state, press).0.status);
                    if status.contains("isn't a branch") || status.contains("takes a commit") {
                        refused.push(format!(
                            "{columns}: `{} {}` → {status}",
                            hint.key(),
                            hint.verb
                        ));
                    }
                }
            }
        }
    }
    assert!(refused.is_empty(), "{}", refused.join("\n"));
}

/// **`?`, `:` and `⌃P` on the graph each open the palette** (m8/15 scope:
/// the old "they do nothing" finding, closed by G-palette).
#[test]
fn help_colon_and_ctrl_p_on_the_graph_each_open_the_palette() {
    let context = HintContext::PlainRow {
        verbs: mergewright_ui::graph_screen::hints::RowVerbs::NONE,
    };
    let state = states_for(&context, Divider::Hidden, 120)
        .into_iter()
        .next()
        .map(settled)
        .expect("a graph screen");
    assert!(state.palette.is_none());
    for press in [
        KeyPress::plain(KeyCode::Char('?')),
        KeyPress::plain(KeyCode::Char(':')),
        KeyPress {
            code: KeyCode::Char('p'),
            modifiers: Modifiers::CONTROL,
        },
    ] {
        let (after, _) = update(&state, AppEvent::Core(Event::Key(press)));
        assert!(after.palette.is_some(), "{press:?} opened no palette");
    }
}

/// **`? keys` opens a palette he can see, everywhere it is drawn** (m8/11c):
/// over the Map, journal, stash (graph and pane focus), blame, history and
/// a marked row, `?` changes state *and* the drawn frame holds the
/// palette's `keys here` title — the unseen-modal class 09e fixed for `/`.
/// Lives here, not in `view::tests`, for `states_for`.
#[test]
fn the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn() {
    use mergewright_ui::graph_screen::{Divider, every_context, hints_for, hints_text};

    const COLUMNS: u16 = 120;
    const ROWS: u16 = 36;
    let mut unseen: Vec<String> = Vec::new();
    let mut checked = 0;
    for context in every_context() {
        let place = matches!(
            context,
            HintContext::PaneOverOverlay { .. }
                | HintContext::ChangesPane {
                    over_overlay: true,
                    ..
                }
                | HintContext::Blame { .. }
                | HintContext::History { .. }
                | HintContext::BackToBlame(_)
                | HintContext::BackToHistory(_)
                | HintContext::Marked(_)
        );
        let keys = mergewright_ui::keymap::Keymap::shipped();
        let bar = hints_text(&hints_for(&context, Divider::Hidden, COLUMNS, keys), keys);
        if !place || !bar.contains("? keys") {
            continue;
        }
        for state in states_for(&context, Divider::Hidden, COLUMNS) {
            let state = settled(AppState {
                size: Size {
                    columns: COLUMNS,
                    rows: ROWS,
                },
                ..state
            });
            let (after, _) = crate::update::update(&state, key(KeyCode::Char('?')));
            checked += 1;
            let frame = crate::view::tests::draw(&after, COLUMNS, ROWS);
            if !frame.contains("keys here") {
                unseen.push(format!("{context:?}"));
            }
        }
    }
    // The journal, stash and Map draw their own footers (m8/11e): each
    // state's drawn footer carries the doorway, and `?` opens the palette.
    for (what, state) in place_footers::place_footer_states() {
        let (state, frame) = place_footers::drawn_at(&state, COLUMNS, ROWS);
        let footer = place_footers::footer_row(&frame);
        assert!(
            footer.contains("? keys"),
            "{what}: the footer draws no doorway: {footer:?}"
        );
        let (after, _) = crate::update::update(&state, key(KeyCode::Char('?')));
        checked += 1;
        if !crate::view::tests::draw(&after, COLUMNS, ROWS).contains("keys here") {
            unseen.push(what);
        }
    }
    assert!(checked >= 8, "only {checked} states swept");
    assert!(
        unseen.is_empty(),
        "`?` drew no palette over:\n  {}",
        unseen.join("\n  ")
    );
}
