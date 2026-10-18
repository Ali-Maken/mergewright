//! `update` tests: sweep. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The screen widths [`every_key_the_bar_draws_is_answered`] sweeps: the
/// two layout breakpoints (100 and 160 columns, DESIGN.md §1) with a width
/// either side of each. The state is built at the width whose bar is being
/// checked, so the handlers see the layout the bar was drawn for
/// (m5/33, review M6).
pub(super) const SWEPT_WIDTHS: [u16; 6] = [40, 80, 100, 120, 160, 200];

/// A key nothing in the app binds — the baseline every press is measured
/// against.
const INERT: KeyCode = KeyCode::Function(12);
/// A second one, so the baseline can be shown to *be* a baseline rather
/// than an action of its own.
pub(super) const ALSO_INERT: KeyCode = KeyCode::Function(11);

/// The state once the app has stopped reacting to being looked at.
///
/// A freshly built pane asks for what it is missing on the first event it
/// sees (`ListFiles` for a pane whose files are `Idle`), which moves the
/// state under a key that binds nothing and would leave the whole screen
/// reading as answered. Pressing an unbound key until nothing moves settles
/// that; whatever moves afterwards is the key's doing.
///
/// A drag is not settled — every key ends one (see the sweep's `blind`),
/// so settling would put the badge down and throw the context away.
pub(super) fn settled(mut state: AppState) -> AppState {
    // A confirm box is left alone too: it swallows every key it has no
    // use for (`confirm::confirm_key`'s fall-through), so there is
    // nothing to settle.
    if state.drag.is_some() || state.confirm.is_some() {
        return state;
    }
    for _ in 0..4 {
        let (next, _) = update(&state, key(INERT));
        if next == state {
            return state;
        }
        state = next;
    }
    panic!("the state never stopped moving under a key that binds nothing");
}

/// What this state looks like after a key **nothing binds** — the shape of
/// nothing happening here.
///
/// It has to be a real press, not an empty outcome: `update` does ambient
/// work on every event whatever the key (the evil-merge probe re-asks for
/// its rows, a job takes an id), so an absolute comparison would read every
/// key as answered.
pub(super) fn nothing_happened(state: &AppState) -> (AppState, Vec<Command>) {
    let idle = update(state, key(INERT));
    assert_eq!(
        update(state, key(ALSO_INERT)),
        idle,
        "the baseline must be a key that does nothing, not one that does something"
    );
    idle
}

/// Does the key the bar spells `name` do anything at all in this state,
/// beyond what a key nothing binds already does?
///
/// This used to be `outcome(..) != *idle`, under a comment claiming that
/// comparing against the inert press is what catches a key **consumed by
/// the focused handler that then does nothing**. It is the inverse (m5/33,
/// review M2). Such a key returns early, so the ambient work never runs and
/// its outcome differs from the baseline by having *less* in it — and a
/// plain `!=` reads any difference as an answer, so the one defect the
/// sweep exists for was the one it waved through.
///
/// The real guarantee is [`did_something`]: **added**, not merely
/// different. Nothing here assumes the baseline is empty, because it is
/// not — `update` does ambient work on every event whatever the key (the
/// evil-merge probe re-asks for its rows), and the inert press comes back
/// carrying that command like any other press.
pub(super) fn answered(state: &AppState, idle: &(AppState, Vec<Command>), hint: &Hint) -> bool {
    presses_for(hint)
        .into_iter()
        .any(|press| did_something(idle, &outcome(state, press)))
}

/// Did this press **add** anything to what a key nothing binds already
/// does: move the state, or produce a command the baseline did not?
///
/// Losing the baseline's ambient work is not an answer, so a press whose
/// commands are a subset of the baseline's and whose state is the
/// baseline's counts as nothing happening.
pub(super) fn did_something(
    idle: &(AppState, Vec<Command>),
    outcome: &(AppState, Vec<Command>),
) -> bool {
    outcome.0 != idle.0 || outcome.1.iter().any(|command| !idle.1.contains(command))
}

/// What a hint tells the hand to do. Not every entry on the bar is a key:
/// `release drop on a row` is the mouse button coming up, which is how a
/// badge drag ends (`update::drag`). The sweep **performs** it rather than
/// exempting it — a rule that needs no list of words to skip, and one that
/// would notice if the release stopped ending the drag.
#[derive(Clone, Copy, Debug)]
pub(super) enum Press {
    /// A key went down.
    Key(KeyPress),
    /// The left button came up.
    Release,
}

/// The presses `hint` stands for: each key it names, as the shipped table
/// binds it (ticket m8/10 — a hint names actions, so its presses are those
/// actions' chords, not a parse of the string the bar drew). A hint naming
/// two keys stands for both.
///
/// The words that are not keys (`mergewright_ui::graph_screen::NOT_A_BINDING`)
/// are performed too; a word this sweep does not know stops it rather than
/// being quietly passed over, because a bar entry nobody can perform is the
/// defect this test is for.
pub(super) fn presses_for(hint: &Hint) -> Vec<Press> {
    let shipped = Keymap::shipped();
    let chord = |chord: &KeyChord| Press::Key(chord.to_press());
    hint.keys
        .iter()
        .flat_map(|key| match key {
            // What the bar drew: the first chord for `Key`, every one for
            // `EveryChord`. A `Fixed` key is a text field's default.
            HintKey::Key(action) | HintKey::Fixed(action) => shipped
                .chords(action)
                .first()
                .map(chord)
                .into_iter()
                .collect(),
            HintKey::EveryChord(action) => shipped.chords(action).iter().map(chord).collect(),
            HintKey::Word("release") => vec![Press::Release],
            // The branch picker's `type to find` (m7-power-tools/10): any
            // letter is typing there, so one letter stands for them all.
            HintKey::Word("type") => vec![Press::Key(KeyPress::plain(KeyCode::Char('m')))],
            // The picker's rows (`shown_key`'s `Up`/`Down` arms).
            HintKey::Word("↑/↓") => vec![
                Press::Key(KeyPress::plain(KeyCode::Up)),
                Press::Key(KeyPress::plain(KeyCode::Down)),
            ],
            HintKey::Word(word) => panic!(
                "the bar draws `{word}`, which this sweep cannot perform — teach \
                 `presses_for` what it is, or it is a bar entry nobody can act on"
            ),
        })
        .collect()
}

/// The actions `hint` names: every key but a word.
pub(super) fn actions_of(hint: &Hint) -> Vec<ActionId> {
    hint.keys
        .iter()
        .filter_map(|key| match key {
            HintKey::Key(action) | HintKey::EveryChord(action) | HintKey::Fixed(action) => {
                Some(*action)
            }
            HintKey::Word(_) => None,
        })
        .collect()
}

/// `press` put through the whole of `update`, exactly as the event loop
/// would deliver it.
pub(super) fn outcome(state: &AppState, press: Press) -> (AppState, Vec<Command>) {
    let event = match press {
        Press::Key(key) => AppEvent::Core(Event::Key(key)),
        Press::Release => AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        })),
    };
    update(state, event)
}

/// **The screen each context's bar is drawn on.** Exhaustive over
/// `HintContext`, so a context added to the enum has to be given a state
/// here before this crate compiles again — which is what keeps the sweep
/// from quietly covering one context fewer than the bar can draw.
///
/// `divider` is not decoration: `Divider::Shown` means a pane is open on
/// this frame, and `<> resize` is drawn because of it. A state built
/// without the pane would fail its own hint.
pub(super) fn states_for(context: &HintContext, divider: Divider, columns: u16) -> Vec<AppState> {
    screen_for(context)
        .into_iter()
        .map(|mut state| {
            // The width the bar being checked was drawn for (m5/33, review
            // M6). It used to be 160 for every fixture while the sweep drew
            // bars at six widths, so every narrow bar was pressed into a
            // wide, side-by-side state — and the `Tab` layout under 100
            // columns is exactly where tickets 29 and 31's defects lived.
            state.size = Size { columns, rows: 48 };
            one_stash(&mut state);
            if holds_stash_row(context) {
                stash_under_cursor(&mut state);
            }
            if holds_search(context) {
                search_standing(&mut state);
            }
            if divider == Divider::Shown && !state.detail.shown {
                // On a commit, even where the cursor is on the working-tree
                // row: a pane open on the working tree *is* the changes
                // pane, which is its own context. This is the moment after
                // the cursor moved onto the row and before the pane has
                // followed it.
                let spec = head_spec(&state)
                    .or_else(|| state.selected_spec())
                    .expect("a row to open the pane on");
                show_pane(&mut state, spec, DetailFocus::Graph);
            }
            state
        })
        // With the pane open and the graph (or its file list) focused,
        // `Tab` goes to the pane, not the banner, so the bar leads with
        // `⇥ banner` only where one `Tab` lands there
        // (`banner::one_press_to_banner`, `⇧⇥` too since m8/20d-b): each
        // state is the form it claims.
        .filter(|state| match context {
            HintContext::OperationLive { way_to_banner, .. } => {
                crate::update::banner::one_press_to_banner(state) == *way_to_banner
            }
            // `tab after` is the narrow editor's alone (R1's breakpoints).
            HintContext::RewriteEditor { narrow, .. } => {
                (state.size.columns < mergewright_ui::rewrite_editor::NARROW_BELOW) == *narrow
            }
            _ => true,
        })
        .collect()
}
