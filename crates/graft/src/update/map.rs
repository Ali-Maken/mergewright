//! The Map view's own state and keys (`M`, DESIGN.md §5.1, m2-details-and-
//! diffs/18): built fresh on `M`, thrown away on `M`/`Esc` (`map.rs`'s own
//! doc comment — cheap enough to redo every time). No drawing here —
//! `view::map` (`map-view-screen`, a later ticket) draws whatever
//! [`MapState::data`] holds.

use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;

use crate::map::{MapData, MapRequest};

use super::detail::Loading;
use super::events::Command;
use super::state::AppState;

/// Columns a row's label, tip date and counts need beside its sparkline —
/// the rest of the terminal's width goes to buckets (this ticket's own
/// sizing rule: `max(10, columns - 24)`).
const MARGIN_COLUMNS: u16 = 24;
/// Fewest buckets a row is ever asked to fill, however narrow the terminal.
const MIN_BUCKETS: usize = 10;

/// The Map view: hidden until `M`, discarded on close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapState {
    /// The view is on screen.
    pub shown: bool,
    /// The build in flight, or its answer.
    pub data: Loading<Arc<MapData>>,
    /// `(done, total)` rows, from the job's own progress reports.
    pub progress: Option<(u32, u32)>,
    /// Which row is under the cursor.
    pub selected: usize,
}

impl Default for MapState {
    fn default() -> Self {
        MapState {
            shown: false,
            data: Loading::Idle,
            progress: None,
            selected: 0,
        }
    }
}

impl MapState {
    /// Show the view and ask for its data, under an id `next_id` hands out.
    pub fn open(&mut self, req: MapRequest, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        self.shown = true;
        self.selected = 0;
        self.progress = None;
        let id = next_id();
        self.data = Loading::Pending(id);
        vec![Command::BuildMap(id, req)]
    }

    /// Hide the view and cancel whatever build was running; the data is
    /// discarded either way (DESIGN.md §5.1: built fresh, thrown away).
    pub fn close(&mut self) -> Vec<Command> {
        self.shown = false;
        self.selected = 0;
        self.progress = None;
        let commands = match self.data {
            Loading::Pending(id) => vec![Command::CancelJob(id)],
            Loading::Idle | Loading::Ready(_) | Loading::Failed(_) => Vec::new(),
        };
        self.data = Loading::Idle;
        commands
    }

    /// The tip of the row under the cursor, once the data is in.
    #[must_use]
    pub fn selected_tip(&self) -> Option<Oid> {
        let Loading::Ready(data) = &self.data else {
            return None;
        };
        data.branches.get(self.selected).map(|branch| branch.tip)
    }

    /// The build finished, or failed, or a stale answer arrived for a job
    /// already superseded.
    fn produced(&mut self, id: JobId, result: Result<Arc<MapData>, String>) {
        if self.data != Loading::Pending(id) {
            return;
        }
        self.data = match result {
            Ok(data) => Loading::Ready(data),
            Err(message) => Loading::Failed(message),
        };
    }

    /// The build finished another row.
    fn progressed(&mut self, id: JobId, done: u32, total: u32) {
        if self.data == Loading::Pending(id) {
            self.progress = Some((done, total));
        }
    }
}

/// Buckets a row's sparkline gets at this terminal width (this ticket's own
/// rule: `max(10, columns - 24)`).
#[must_use]
fn buckets_for(columns: u16) -> usize {
    usize::from(columns.saturating_sub(MARGIN_COLUMNS)).max(MIN_BUCKETS)
}

/// What `M` asks the job for: this terminal's width, the wall clock the rest
/// of the run reads dates against, the Mistake Radar's policy, and its
/// context's previous default-branch tips — empty until that context has
/// landed, so the Map view marks nothing backward before then rather than
/// waiting on it (this ticket's own rule).
fn request(state: &AppState) -> MapRequest {
    let previous_default_tips = match &state.radar.context {
        Loading::Ready(context) => context.previous_default_tips.clone(),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => Vec::new(),
    };
    MapRequest {
        buckets: buckets_for(state.size.columns),
        clock: state.harness.clock,
        policy: state.radar.policy.clone(),
        previous_default_tips,
    }
}

/// `M` (opens or closes) and, while shown, the view's own movement and
/// `Enter`/`Esc`. Returns whether the key was this view's to answer.
///
/// `M` opens only on the graph screen, with no search prompt open (the
/// prompt already claims every key before this is reached, `core_event`) and
/// the detail pane's focus on the graph (`Files`/`Diff` keep their own
/// keys — ADR-0023's precedence, mirrored here).
pub(super) fn map_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    if !plain {
        return false;
    }
    if next.map.shown {
        return shown_key(next, commands, key);
    }
    if key.code == KeyCode::Char('M') && next.graph_has_focus() {
        let req = request(next);
        let AppState {
            map, local_jobs, ..
        } = next;
        commands.extend(map.open(req, &mut || local_jobs.next_id()));
        return true;
    }
    false
}

/// The view's keys while it is shown: `M`/`Esc` close it, `↑`/`↓`/`j`/`k`
/// move the selection, `Enter` seeks the graph to the selected row's tip.
fn shown_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    match key.code {
        KeyCode::Char('M') | KeyCode::Escape => {
            commands.extend(next.map.close());
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            next.map.selected = next.map.selected.saturating_sub(1);
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let last = row_count(next).saturating_sub(1);
            next.map.selected = (next.map.selected + 1).min(last);
            true
        }
        KeyCode::Enter => {
            seek_selected(next, commands);
            true
        }
        _ => true,
    }
}

/// Rows on hand right now, for clamping the selection.
fn row_count(next: &AppState) -> usize {
    match &next.map.data {
        Loading::Ready(data) => data.branches.len(),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => 0,
    }
}

/// `Enter` on a row: close the view and seek the graph to that branch's tip
/// (the same jump a search hit makes, `History::seek`).
fn seek_selected(next: &mut AppState, commands: &mut Vec<Command>) {
    let Some(tip) = next.map.selected_tip() else {
        commands.extend(next.map.close());
        return;
    };
    commands.extend(next.map.close());
    seek_to(next, commands, tip);
}

/// Seek the graph cursor to `tip`: at once when its row is loaded, else by
/// asking the walk for rows around it (`History::seek`). The Map's `Enter`
/// and the branch picker's `Tab` (m7-power-tools/10) both land this way.
pub(super) fn seek_to(next: &mut AppState, commands: &mut Vec<Command>, tip: Oid) {
    next.on_working_tree = false;
    let height = next.graph_rows();
    let Some(history) = next.history.as_mut() else {
        return;
    };
    let request = history.seek(tip, height);
    if let (Some(request), Some(walk)) = (request, next.awaiting) {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// The Map view's build answered, or reported another row done. `answer`
/// routes only [`super::events::Product::Map`] and
/// [`super::events::Progress::Map`] here.
pub(super) fn map_event(next: &mut AppState, event: super::events::AppEvent) {
    match event {
        super::events::AppEvent::Produced(id, super::events::Product::Map(result)) => {
            next.map.produced(id, result);
        }
        super::events::AppEvent::Progress(id, super::events::Progress::Map { done, total }) => {
            next.map.progressed(id, done, total);
        }
        _ => {}
    }
}

/// A resize while the view is shown: its sparkline width changed, so cancel
/// whatever build was running and ask again at the new width.
pub(super) fn resized(next: &mut AppState, commands: &mut Vec<Command>) {
    if !next.map.shown {
        return;
    }
    let req = request(next);
    let AppState {
        map, local_jobs, ..
    } = next;
    commands.extend(map.close());
    commands.extend(map.open(req, &mut || local_jobs.next_id()));
}

#[cfg(test)]
mod tests {
    use graft_ops::mistake_radar::Policy;
    use graft_ui::detail::DetailFocus;

    use super::*;
    use crate::map::{MapBranch, MapData};
    use graft_core::ref_name::RefName;
    use graft_graph::identity::BranchIdentity;

    fn key(code: KeyCode) -> KeyPress {
        KeyPress::plain(code)
    }

    fn state_with_columns(columns: u16) -> AppState {
        let mut state = AppState::default();
        state.size.columns = columns;
        state.size.rows = 40;
        state
    }

    fn sample_data(rows: usize) -> Arc<MapData> {
        let branches = (0..rows)
            .map(|i| MapBranch {
                name: RefName::new(format!("refs/heads/b{i}").into_bytes()).unwrap(),
                identity: BranchIdentity::of_name(format!("b{i}").as_bytes()),
                tip: Oid::from_hex(&"1".repeat(40)).unwrap(),
                tip_date: 0,
                counts: vec![0; 10],
                is_current: i == 0,
                is_default: i == 0,
                merged_into: None,
                marks: Vec::new(),
                complete: true,
            })
            .collect();
        Arc::new(MapData {
            branches,
            more: 0,
            from: 0,
            to: 1,
            buckets: 10,
            complete: true,
        })
    }

    #[test]
    fn m_opens_the_view_with_one_build_map_command() {
        let mut state = state_with_columns(100);
        let mut commands = Vec::new();
        assert!(map_key(&mut state, &mut commands, key(KeyCode::Char('M'))));
        assert!(state.map.shown);
        match commands.as_slice() {
            [Command::BuildMap(_, req)] => {
                assert_eq!(req.buckets, 100 - 24);
                assert_eq!(req.policy, Policy::default());
                assert!(req.previous_default_tips.is_empty());
            }
            other => panic!("expected one BuildMap command, got {other:?}"),
        }
    }

    #[test]
    fn narrow_terminals_never_ask_for_fewer_than_ten_buckets() {
        let state = state_with_columns(5);
        assert_eq!(request(&state).buckets, MIN_BUCKETS);
    }

    #[test]
    fn m_again_or_escape_closes_and_cancels_a_pending_build() {
        let mut state = state_with_columns(100);
        let mut commands = Vec::new();
        map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
        let Loading::Pending(id) = state.map.data else {
            panic!("expected a pending build")
        };
        commands.clear();
        assert!(map_key(&mut state, &mut commands, key(KeyCode::Escape)));
        assert!(!state.map.shown);
        assert_eq!(state.map.data, Loading::Idle);
        assert_eq!(commands, vec![Command::CancelJob(id)]);
    }

    #[test]
    fn a_product_lands_as_ready_and_a_stale_one_is_dropped() {
        let mut state = state_with_columns(100);
        let mut commands = Vec::new();
        map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
        let Loading::Pending(id) = state.map.data else {
            panic!("expected a pending build")
        };
        let data = sample_data(2);
        map_event(
            &mut state,
            super::super::events::AppEvent::Produced(
                id,
                super::super::events::Product::Map(Ok(data.clone())),
            ),
        );
        assert_eq!(state.map.data, Loading::Ready(data.clone()));

        // A stale id (from a build already superseded) changes nothing.
        let other = super::super::state::AppState::default()
            .local_jobs
            .next_id();
        map_event(
            &mut state,
            super::super::events::AppEvent::Produced(
                other,
                super::super::events::Product::Map(Err("stale".to_owned())),
            ),
        );
        assert_eq!(state.map.data, Loading::Ready(data));
    }

    #[test]
    fn enter_on_a_row_closes_the_view_and_seeks_its_tip() {
        let mut state = state_with_columns(100);
        state.history = Some(crate::history::History::started());
        let mut commands = Vec::new();
        map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
        let Loading::Pending(id) = state.map.data else {
            panic!("expected a pending build")
        };
        state.map.data = Loading::Ready(sample_data(2));
        state.awaiting = Some(id);
        commands.clear();
        assert!(map_key(&mut state, &mut commands, key(KeyCode::Enter)));
        assert!(!state.map.shown);
        assert_eq!(state.map.data, Loading::Idle);
    }

    #[test]
    fn resize_while_shown_cancels_and_rebuilds_at_the_new_width() {
        let mut state = state_with_columns(100);
        let mut commands = Vec::new();
        map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
        let Loading::Pending(old) = state.map.data else {
            panic!("expected a pending build")
        };
        state.size.columns = 60;
        commands.clear();
        resized(&mut state, &mut commands);
        match commands.as_slice() {
            [Command::CancelJob(cancelled), Command::BuildMap(_, req)] => {
                assert_eq!(*cancelled, old);
                assert_eq!(req.buckets, 60 - 24);
            }
            other => panic!("expected cancel + rebuild, got {other:?}"),
        }
    }

    #[test]
    fn nothing_happens_while_the_detail_pane_is_not_focused_on_the_graph() {
        let mut state = state_with_columns(100);
        state.detail.focus = DetailFocus::Files;
        let mut commands = Vec::new();
        assert!(!map_key(&mut state, &mut commands, key(KeyCode::Char('M'))));
        assert!(!state.map.shown);
    }
}
