//! App rows that are not key-table actions: the themes, the tour, and a
//! setting's choices — `GRAPH MODE` (ease-pass 06i), two keyless rows whose
//! `⏎` picks the mode and keeps it in his config file.

use mergewright_term::sanitize_line;
use mergewright_ui::palette::{ConfigId, PaletteRow, RowKind};
use mergewright_ui::theme::ThemeKind;

use super::{Entry, PaletteTarget};
use crate::config::MergedBranches;
use crate::update::state::Status;

/// What an app row does: not a key-table action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    /// `Theme: dark` / `light` / `high contrast`, for this run only.
    Theme(ThemeKind),
    /// `Practice tour` (m8/19b): from a repository, into the practice
    /// tour, coming back to the commit under the cursor. Outside one, the
    /// empty state's own `empty_state.tour` row is listed instead.
    Tour,
    /// A `GRAPH MODE` row (ease-pass 06i): switch to this mode and keep it.
    GraphMode(MergedBranches),
}

/// One row of a setting group the `?` list offers.
pub(crate) struct Choice {
    pub value: MergedBranches,
    pub title: &'static str,
    pub detail: &'static str,
}

/// The `GRAPH MODE` group's header.
pub(crate) const GRAPH_MODE_HEADER: &str = "GRAPH MODE";

/// The graph mode's choices, in the order listed (the mock's).
pub(crate) const GRAPH_MODE: [Choice; 2] = [
    Choice {
        value: MergedBranches::Folded,
        title: "Folded merges",
        detail: "merged branches fold into their merge row",
    },
    Choice {
        value: MergedBranches::Shown,
        title: "Every branch",
        detail: "every branch on its own lane, as before",
    },
];

/// The words that find the group besides its titles (the mock's).
pub(crate) const GRAPH_MODE_WORDS: &[&str] = &[
    "graph", "mode", "view", "fold", "folded", "merges", "merged", "every", "branch", "all",
    "lanes",
];

/// The `GRAPH MODE` rows, the one in force (`now`) marked on.
pub(super) fn graph_mode_rows(now: MergedBranches) -> Vec<Entry> {
    GRAPH_MODE
        .iter()
        .map(|choice| Entry {
            row: PaletteRow {
                label: sanitize_line(choice.title),
                key: None,
                config: Some(ConfigId::Key(MergedBranches::KEY)),
                detail: Some(choice.detail),
                kind: RowKind::Setting {
                    on: choice.value == now,
                },
            },
            target: PaletteTarget::App(AppAction::GraphMode(choice.value)),
            search: GRAPH_MODE_WORDS,
        })
        .collect()
}

/// The bar once the loop has kept the mode, or could not (06i): the switch
/// stands either way; only the keep failed.
pub(crate) fn graph_mode_kept(
    state: &mut crate::update::AppState,
    mode: MergedBranches,
    result: Result<(), String>,
) {
    let named = match mode {
        MergedBranches::Folded => "folded merges",
        MergedBranches::Shown => "every branch",
    };
    state.status = match result {
        Ok(()) => Status::done(format!("Graph: {named} · kept for next launch")),
        Err(why) => Status::Failed(format!("Graph: {named} for this run · not kept: {why}")),
    };
}
