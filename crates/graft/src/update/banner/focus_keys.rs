//! The banner's focus and keys: `Tab` onto it, `c`/`s`/`a` routed from it.

use super::{
    AppState, Command, JobId, KeyCode, KeyPress, ReturnTo, Status, abort_pressed, banner_view,
    continue_pressed, edit_conflict, first_conflicted, open_mergetool, skip_pressed,
};
use graft_ui::keymap::{ActionId, KeyChord, Keymap};

/// `Tab`/`Shift+Tab` **into** the banner: the door back in.
///
/// The detail pane cycles Graph → Files → Diff → Graph. While a banner is
/// drawn but unfocused, the step that would land back on the graph lands on
/// the banner instead, so the cycle reads Graph → Files → Diff → banner →
/// Graph — and with the detail pane hidden, one `Tab` from the graph.
/// `Shift+Tab` runs the same cycle backwards (keymap §2.2, m8/20d): one
/// press from the graph. Both steps are `stop::next_of` / `prev_of`.
///
/// A mode you can leave and never re-enter is worse than no mode at all:
/// `Tab` out of a conflict would otherwise strand `c`, `s` and `a` behind a
/// key with nothing on screen to find it by.
///
/// `true` when it took the focus, in which case the detail pane must not
/// also move.
pub fn take_focus(state: &mut AppState, key: KeyPress) -> bool {
    if state.banner_focus || banner_view(state).is_none() {
        return false;
    }
    let forward = match key.code {
        KeyCode::Tab if !key.modifiers.control && !key.modifiers.alt => true,
        KeyCode::BackTab => false,
        _ => return false,
    };
    let (at, shown) = super::stop::pane_place(state);
    let step = if forward {
        super::stop::next_of
    } else {
        super::stop::prev_of
    };
    if step(at, shown, true) != Some(super::stop::TabTarget::Banner) {
        return false;
    }
    state.banner_focus = true;
    true
}

/// `Tab`, `Shift+Tab` or `Esc`: the banner gives the keyboard up.
///
/// `Tab` out of the banner is the next step of the cycle, not a return
/// ticket: the rule is graph → detail → banner → graph. Leaving
/// `detail.focus` where it was sent `Tab` straight back into the banner from
/// the Diff pane, so the graph was unreachable by `Tab` for as long as a
/// banner was drawn (m5-09 review, Major 1). `BackTab` is the same cycle run
/// backwards, so it lands on the pane the forward step came from.
///
/// `Esc` is not part of the cycle — it is "leave the mode" — so it puts the
/// keyboard back where it found it.
pub(super) fn hand_the_keyboard_back(state: &mut AppState, key: KeyPress) {
    // Graft waiting to quit on a `c` it launched from here (m8/13b): `Esc`
    // drops the wait, as on the graph (`remote::stop_write`), and that is
    // all it does — the banner keeps the keyboard.
    if key.code == KeyCode::Escape && crate::update::quit::drop_wait(state) {
        return;
    }
    // `Esc` dismisses a problem on the status line, as `keys::key_event`
    // does app-wide (the banner reaches the key first), and still leaves
    // the banner in the same press — one `Esc` from a refused `c` to the
    // pane, as before m6-conflicts/12.
    if key.code == KeyCode::Escape && matches!(state.status, Status::Sticky { .. }) {
        state.status = Status::Ready;
    }
    state.banner_focus = false;
    if state.detail.shown {
        use super::stop::{TabTarget, next_of, prev_of};
        use graft_ui::detail::DetailFocus;
        let to = match key.code {
            KeyCode::Tab => next_of(TabTarget::Banner, true, true),
            KeyCode::BackTab => prev_of(TabTarget::Banner, true, true),
            _ => None,
        };
        match to {
            Some(TabTarget::Graph) => state.detail.focus = DetailFocus::Graph,
            Some(TabTarget::Files) => state.detail.focus = DetailFocus::Files,
            Some(TabTarget::Diff) => state.detail.focus = DetailFocus::Diff,
            Some(TabTarget::Banner) | None => {}
        }
    }
    // `Tab`/`⇧Tab` leave the status line alone (m6-conflicts/12). They used
    // to reset a `Sticky` on the reasoning that the banner's own line was
    // one; the stop is `AppState::stopped` now, so any `Sticky` still up is
    // a real problem — a failed resume's `a abort` — and only `Esc`
    // dismisses one (`Status::Sticky`'s contract), above. The stop itself
    // (`AppState::stopped`) is untouched by either.
}

/// The banner's keys, while it has focus.
///
/// **Modal**: `true` for every key but `Ctrl+C`, which passes so that a
/// terminal application never ignores it (`is_quit`'s reasoning, and §2.1's
/// one exception). Plain `q` is answered here by the one quit path,
/// `quit::asked`, which boxes first while a write runs or git is stopped
/// (keymap §2.2; m8-polish-and-ship/13, review Major 1) — a stop, a
/// relaunch and an undo all land on the banner, so that is where `q` is
/// pressed.
pub fn banner_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if !state.banner_focus || banner_view(state).is_none() {
        return false;
    }
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    if let Some(claimed) = undo_keys(state, commands, key, next_id) {
        return claimed;
    }
    if banner_press(state, commands, key, next_id) {
        return true;
    }
    let plain = !key.modifiers.control && !key.modifiers.alt;
    match key.code {
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Escape => hand_the_keyboard_back(state, key),
        // `⏎` opens the resolver on the first conflicted file
        // (`docs/design/02-keymap.md` §2.6a, m6-conflicts/04). It was the
        // catch-all's before, which meant the one key §2.2 gives to "open /
        // drill in" did nothing at all on the banner.
        KeyCode::Enter if plain => match first_conflicted(state) {
            Some(path) => {
                crate::update::resolver::open_resolver(
                    state,
                    commands,
                    path,
                    crate::update::resolver::OpenedFrom::Banner,
                    next_id,
                );
            }
            None => nothing_conflicted(state),
        },
        // `q` quits as it does from the graph, through the same path, so the
        // stopped or running box asks first. It used to fall into the silent
        // arm below and read as a hang (review m8-13 Major 1).
        KeyCode::Char('q') if plain => crate::update::quit::asked(state, key),
        // Swallowed on purpose, and **silently**: §2.1's modal rule.
        // Nothing outside the banner is live while it has focus, so `u`
        // cannot undo the entry *before* the one still running (answered
        // in words by `undo_keys`, per §2.6) and `S` cannot stash a tree full
        // of conflict markers. Anything reaching here draws nothing — the
        // banner's own hint bar is what says which keys do apply.
        _ => {}
    }
    true
}

/// The banner's action keys (`c`, `s`, `a`, `M`, `e` — its `ContextId::Banner`
/// rows), applied whatever `banner_focus` is; focus unchanged. `false` for
/// any other key.
///
/// Callers: [`banner_key`] (focused), and `update::palette::run` for an
/// `IN PROGRESS` row (m8/12e), which runs in the context it was spelled in —
/// the banner's — so `a Abort` from `?` on the graph asks to abort, never
/// reaches the graph's `a` (amend; review m8-palette-theme Major 1).
pub(crate) fn banner_press(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if key.modifiers.control || key.modifiers.alt {
        return false;
    }
    match key.code {
        KeyCode::Char('c') => {
            continue_pressed(state, commands, ReturnTo::Banner, next_id);
        }
        KeyCode::Char('s') => skip_pressed(state),
        KeyCode::Char('a') => abort_pressed(state),
        KeyCode::Char('M') => match first_conflicted(state) {
            Some(path) => {
                open_mergetool(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        KeyCode::Char('e') => match first_conflicted(state) {
            Some(path) => {
                edit_conflict(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        _ => return false,
    }
    true
}

/// `app.undo` (`u`, `⌃Z`) and `app.redo` (`U`) on the focused banner;
/// `None` for any other key. The chords are read off the shipped table:
/// the edge hands a handler shipped chords only (`update/edge.rs`), so this
/// is the action whichever chord he pressed and whatever he moved it to.
///
/// They answer exactly as the global keys do, focused or not — `undo_key`
/// is the one path. That is a refusal **in words** while an operation is
/// live (§2.6a); `u`/`U` used to fall into the silent arm, so the key
/// pressed on purpose drew nothing at all (review M3), and `⌃Z` did until
/// m8/09g C. In a merge begun in the shell they are sent, and ops' gate
/// decides (§2.6a's one exception, m6-conflicts/57). The refusal comes
/// first: `undo_key` asks the undo-after-push box before `launch` refuses,
/// and a box on a key that then refuses is not what the banner has ever
/// answered. With no refusal, when the next undo is a push, the push box
/// opens first, and "Undo locally" then meets ops' `OperationLive` refusal
/// (review 54–58 m1; `KNOWN_GAPS` M6/57 holds the question).
///
/// With `suspend_on_ctrl_z`, `⌃Z` is `app.suspend`, never undo: not the
/// banner's (`Some(false)`), so it goes on to the one suspend path
/// (`update/keys.rs`), as from the graph.
fn undo_keys(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<bool> {
    let chord = KeyChord::from_press(&key)?;
    if state.suspend_on_ctrl_z && chord == KeyChord::ctrl('z') {
        return Some(false);
    }
    let of = |id| Keymap::shipped().chords(&ActionId(id)).contains(&chord);
    let redo = match (of("app.undo"), of("app.redo")) {
        (true, _) => false,
        (_, true) => true,
        _ => return None,
    };
    match crate::update::undo::in_progress_refusal(state) {
        Some(refusal) => state.status = refusal,
        None => commands.extend(crate::update::undo::undo_key(state, redo, next_id)),
    }
    Some(true)
}

/// `M`/`e` with nothing unmerged: say so, and name the two keys that do
/// apply.
///
/// The banner acts on "the first conflicted file", and a stalled
/// `--continue` or a tree the user has just resolved has none. Both keys
/// used to return `false` into a `banner_key` that claimed them anyway, so
/// they read as broken (m5-09 review, Major 2). The hint bar drops them in
/// this state too (`hints::banner_list`'s `conflicted`), so this line is
/// what a user who learned the key elsewhere gets.
pub(super) fn nothing_conflicted(state: &mut AppState) {
    state.status = Status::Sticky {
        reason: "nothing conflicted".to_owned(),
        keys: continue_abort(&state.keymap),
    };
}

/// A stop's two ways out, `c continue  a abort`, spelled from his keymap
/// (m8/10f, 10b W1): the banner's rows, an unbound one left out, `None`
/// when both are. The one wording of this state: the banner's lines and the
/// journal's `in progress` row both take it.
pub(crate) fn continue_abort(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    let words = keys.key_verbs(&[
        ("operation.continue", "continue"),
        ("operation.abort", "abort"),
    ]);
    (!words.is_empty()).then_some(words)
}

/// `a abort` alone, spelled from his keymap; `None` when unbound.
pub(crate) fn abort_key(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(&[graft_ui::keymap::ActionId("operation.abort")], "abort")
}

#[cfg(test)]
mod tests {
    use graft_core::event::{Event, KeyCode as CoreKey, KeyPress as CorePress};
    use graft_git::state::InProgress;
    use graft_ui::detail::DetailFocus;

    use crate::update::{AppEvent, AppState, update};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Place {
        Graph,
        Files,
        Diff,
        Banner,
    }

    fn place(state: &AppState) -> Place {
        if state.banner_focus {
            return Place::Banner;
        }
        if !state.detail.shown {
            return Place::Graph;
        }
        match state.detail.focus {
            DetailFocus::Graph => Place::Graph,
            DetailFocus::Files => Place::Files,
            DetailFocus::Diff => Place::Diff,
        }
    }

    fn put(state: &mut AppState, at: Place) {
        state.banner_focus = at == Place::Banner;
        state.detail.focus = match at {
            Place::Files => DetailFocus::Files,
            Place::Diff => DetailFocus::Diff,
            Place::Graph | Place::Banner => DetailFocus::Graph,
        };
    }

    fn press(state: &AppState, code: CoreKey) -> AppState {
        update(state, AppEvent::Core(Event::Key(CorePress::plain(code)))).0
    }

    /// Every state a banner is drawn in where `Tab` moves focus (check 1):
    /// a conflict stop and an `edit` stop, each with the pane closed and
    /// open. The resolver owns its keys (`modal_owns_keys`), so it has no
    /// cycle to reverse.
    fn banner_states() -> Vec<(&'static str, AppState, &'static [Place])> {
        let mut out = Vec::new();
        for (name, base) in [
            (
                "conflict stop",
                crate::update::banner::tests::stopped(InProgress::Rebase, &["a.rs"]),
            ),
            (
                "edit stop",
                crate::update::banner::rewrite_stop_tests::edit_stop(),
            ),
        ] {
            assert!(
                crate::update::banner::banner_view(&base).is_some(),
                "{name}: a banner is drawn"
            );
            let mut closed = base.clone();
            closed.detail.shown = false;
            out.push((name, closed, &[Place::Graph, Place::Banner][..]));
            let mut open = base;
            open.detail.shown = true;
            out.push((
                name,
                open,
                &[Place::Graph, Place::Files, Place::Diff, Place::Banner][..],
            ));
        }
        out
    }

    /// Keymap §2.2: `Shift+Tab` is the previous pane, so from every place
    /// `Tab` then `⇧⇥` comes home, and `⇧⇥` then `Tab` does too.
    #[test]
    fn back_tab_is_the_tab_cycle_reversed_in_every_banner_state() {
        for (name, state, places) in banner_states() {
            let shown = state.detail.shown;
            for &at in places {
                let mut start = state.clone();
                put(&mut start, at);
                for (first, second) in [
                    (CoreKey::Tab, CoreKey::BackTab),
                    (CoreKey::BackTab, CoreKey::Tab),
                ] {
                    let moved = press(&start, first);
                    assert_ne!(
                        place(&moved),
                        at,
                        "{name}, pane shown {shown}: {first:?} from {at:?} moves"
                    );
                    let back = press(&moved, second);
                    assert_eq!(
                        place(&back),
                        at,
                        "{name}, pane shown {shown}: {first:?} from {at:?} → {:?}, then {second:?}",
                        place(&moved)
                    );
                }
            }
        }
    }
}
