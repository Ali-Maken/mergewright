//! The box's bottom border: its own keys, spelled from his keymap, on
//! the widest rung that fits (ticket m8/10), and the verb slot ([`Again`]).

use crate::frame_box::cells;
use crate::keymap::{ActionId, Keymap};

use super::PreviewBox;

/// The box's own keys, spelled from `keys` (ticket m8/10): `←/→ choose`,
/// `⏎ act`, `esc cancel`, `esc back`, `y confirm`, `R keep`.
fn part(keys: &Keymap, actions: &[&'static str], verb: &str) -> Option<String> {
    let actions: Vec<ActionId> = actions.iter().map(|name| ActionId(name)).collect();
    keys.key_verb(&actions, verb)
}

/// Parts joined by the footer's two-space gap, an unbound one left out.
pub(super) fn join(parts: &[Option<String>]) -> String {
    parts
        .iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("  ")
}

/// The widest footer rung whose padded label fits `inner` cells:
/// `y confirm  r rebase  R keep  ←/→ choose  ⏎ act  esc cancel`, then
/// without the arrows, then without the verb, then without `R keep`, then
/// the plain `←/→ choose  ⏎ act  esc cancel`. Parts whose field is off are
/// left out of every rung; each part is whole or absent, never cut. Keys
/// are spelled from `keys`; one he left unbound is left out like an off
/// field.
pub(crate) fn footer_label(view: &PreviewBox, inner: usize, keys: &Keymap) -> String {
    let cancel = part(keys, &["app.back"], "cancel");
    let act = part(keys, &["preview.choose"], "act");
    let choose = part(keys, &["preview.left", "preview.right"], "choose");
    let plain = join(&[choose.clone(), act.clone(), cancel.clone()]);
    // From the choices, not a flag: every single-`Close` box — refused,
    // failed, undrawable, nothing to do — had the full footer while only
    // the stash put-back set the flag (m7-power-tools/15, :860).
    if view.choices.len() == 1 {
        return part(keys, &["app.back"], "back").unwrap_or_default();
    }
    let confirm = if view.confirm_key {
        part(keys, &["confirm.yes"], "confirm")
    } else {
        None
    };
    let again = view.again.and_then(|again| again.text(keys));
    if confirm.is_none() && again.is_none() {
        return plain;
    }
    let keep = if view.keep {
        part(keys, &["preview.rewrite"], "keep")
    } else {
        None
    };
    let rungs = [
        join(&[
            confirm.clone(),
            again.clone(),
            keep.clone(),
            choose,
            act.clone(),
            cancel.clone(),
        ]),
        join(&[
            confirm.clone(),
            again,
            keep.clone(),
            act.clone(),
            cancel.clone(),
        ]),
        join(&[confirm.clone(), keep, act.clone(), cancel.clone()]),
        join(&[confirm, act, cancel]),
    ];
    rungs
        .into_iter()
        .find(|label| cells(label) + 2 <= inner)
        .unwrap_or(plain)
}

/// The message drawn instead of the box when the frame is smaller than
/// [`PREVIEW_MIN`]: `preview needs 40x12 · esc closes`.
pub(super) fn too_small(keys: &Keymap) -> String {
    keys.after("preview needs 40x12", ActionId("app.back"), "closes")
}

/// The footer's verb slot: which key confirms, and the word after it.
///
/// `y` is `confirm.yes`, the one confirm key of every box (§2.3b); an `m`-
/// or `r`-opened preview also confirms on its opening key again, which is
/// `preview.confirm_merge` / `preview.confirm_rebase` — rows whose chords
/// follow `graph.merge`'s / `graph.rebase`'s (ticket m8/09c, D3). So with
/// `merge = "M"` the slot reads `M merge`, and with merge unbound it is
/// left out. Spelled from the action's first live chord.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Again {
    /// The action whose chord confirms.
    pub action: ActionId,
    /// The word after the key: `merge`, `apply`, `put it back`.
    pub verb: &'static str,
}

impl Again {
    /// `confirm.yes`'s first live chord, then `verb` (`y apply`).
    #[must_use]
    pub fn confirm(verb: &'static str) -> Self {
        Self {
            action: ActionId("confirm.yes"),
            verb,
        }
    }

    /// The opening key again: `action` is `preview.confirm_merge` or
    /// `preview.confirm_rebase` (`m merge`).
    #[must_use]
    pub fn opening(action: ActionId, verb: &'static str) -> Self {
        Self { action, verb }
    }

    /// `<key> <verb>` spelled from `keys`; `None` when the action has no
    /// live chord.
    #[must_use]
    pub fn text(&self, keys: &Keymap) -> Option<String> {
        let key = keys.spell_keys(&[self.action])?;
        Some(format!("{key} {}", self.verb))
    }
}
