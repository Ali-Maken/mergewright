//! The command palette and help (§2.12, ADR-0056; m8-polish-and-ship/11):
//! `?` opens it on this place, `⌃P`/`:` on every action. One state, one
//! listing, one filter; `graft_ui::palette` draws it.
//!
//! The listing reads the live key table (ADR-0055), so a row's key is what
//! pressing it does **in the place it was opened from** (decision 3): after
//! `merge = "j"`, `?` on the graph spells merge `j` and move `↓`. `⏎` runs a
//! row the way its key would: an `IN PROGRESS` row runs as the banner's
//! key (`banner_press`), any other through [`super::dispatch_key`], so an
//! action he unbound still runs and a refusal reads exactly as the key's
//! (decision 5).
//!
//! The palette is a text field (§2.1): every key but `Ctrl+C` is its own
//! while it is up, and the edge passes them untranslated
//! (`ContextId::Palette`). The empty state, which has no `AppState`, keeps
//! a [`PaletteState`] of its own and builds its rows through the same
//! [`listing`] (`crate::empty_state`).

use std::collections::VecDeque;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::switch_refs::SwitchRefs;
use graft_ops::matcher::fuzzy_score;
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::keymap::{
    ActionId, Binding, ContextId, KeyChord, KeyLabel, Keymap, Translated, defaults,
};
use graft_ui::palette::{PaletteMode, PaletteRow, PaletteView, RowKind, WIDE_FROM};
use graft_ui::text_input::TextField;
use graft_ui::theme::ThemeKind;

use super::{AppState, Command};
use crate::jobs::palette::{NameHits, NamesRequest, Ranked, RefHit};
use crate::search::CommitHit;
use graft_git::status::TrackedPaths;

mod find;
mod groups;
mod input;
mod listing;
mod run;

pub(super) use self::find::*;
#[allow(clippy::wildcard_imports)]
use self::groups::*;
pub(crate) use self::input::*;
pub(crate) use self::listing::*;
pub(crate) use self::run::first_runnable;
#[allow(clippy::wildcard_imports)]
use self::run::*;

/// The most distinct commands COMMANDS RUN RECENTLY keeps (ADR-0061).
pub const RECENT_CAP: usize = 8;
/// Rows a find group shows before its `+N more` row.
pub const GROUP_ROWS: usize = 8;
/// Rows a find group shows once `+N more` widened it.
pub const WIDE_ROWS: usize = 50;
/// How many commits COMMITS lists before `more · / searches all of history`
/// (12b): the commits job stops reading at this many.
pub const COMMIT_HITS: usize = 9;
/// The recent group's header, exactly as DESIGN §1.6 draws it (P-find-2).
pub const RECENT_HEADER: &str = "COMMANDS RUN RECENTLY";
/// What the empty `⌃P` field says, muted, until the first character
/// (P-find-1).
pub const FIND_PLACEHOLDER: &str = "type to find actions, branches, tags, files, commits";

/// The palette while it is up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaletteState {
    /// How it was opened.
    pub mode: PaletteMode,
    /// What he has typed.
    pub query: TextField,
    /// The row under the cursor, over every row with groups flattened.
    pub cursor: usize,
    /// The place it was opened on: what its rows' keys are spelled for,
    /// and where `⏎` runs them. Never `Palette`.
    pub opened_from: ContextId,
    /// The last click on a row and when, in the clock's milliseconds: a
    /// second click on the same row within the picker's double-click rule
    /// runs it (m8/11b decision 6).
    pub last_click: Option<(usize, i64)>,
    /// What `⌃P` finds besides actions, and the jobs finding it (ticket
    /// 12). Default, and never asked, under `?`.
    pub find: Find,
}

/// A find group: what `+N more` widens (`more · /` for COMMITS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindGroup {
    /// BRANCHES.
    Branches,
    /// TAGS.
    Tags,
    /// FILES (12b).
    Files,
    /// COMMITS (12b): its `more` row hands the query to `/`.
    Commits,
}

impl FindGroup {
    /// Its header, as drawn.
    fn header(self) -> &'static str {
        match self {
            Self::Branches => "BRANCHES",
            Self::Tags => "TAGS",
            Self::Files => "FILES",
            Self::Commits => "COMMITS",
        }
    }
}

/// A read done once per `⌃P` open, cancelled on close.
#[derive(Clone, Debug, Default)]
pub(crate) enum ReadOnce<T> {
    /// Not asked: `?`, or no repository.
    #[default]
    NotAsked,
    /// In flight under this id.
    Reading(JobId),
    /// Landed.
    Read(Arc<T>),
    /// Failed, in its words.
    Failed(String),
}

/// By `Arc::ptr_eq` for `Read`: the read is one value per open, and a
/// derive would compare every listing on every state comparison (no
/// derives are added to graft-git's types).
impl<T> PartialEq for ReadOnce<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NotAsked, Self::NotAsked) => true,
            (Self::Reading(a), Self::Reading(b)) => a == b,
            (Self::Read(a), Self::Read(b)) => Arc::ptr_eq(a, b),
            (Self::Failed(a), Self::Failed(b)) => a == b,
            _ => false,
        }
    }
}

impl<T> Eq for ReadOnce<T> {}

/// A job asked per query: the one in flight and the last answer, each with
/// the query it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Asked<T> {
    /// In flight: its id and query.
    pub pending: Option<(JobId, String)>,
    /// The last answer and the query it answered.
    pub answer: Option<(String, T)>,
}

impl<T> Default for Asked<T> {
    fn default() -> Self {
        Self {
            pending: None,
            answer: None,
        }
    }
}

/// The palette's finds (ticket 12).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct Find {
    /// Every branch, read once per open.
    pub branches: ReadOnce<SwitchRefs>,
    /// Every tracked path, read once per open (12b).
    pub files: ReadOnce<TrackedPaths>,
    /// Branch, tag and path names matched against the query.
    pub names: Asked<Arc<NameHits>>,
    /// The first [`COMMIT_HITS`] commits matching the query, or why the
    /// walk failed (12b).
    pub commits: Asked<Result<Arc<[CommitHit]>, String>>,
    /// The group `+N more` widened; reset by a query edit.
    pub widened: Option<FindGroup>,
}

/// COMMANDS RUN RECENTLY: the last [`RECENT_CAP`] distinct actions and app
/// rows run from either palette mode this session, newest first. No
/// argument stored (P-find-2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RecentRuns(VecDeque<PaletteTarget>);

impl RecentRuns {
    /// `target` was run from the palette: to the front, once. Only actions
    /// and app rows are commands; a seek or a note is not kept.
    /// Kept with the context it ran in; one command is kept once whatever
    /// context it ran in ([`same_command`]).
    pub(crate) fn ran(&mut self, target: PaletteTarget) {
        if !matches!(target, PaletteTarget::Action { .. } | PaletteTarget::App(_)) {
            return;
        }
        self.0.retain(|kept| !same_command(*kept, target));
        self.0.push_front(target);
        self.0.truncate(RECENT_CAP);
    }

    /// Newest first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = PaletteTarget> + '_ {
        self.0.iter().copied()
    }
}

/// The same command, whatever context its row was spelled in: an action by
/// its id, anything else by equality. COMMANDS RUN RECENTLY keeps commands,
/// and draws each as ACTIONS spells it now (`recent_rows`).
fn same_command(a: PaletteTarget, b: PaletteTarget) -> bool {
    match (a, b) {
        (PaletteTarget::Action { id: a, .. }, PaletteTarget::Action { id: b, .. }) => a == b,
        (a, b) => a == b,
    }
}

/// No recent runs: the empty state's.
static NO_RECENT: RecentRuns = RecentRuns(VecDeque::new());

/// What an app row does: not a key-table action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    /// `Theme: dark` / `light` / `high contrast`, for this run only.
    Theme(ThemeKind),
    /// `Practice tour` (m8/19b): from a repository, into the practice
    /// tour, coming back to the commit under the cursor. Outside one, the
    /// empty state's own `empty_state.tour` row is listed instead.
    Tour,
}

/// What `⏎` on a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaletteTarget {
    /// Run this action as its default chord would **in `ctx`**, the
    /// context its row was spelled in (m8/12e): `Banner` for `IN PROGRESS`
    /// rows, else the place the palette was opened on.
    Action { id: ActionId, ctx: ContextId },
    /// An app row.
    App(AppAction),
    /// A `YOUR CONFIG` row: nothing to run; `⏎` closes. Also a find
    /// group's placeholder (`reading…`, `searching…`, a read failure, the
    /// last `+N more · type more to narrow`), drawn as [`RowKind::More`],
    /// where `⏎` does nothing.
    Note,
    /// A branch, tag (12) or commit (12b) row: put the graph cursor on it.
    Seek(Oid),
    /// A FILES row (12b): open that path's history place. An index into
    /// the open's [`TrackedPaths`], which is not re-read while it is up.
    File(u32),
    /// `+N more`: widen this group.
    More(FindGroup),
}

/// A group as drawn: its header and its rows, each with what it runs.
pub(crate) type Group = (SanitizedText, Vec<(PaletteRow, PaletteTarget)>);

/// A group's rows, each with what it runs.
type Rows = Vec<(PaletteRow, PaletteTarget)>;

/// What a listing is built from — the few things it reads, so the empty
/// state (no `AppState`) builds the same rows.
pub(crate) struct Source<'a> {
    /// The live key table.
    pub keymap: &'a Keymap,
    /// His config's problems, each in its words (`YOUR CONFIG`).
    pub problems: &'a [String],
    /// An operation is live: `IN PROGRESS` comes first under `?`.
    pub operation_live: bool,
    /// The live operation can skip a step: not a merge, which has no `s`
    /// (the banner's hint bar drops it too, `hints::catalogue`).
    pub skippable: bool,
    /// COMMANDS RUN RECENTLY, under `⌃P`.
    pub recent: &'a RecentRuns,
    /// There is a repository to find in: `⌃P` lists the find groups. The
    /// empty state has none.
    pub finds: bool,
    /// The refs job has answered: TAGS can be matched.
    pub tags_read: bool,
}

/// One row before filtering: its words to match on, besides the label.
#[derive(Clone)]
struct Entry {
    row: PaletteRow,
    target: PaletteTarget,
    search: &'static [&'static str],
}

/// The places whose rows are never listed: text fields and the confirm
/// box, whose keys are fixed and mean something only inside them
/// (decision 2).
const NEVER_LISTED: &[ContextId] = &[
    ContextId::Palette,
    ContextId::Picker,
    ContextId::Confirm,
    ContextId::Credential,
    ContextId::CommitEditor,
];

/// The three theme rows, in the order `⌃P` lists them.
const THEMES: [(ThemeKind, &str); 3] = [
    (ThemeKind::Dark, "Theme: dark"),
    (ThemeKind::Light, "Theme: light"),
    (ThemeKind::HighContrast, "Theme: high contrast"),
];

/// A key while the palette is up: `true` when the palette took it.
pub(crate) fn palette_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let Some(mut palette) = state.palette.take() else {
        return false;
    };
    let shown = rows(state, &palette);
    let before = palette.query.text().to_owned();
    let edited = edit(&mut palette, key, &shown, |p| rows(state, p));
    match edited {
        Edited::Stay => {
            if palette.query.text() != before {
                query_changed(state, commands, &mut palette);
            }
            state.palette = Some(palette);
        }
        Edited::Close => cancel_finds(&palette, commands),
        Edited::Pass => {
            state.palette = Some(palette);
            return false;
        }
        Edited::Run(PaletteTarget::More(group)) if group != FindGroup::Commits => {
            widen(state, &mut palette, group);
            state.palette = Some(palette);
        }
        Edited::Run(target) => {
            cancel_finds(&palette, commands);
            run(state, commands, &palette, target);
        }
    }
    true
}

/// The palette as the widget draws it.
pub(crate) fn view(state: &AppState, palette: &PaletteState) -> PaletteView {
    with_placeholder(
        view_of(
            rows(state, palette),
            palette,
            state.size.columns >= WIDE_FROM,
        ),
        palette,
    )
}

/// `view` with the empty field's words (P-find-1): under `⌃P` in a
/// repository, what the box finds, muted, until the first character.
fn with_placeholder(mut view: PaletteView, palette: &PaletteState) -> PaletteView {
    if palette.mode == PaletteMode::Everything && palette.query.text().is_empty() {
        view.placeholder = Some(FIND_PLACEHOLDER);
    }
    view
}

/// [`view`] from rows already listed.
pub(crate) fn view_of(groups: Vec<Group>, palette: &PaletteState, wide: bool) -> PaletteView {
    PaletteView {
        mode: palette.mode,
        query: sanitize_line(palette.query.text()),
        query_cursor: palette.query.cursor(),
        groups: groups
            .into_iter()
            .map(|(header, rows)| (header, rows.into_iter().map(|(row, _)| row).collect()))
            .collect(),
        cursor: palette.cursor,
        wide,
        placeholder: None,
    }
}

/// The bottom row while the palette is up: its own keys, spelled from the
/// shipped table — a text field's keys are fixed (decision 8, R4).
#[must_use]
pub(crate) fn bar() -> String {
    let shipped = Keymap::shipped();
    let arrows = format!(
        "{}{} move",
        KeyChord::plain(KeyCode::Up).label().0,
        KeyChord::plain(KeyCode::Down).label().0
    );
    let rest = shipped.key_verbs(&[("palette.run", "run"), ("app.back", "close")]);
    format!("{arrows}  {rest}")
}

#[cfg(test)]
mod tests;
