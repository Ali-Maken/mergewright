//! What was wrong with his `[keys]` table, in words for the status row and
//! `?`'s `YOUR CONFIG` group. Never fatal: the graph opens as normal.

use super::{ActionId, ContextId, KeyChord, Keymap, defaults};

/// One problem with one override line.
#[derive(Debug, PartialEq, Eq)]
pub enum KeyWarning {
    /// No action has this id (`grpah.merge`).
    UnknownAction(String),
    /// The text names no key, or `none` sits inside a list.
    BadKey {
        /// The action the line was for.
        action: String,
        /// The text as he wrote it.
        text: String,
    },
    /// One chord ended up on two actions in `context`: `kept` answers it
    /// there, `lost` no longer does.
    Collision {
        /// The chord.
        key: KeyChord,
        /// Where the two meet.
        context: ContextId,
        /// The action that answers it there.
        kept: ActionId,
        /// The action that lost it there.
        lost: ActionId,
    },
    /// `Ctrl+C` always quits: it was given to `action`, or `app.quit_now`
    /// was moved (to `key`).
    Reserved {
        /// The action the line was for.
        action: String,
        /// The chord asked for.
        key: KeyChord,
    },
    /// The action lives in a text field, where keys are never remapped.
    Fixed(ActionId),
}

impl KeyWarning {
    /// The problem in one lowercase line, keys spelled as the app spells
    /// them.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::UnknownAction(name) => format!("{name} is not an action"),
            Self::BadKey { action, text } => format!("\"{text}\" is not a key ({action})"),
            Self::Reserved { action, key } if action == "app.quit_now" => format!(
                "⌃C always quits — app.quit_now can't move to {}",
                key.label().0
            ),
            Self::Reserved { action, .. } => format!("⌃C always quits — not given to {action}"),
            Self::Fixed(id) => format!(
                "{} can't be remapped — keys in a text field stay as they are",
                id.0
            ),
            Self::Collision {
                key,
                context,
                kept,
                lost,
            } => collision_words(*key, *context, kept, lost),
        }
    }
}

/// `M given to graph.merge — map view has no key` when his override took
/// the chord; `o given to app.undo — in the resolver o stays take ours`
/// when his override lost it.
fn collision_words(key: KeyChord, ctx: ContextId, kept: &ActionId, lost: &ActionId) -> String {
    let k = key.label().0;
    let lost_had_it = binding(lost).is_some_and(|b| b.default.contains(&key));
    if !lost_had_it {
        return format!(
            "{k} given to {} — {} {k} stays {}",
            lost.0,
            place(ctx),
            label(kept)
        );
    }
    let left = Keymap::default().left_in(lost, ctx, key);
    let what = match left {
        Some(other) => format!("{} keeps {}", label(lost), other.label().0),
        None => format!("{} has no key", label(lost)),
    };
    let lost_home = binding(lost).map(|b| b.context);
    if lost_home == Some(ctx) {
        format!("{k} given to {} — {what}", kept.0)
    } else {
        format!("{k} given to {} — {what} {}", kept.0, place(ctx))
    }
}

fn binding(action: &ActionId) -> Option<&'static super::Binding> {
    defaults().iter().find(|b| b.action == *action)
}

/// The action's label, first letter lowercased, to sit inside a sentence.
fn label(action: &ActionId) -> String {
    let words = binding(action).map_or(action.0, |b| b.label);
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Where, as a phrase: `on the graph`, `in the resolver`.
fn place(ctx: ContextId) -> &'static str {
    match ctx {
        ContextId::Global => "everywhere",
        ContextId::Graph => "on the graph",
        ContextId::Map => "in the map view",
        ContextId::Preview => "in a preview",
        ContextId::Confirm => "in a confirm box",
        ContextId::Changes => "in the changes pane",
        ContextId::Diff => "in a diff",
        ContextId::Resolver => "in the resolver",
        ContextId::Banner => "on the operation banner",
        ContextId::Finish => "in the finish summary",
        ContextId::Rewrite => "in the rewrite editor",
        ContextId::Picker => "in the branch picker",
        ContextId::Blame => "in blame",
        ContextId::Stash => "in the stash list",
        ContextId::CommitEditor => "in the commit editor",
        ContextId::Journal => "in the undo journal",
        ContextId::History => "in history",
        ContextId::Palette => "in the palette",
        ContextId::EmptyState => "on the start screen",
        ContextId::Tour => "in the tour",
        ContextId::Credential => "in the credential prompt",
        ContextId::Menu => "in the . menu",
    }
}
