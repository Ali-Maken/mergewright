//! The banner's focus and keys: `Tab` onto it, `c`/`s`/`a` routed from it.

use super::{
    AppState, Command, JobId, KeyCode, KeyPress, ReturnTo, Status, abort_pressed, banner_view,
    continue_pressed, edit_conflict, first_conflicted, open_mergetool, skip_pressed,
};

/// `Tab`/`Shift+Tab` **into** the banner: the door back in.
///
/// The detail pane cycles Graph → Files → Diff → Graph. While a banner is
/// drawn but unfocused, the step that would land back on the graph lands on
/// the banner instead, so the cycle reads Graph → Files → Diff → banner →
/// Graph — and with the detail pane hidden, one `Tab` from the graph.
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
    let lands_on_the_graph = !state.detail.shown
        || matches!(
            (state.detail.focus, forward),
            (graft_ui::detail::DetailFocus::Diff, true)
                | (graft_ui::detail::DetailFocus::Files, false)
        );
    if !lands_on_the_graph {
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
        match key.code {
            KeyCode::Tab => state.detail.focus = graft_ui::detail::DetailFocus::Graph,
            KeyCode::BackTab => state.detail.focus = graft_ui::detail::DetailFocus::Diff,
            _ => {}
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
    let plain = !key.modifiers.control && !key.modifiers.alt;
    match key.code {
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Escape => hand_the_keyboard_back(state, key),
        KeyCode::Char('c') if plain => {
            continue_pressed(state, commands, ReturnTo::Banner, next_id);
        }
        KeyCode::Char('s') if plain => skip_pressed(state),
        KeyCode::Char('a') if plain => abort_pressed(state),
        KeyCode::Char('M') if plain => match first_conflicted(state) {
            Some(path) => {
                open_mergetool(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
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
        KeyCode::Char('e') if plain => match first_conflicted(state) {
            Some(path) => {
                edit_conflict(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        // `u`/`U` answer exactly as the global keys do, focused or not —
        // `undo_key` is the one path. That is a refusal **in words** while an
        // operation is live (§2.6a); they used to fall into the silent arm
        // below, so the key pressed on purpose drew nothing at all (review
        // M3). In a merge begun in the shell they are sent, and ops' gate
        // decides (§2.6a's one exception, m6-conflicts/57).
        KeyCode::Char(letter @ ('u' | 'U')) if plain => {
            // The refusal first where there is one: `undo_key` asks the
            // undo-after-push box before `launch` refuses, and a box on a key
            // that then refuses is not what the banner has ever answered. In
            // a merge begun in the shell there is none (`in_progress_refusal`
            // is `None`), so `undo_key` runs as is: when the next undo is a
            // push, the push box opens first, and "Undo locally" then meets
            // ops' `OperationLive` refusal (review 54–58 m1; KNOWN_GAPS
            // M6/57 holds the question).
            match crate::update::undo::in_progress_refusal(state) {
                Some(refusal) => state.status = refusal,
                None => {
                    commands.extend(crate::update::undo::undo_key(state, letter == 'U', next_id));
                }
            }
        }
        // `q` quits as it does from the graph, through the same path, so the
        // stopped or running box asks first. It used to fall into the silent
        // arm below and read as a hang (review m8-13 Major 1).
        KeyCode::Char('q') if plain => crate::update::quit::asked(state, key),
        // Swallowed on purpose, and **silently**: §2.1's modal rule.
        // Nothing outside the banner is live while it has focus, so `u`
        // cannot undo the entry *before* the one still running (answered
        // in words just above, per §2.6) and `S` cannot stash a tree full
        // of conflict markers. Anything reaching here draws nothing — the
        // banner's own hint bar is what says which keys do apply.
        _ => {}
    }
    true
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
