//! What was wrong with his `[keys]` table, in words for the status row and
//! `?`'s `YOUR CONFIG` group. Never fatal: the graph opens as normal.

use super::{ActionId, ContextId, KeyChord, defaults};

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
    /// His line `line` changed what a key does somewhere: one problem per
    /// line, naming every place (ticket m8/09c, D1).
    Collision {
        /// The action his line is for.
        line: ActionId,
        /// Each place a key changed hands, in `ContextId::ALL` order.
        places: Vec<Clash>,
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
    /// The action has no key while `switch` is off; the line is not applied.
    SwitchOff {
        /// The action the line was for (`app.suspend`).
        action: ActionId,
        /// The `[keys]` switch it needs.
        switch: &'static str,
    },
    /// The action's keys are `leader`'s, never set on their own; the line
    /// is not applied.
    Follows {
        /// The action the line was for (`preview.confirm_merge`).
        action: ActionId,
        /// Whose keys it has (`graph.merge`).
        leader: ActionId,
    },
}

/// One place where a key changed hands: in `context`, `kept` answers `key`
/// and `lost` no longer does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clash {
    /// The chord.
    pub key: KeyChord,
    /// Where it changed hands.
    pub context: ContextId,
    /// The action that answers it there.
    pub kept: ActionId,
    /// The action that lost it there.
    pub lost: ActionId,
    /// What `lost` still answers to there after his lines, when his line
    /// took the key (`move down keeps ↓`); `None` when it has nothing left,
    /// or when his own line is the one that lost.
    pub keeps: Option<KeyChord>,
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
            Self::SwitchOff { action, switch } => {
                format!("{} does nothing while {switch} is off", action.0)
            }
            Self::Follows { action, leader } => format!(
                "{} can't be remapped — it is {}'s key, so move that",
                action.0, leader.0
            ),
            Self::Collision { line, places } => collision_words(line, places),
        }
    }
}

/// `j given to graph.merge — move down keeps ↓ on the graph` when his line
/// took the key; `o given to app.undo — on the graph o stays take, in the
/// resolver o stays take ours` when it lost it. One clause per place; a
/// line with several keys says each, `;` apart.
fn collision_words(line: &ActionId, places: &[Clash]) -> String {
    let mut keys: Vec<KeyChord> = Vec::new();
    for clash in places {
        if !keys.contains(&clash.key) {
            keys.push(clash.key);
        }
    }
    keys.iter()
        .map(|key| {
            let clauses: Vec<String> = places
                .iter()
                .filter(|clash| clash.key == *key)
                .map(|clash| clause(line, clash))
                .collect();
            format!(
                "{} given to {} — {}",
                key.label().0,
                line.0,
                clauses.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// One place's words: what lost the key there and what it keeps, or what
/// kept it when his line's own row lost.
fn clause(line: &ActionId, clash: &Clash) -> String {
    let k = clash.key.label().0;
    let ctx = clash.context;
    if is_lines(&clash.lost, line) {
        return format!("{} {k} stays {}", place(ctx), label(&clash.kept));
    }
    let what = match clash.keeps {
        Some(other) => format!("{} keeps {}", label(&clash.lost), other.label().0),
        None => format!("{} has no key", label(&clash.lost)),
    };
    if binding(&clash.lost).map(|b| b.context) == Some(ctx) {
        what
    } else {
        format!("{what} {}", place(ctx))
    }
}

/// Whether `action` is his line's row, or one that follows it.
fn is_lines(action: &ActionId, line: &ActionId) -> bool {
    action == line
        || super::resolve::FOLLOWS
            .iter()
            .any(|(follower, leader)| follower == action && leader == line)
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
