//! How a repository's run begins: [`Arrival`], and [`arrive`] seeding the
//! first state with it.

use std::sync::Arc;

use crate::update::AppState;

/// How a repository's run begins, beyond the repository itself.
#[derive(Debug, Default)]
pub(in crate::event_loop) struct Arrival {
    /// Put on the first frame's status row (`recent: not saved · …`).
    pub status: Option<String>,
    /// What the place before handed on, said once on the first frame: a
    /// clone's askpass close failure (m8/18b). `None` from every other
    /// arrival.
    pub notice: Option<String>,
    /// The cursor lands on `HEAD`'s row (`update::lands_at_open`), not the
    /// top row: every arrival on a terminal — launch, open, clone, init;
    /// off a tty (`run`) none.
    pub on_head: bool,
    /// The tour's way back (m8/19b): the commit to land on instead of
    /// `HEAD`. Read only with `on_head`.
    pub at: Option<mergewright_core::oid::Oid>,
    /// This place is the practice repository: the tour's card and ticks.
    pub practice: bool,
    /// The tour was finished in the place before: `✓ practice done`.
    pub tour_done: bool,
    /// The process's one keymap (`config::keymap_from`, ADR-0055).
    pub keymap: Arc<mergewright_ui::keymap::Keymap>,
    /// His config's problems in words — on the process's first repository
    /// screen only; empty on every later one.
    pub config_problems: Vec<String>,
    /// The first-launch line as this place arrives (m8/18d, 18e): `drive`'s
    /// switch read now (`FirstLaunch::line`), so a line ended in an earlier
    /// place stays gone. `None` by default (`run`, the gallery, the walks).
    pub first_launch: Option<std::path::PathBuf>,
    /// The graph mode a walk starts in (`[graph] merged_branches`).
    pub graph_mode: crate::config::MergedBranches,
}

impl Arrival {
    /// The first repository screen of a process started on `config`: its
    /// keymap, and every problem with the file — the file's and the keymap's
    /// (`config::keymap_from`), then each contrast floor his `[theme.*]`
    /// colours miss (none under `NO_COLOR`, where no colour is drawn),
    /// then a `graph.merged_branches` it does not take (ease-pass 06h).
    pub(in crate::event_loop) fn first(config: &crate::config::Loaded, no_color: bool) -> Self {
        let (keymap, mut config_problems) = crate::config::keymap_from(config);
        config_problems.extend(crate::config::theme_problems(
            &config.config.theme,
            no_color,
        ));
        let raw = config.config.graph.merged_branches.as_ref();
        let graph_mode = match crate::config::MergedBranches::from_config(raw.map(|raw| &raw.0)) {
            Ok(mode) => mode,
            Err(problem) => {
                config_problems.push(problem);
                crate::config::MergedBranches::Folded
            }
        };
        Self {
            keymap: Arc::new(keymap),
            config_problems,
            graph_mode,
            ..Self::default()
        }
    }
}

/// Seed a repository's first state with how it was arrived at: the keys,
/// his config's problems, the first-launch line, what to say on the first
/// frame (a notice the last place handed on, then the arrival's own status,
/// one sticky line), and the cursor on `HEAD`.
pub(in crate::event_loop) fn arrive(state: &mut AppState, arrival: Arrival) {
    state.seed_keys(arrival.keymap, arrival.config_problems);
    state.graph_mode = arrival.graph_mode;
    state.first_launch = arrival.first_launch.map(Box::new);
    if let Some(reason) = super::super::places::joined(arrival.notice, arrival.status) {
        state.status = crate::update::Status::Sticky { reason, keys: None };
    } else if arrival.tour_done {
        let again = state.keymap.key_verb(
            &[mergewright_ui::keymap::ActionId("app.palette")],
            "tour goes again",
        );
        state.status = match again {
            Some(keys) => crate::update::Status::done_with_keys("practice done", keys),
            None => crate::update::Status::done("practice done"),
        };
    }
    if arrival.on_head {
        crate::update::lands_at_open(state, arrival.at);
    }
    if arrival.practice {
        state.tour = Some(Box::new(crate::update::tour::TourState::arrived()));
    }
}
