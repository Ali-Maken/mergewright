//! The command palette and help (§2.12, ADR-0056; m8-polish-and-ship/11):
//! `?` opens it on this place, `⌃P`/`:` on every action. One state, one
//! listing, one filter; `graft_ui::palette` draws it.
//!
//! The listing reads the live key table (ADR-0055), so a row's key is what
//! pressing it does **in the place it was opened from** (decision 3): after
//! `merge = "j"`, `?` on the graph spells merge `j` and move `↓`. `⏎` runs a
//! row the way its key would — the default chord, straight to
//! [`super::dispatch_key`], so an action he unbound still runs and a
//! refusal reads exactly as the key's (decision 5).
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
use graft_core::matcher::fuzzy_score;
use graft_core::oid::Oid;
use graft_git::switch_refs::SwitchRefs;
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::keymap::{
    ActionId, Binding, ContextId, KeyChord, KeyLabel, Keymap, Translated, defaults,
};
use graft_ui::palette::{PaletteMode, PaletteRow, PaletteView, RowKind, WIDE_FROM};
use graft_ui::text_input::TextField;
use graft_ui::theme::ThemeKind;

use super::{AppState, Command};
use crate::jobs::palette::{NameHits, NamesRequest, Ranked, RefHit};

/// The most distinct commands COMMANDS RUN RECENTLY keeps (ADR-0061).
pub const RECENT_CAP: usize = 8;
/// Rows a find group shows before its `+N more` row.
pub const GROUP_ROWS: usize = 8;
/// Rows a find group shows once `+N more` widened it.
pub const WIDE_ROWS: usize = 50;
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

/// A find group: what `+N more` widens. 12b adds `Files` and `Commits`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindGroup {
    /// BRANCHES.
    Branches,
    /// TAGS.
    Tags,
}

impl FindGroup {
    /// Its header, as drawn.
    fn header(self) -> &'static str {
        match self {
            Self::Branches => "BRANCHES",
            Self::Tags => "TAGS",
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
    /// Branch and tag names matched against the query.
    pub names: Asked<Arc<NameHits>>,
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
    pub(crate) fn ran(&mut self, target: PaletteTarget) {
        if !matches!(target, PaletteTarget::Action(_) | PaletteTarget::App(_)) {
            return;
        }
        self.0.retain(|kept| *kept != target);
        self.0.push_front(target);
        self.0.truncate(RECENT_CAP);
    }

    /// Newest first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = PaletteTarget> + '_ {
        self.0.iter().copied()
    }
}

/// No recent runs: the empty state's.
static NO_RECENT: RecentRuns = RecentRuns(VecDeque::new());

/// What an app row does: not a key-table action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppAction {
    /// `Theme: dark` / `light` / `high contrast`, for this run only.
    Theme(ThemeKind),
}

/// What `⏎` on a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaletteTarget {
    /// Run this action as its default chord would.
    Action(ActionId),
    /// An app row.
    App(AppAction),
    /// A `YOUR CONFIG` row: nothing to run; `⏎` closes. Also a find
    /// group's placeholder (`reading…`, `searching…`, a read failure, the
    /// last `+N more · type more to narrow`), drawn as [`RowKind::More`],
    /// where `⏎` does nothing.
    Note,
    /// A branch, tag (12) or commit (12b) row: put the graph cursor on it.
    Seek(Oid),
    /// `+N more`: widen this group.
    More(FindGroup),
}

/// A group as drawn: its header and its rows, each with what it runs.
pub(crate) type Group = (SanitizedText, Vec<(PaletteRow, PaletteTarget)>);

/// What a listing is built from — the few things it reads, so the empty
/// state (no `AppState`) builds the same rows.
pub(crate) struct Source<'a> {
    /// The live key table.
    pub keymap: &'a Keymap,
    /// His config's problems, each in its words (`YOUR CONFIG`).
    pub problems: &'a [String],
    /// An operation is live: `IN PROGRESS` comes first under `?`.
    pub operation_live: bool,
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

/// Which way `key` (a default chord, after the edge) opens the palette:
/// `?` on this place, `⌃P`/`:` on everything. `None` for any other key.
pub(crate) fn opener(key: KeyPress) -> Option<Opens> {
    let chord = KeyChord::from_press(&key)?;
    let holds = |id: &str| {
        defaults()
            .iter()
            .any(|b| b.action.0 == id && b.default.contains(&chord))
    };
    if holds("app.help") {
        Some(Opens::Here)
    } else if holds("app.palette") {
        Some(Opens::Everything)
    } else {
        None
    }
}

/// Which way a key opens the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opens {
    /// `?`.
    Here,
    /// `⌃P`, `:`.
    Everything,
}

/// A fresh palette opened on `opened_from`, its cursor on the first row
/// that runs.
pub(crate) fn opened(source: &Source<'_>, opened_from: ContextId, how: Opens) -> PaletteState {
    let mode = match how {
        Opens::Here => PaletteMode::Here(opened_from),
        Opens::Everything => PaletteMode::Everything,
    };
    let mut palette = PaletteState {
        mode,
        query: TextField::new(""),
        cursor: 0,
        opened_from,
        last_click: None,
        find: Find::default(),
    };
    palette.cursor = first_runnable(&listing(source, &palette));
    palette
}

/// `?`, `⌃P` or `:` reached the dispatchers: open the palette on the place
/// the key would have gone to, unless a text field is open (it types the
/// key, and never reaches here) or a confirm box is up (it swallows every
/// key but its own, §2.3c). `true` when it opened.
pub(crate) fn open_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let Some(how) = opener(key) else {
        return false;
    };
    if state.palette.is_some() || state.text_field_open() || state.confirm.is_some() {
        return false;
    }
    open(state, commands, how);
    true
}

/// Open the palette over whatever is on screen. `⌃P`/`:` read the
/// branches once, for BRANCHES (ticket 12); `?` reads nothing.
pub(crate) fn open(state: &mut AppState, commands: &mut Vec<Command>, how: Opens) {
    let opened_from = super::edge::context_of(state);
    let mut palette = opened(&source_of(state), opened_from, how);
    if how == Opens::Everything {
        let id = next_id(state);
        palette.find.branches = ReadOnce::Reading(id);
        commands.push(Command::ReadPaletteBranches(id));
    }
    state.palette = Some(Box::new(palette));
}

fn next_id(state: &mut AppState) -> JobId {
    let mut local_jobs = state.local_jobs;
    let id = local_jobs.next_id();
    state.local_jobs = local_jobs;
    id
}

fn source_of(state: &AppState) -> Source<'_> {
    Source {
        keymap: &state.keymap,
        problems: &state.config_problems,
        operation_live: super::banner::live(state).is_some(),
        recent: &state.palette_recent,
        finds: true,
        tags_read: state.refs.is_some(),
    }
}

/// What the empty state lists from: no repository, so no finds and no
/// recent runs (it has no `AppState` to keep them in).
pub(crate) fn outside_source<'a>(keymap: &'a Keymap, problems: &'a [String]) -> Source<'a> {
    Source {
        keymap,
        problems,
        operation_live: false,
        recent: &NO_RECENT,
        finds: false,
        tags_read: false,
    }
}

/// The palette's rows as drawn: groups in order, filtered by the query.
pub(crate) fn rows(state: &AppState, palette: &PaletteState) -> Vec<Group> {
    listing(&source_of(state), palette)
}

/// [`rows`] from a [`Source`]: the groups of decision 2, filtered and
/// ordered by decision 4.
/// Under `⌃P` with something typed, the find groups come after ACTIONS and
/// before COMMANDS RUN RECENTLY (§2.12, ticket 12).
pub(crate) fn listing(source: &Source<'_>, palette: &PaletteState) -> Vec<Group> {
    let groups = groups(source, palette);
    let mut out = filter(
        groups,
        palette.query.text(),
        source.keymap,
        palette.opened_from,
    );
    let query = palette.query.text().trim();
    if palette.mode == PaletteMode::Everything && source.finds && !query.is_empty() {
        let at = out
            .iter()
            .position(|(header, _)| header.as_str() == RECENT_HEADER)
            .unwrap_or(out.len());
        let finds = find_groups(&palette.find, query, source.tags_read);
        out.splice(at..at, finds);
    }
    out
}

/// BRANCHES and TAGS for `query` (ticket 12). A group not read yet says
/// `reading…`; one whose answer is for an older query shows that answer's
/// rows the query still matches, then `searching…`, so every row shown
/// matches what is typed; a current answer shows its best [`GROUP_ROWS`]
/// (or [`WIDE_ROWS`] once widened) and `+N more`. A group with no row is
/// left out.
fn find_groups(find: &Find, query: &str, tags_read: bool) -> Vec<Group> {
    let mut out = Vec::new();
    for (group, kind) in [
        (FindGroup::Branches, RowKind::Branch),
        (FindGroup::Tags, RowKind::Tag),
    ] {
        let read = match group {
            FindGroup::Branches => match &find.branches {
                ReadOnce::NotAsked | ReadOnce::Reading(_) => false,
                ReadOnce::Read(_) => true,
                ReadOnce::Failed(why) => {
                    // One line, as the `C` picker words its failure.
                    let why: Vec<&str> = why
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect();
                    let words = format!("branches: {}", why.join(" "));
                    out.push((sanitize_line(group.header()), vec![placeholder(&words)]));
                    continue;
                }
            },
            FindGroup::Tags => tags_read,
        };
        let rows = if read {
            group_rows(find, group, kind, query)
        } else {
            vec![placeholder("reading…")]
        };
        if !rows.is_empty() {
            out.push((sanitize_line(group.header()), rows));
        }
    }
    out
}

/// One read group's rows from the names answer: current, or stale and
/// re-filtered with `searching…` after.
fn group_rows(
    find: &Find,
    group: FindGroup,
    kind: RowKind,
    query: &str,
) -> Vec<(PaletteRow, PaletteTarget)> {
    let ranked = find.names.answer.as_ref().and_then(|(asked, hits)| {
        let ranked = match group {
            FindGroup::Branches => hits.branches.as_ref(),
            FindGroup::Tags => hits.tags.as_ref(),
        };
        ranked.map(|ranked| (asked.as_str(), ranked))
    });
    let cap = if find.widened == Some(group) {
        WIDE_ROWS
    } else {
        GROUP_ROWS
    };
    match ranked {
        Some((asked, ranked)) if asked == query => current_rows(ranked, group, kind, cap),
        stale => {
            let mut rows: Vec<_> = stale
                .map_or(&[][..], |(_, ranked)| ranked.top.as_slice())
                .iter()
                .filter(|hit| fuzzy_score(query, &String::from_utf8_lossy(&hit.name)).is_some())
                .take(cap)
                .map(|hit| hit_row(hit, kind))
                .collect();
            rows.push(placeholder("searching…"));
            rows
        }
    }
}

/// A current answer's rows: the best `cap`, then `+N more` (widens) or,
/// once widened, `+N more · type more to narrow` (a note).
fn current_rows(
    ranked: &Ranked<RefHit>,
    group: FindGroup,
    kind: RowKind,
    cap: usize,
) -> Vec<(PaletteRow, PaletteTarget)> {
    let mut rows: Vec<_> = ranked
        .top
        .iter()
        .take(cap)
        .map(|hit| hit_row(hit, kind))
        .collect();
    let more = ranked.total.saturating_sub(rows.len());
    if more > 0 {
        if cap >= WIDE_ROWS {
            rows.push(placeholder(&format!("+{more} more · type more to narrow")));
        } else {
            let (row, _) = placeholder(&format!("+{more} more"));
            rows.push((row, PaletteTarget::More(group)));
        }
    }
    rows
}

fn hit_row(hit: &RefHit, kind: RowKind) -> (PaletteRow, PaletteTarget) {
    (
        PaletteRow {
            label: sanitize_line(&String::from_utf8_lossy(&hit.name)),
            key: None,
            config: None,
            kind,
        },
        PaletteTarget::Seek(hit.tip),
    )
}

/// A muted row `⏎` does nothing on: [`RowKind::More`], a note.
fn placeholder(words: &str) -> (PaletteRow, PaletteTarget) {
    (
        PaletteRow {
            label: sanitize_line(words),
            key: None,
            config: None,
            kind: RowKind::More,
        },
        PaletteTarget::Note,
    )
}

/// A placeholder row: muted, `⏎` does nothing.
fn is_placeholder(row: &PaletteRow, target: PaletteTarget) -> bool {
    row.kind == RowKind::More && target == PaletteTarget::Note
}

/// Every group before the query: `YOUR CONFIG` first when there are
/// problems; then, under `?`, `IN PROGRESS`, `HERE`, `EVERYWHERE`; under
/// `⌃P`, one `ACTIONS` group and the app rows.
fn groups(source: &Source<'_>, palette: &PaletteState) -> Vec<(&'static str, Vec<Entry>)> {
    let keymap = source.keymap;
    let at = palette.opened_from;
    let mut out = Vec::new();
    if !source.problems.is_empty() {
        let notes = source
            .problems
            .iter()
            .map(|words| Entry {
                row: PaletteRow {
                    label: sanitize_line(words),
                    key: None,
                    config: None,
                    kind: RowKind::App,
                },
                target: PaletteTarget::Note,
                search: &[],
            })
            .collect();
        out.push(("YOUR CONFIG", notes));
    }
    let in_progress: Vec<Entry> = if source.operation_live {
        // Spelled where the operation takes them: the banner's own keys.
        keymap
            .in_context(ContextId::Banner)
            .into_iter()
            .filter(|(b, _)| listed(b))
            .filter_map(|(b, _)| key_in(keymap, ContextId::Banner, b).map(|key| action(b, key)))
            .collect()
    } else {
        Vec::new()
    };
    let taken: Vec<ActionId> = in_progress.iter().filter_map(action_of).collect();
    let here = place_rows(keymap, at, false, &taken);
    let everywhere = place_rows(keymap, at, true, &taken);
    match palette.mode {
        PaletteMode::Here(_) => {
            out.push(("IN PROGRESS", in_progress));
            out.push(("HERE", here));
            out.push(("EVERYWHERE", everywhere));
        }
        PaletteMode::Everything => {
            let actions = everything(in_progress, here, everywhere);
            let recent = recent_rows(source.recent, &actions);
            // Empty box: recent first; typed: last (ADR-0061).
            if palette.query.text().trim().is_empty() {
                out.push((RECENT_HEADER, recent));
                out.push(("ACTIONS", actions));
            } else {
                out.push(("ACTIONS", actions));
                out.push((RECENT_HEADER, recent));
            }
        }
    }
    out
}

/// COMMANDS RUN RECENTLY: the action row as ACTIONS draws it, for each
/// recent run still listed (ADR-0061), newest first.
fn recent_rows(recent: &RecentRuns, actions: &[Entry]) -> Vec<Entry> {
    recent
        .iter()
        .filter_map(|target| actions.iter().find(|e| e.target == target))
        .map(|entry| Entry {
            row: PaletteRow {
                kind: RowKind::Recent,
                ..entry.row.clone()
            },
            ..entry.clone()
        })
        .collect()
}

/// `ACTIONS`: every row of the three groups once, then the app rows.
fn everything(in_progress: Vec<Entry>, here: Vec<Entry>, everywhere: Vec<Entry>) -> Vec<Entry> {
    let mut actions: Vec<Entry> = Vec::new();
    for entry in in_progress.into_iter().chain(here).chain(everywhere) {
        let id = action_of(&entry);
        if !actions.iter().any(|kept| action_of(kept) == id) {
            actions.push(entry);
        }
    }
    actions.extend(THEMES.iter().map(|(kind, label)| Entry {
        row: PaletteRow {
            label: sanitize_line(label),
            key: None,
            config: None,
            kind: RowKind::App,
        },
        target: PaletteTarget::App(AppAction::Theme(*kind)),
        search: &[],
    }));
    actions
}

/// `HERE` (`global` false: rows not bound in `Global`) or `EVERYWHERE`
/// (`global` true): first each row a key reaches in `at`, in resolution
/// order, spelled with that key; then the place's own rows (`at`'s, or
/// `Global`'s) no key reaches there, spelled `—`. Less `taken` (the
/// `IN PROGRESS` rows).
fn place_rows(keymap: &Keymap, at: ContextId, global: bool, taken: &[ActionId]) -> Vec<Entry> {
    let wanted = |b: &Binding| {
        listed(b) && (b.context == ContextId::Global) == global && !taken.contains(&b.action)
    };
    let mut out: Vec<Entry> = keymap
        .reachable(at)
        .into_iter()
        .filter(|(b, _)| wanted(b))
        .filter_map(|(b, _)| key_in(keymap, at, b).map(|key| action(b, key)))
        .collect();
    let own = if global { ContextId::Global } else { at };
    for (b, _) in keymap.in_context(own) {
        if wanted(b) && key_in(keymap, at, b).is_none() {
            out.push(action(b, KeyLabel("—".to_owned())));
        }
    }
    out
}

/// Whether a row can be listed at all: something can run it, and it is not
/// a text field's or the confirm box's.
fn listed(binding: &Binding) -> bool {
    !binding.default.is_empty() && !NEVER_LISTED.contains(&binding.context)
}

/// The key that does `binding` when pressed in `at`, his lines applied:
/// the first of its chords that acts as it there and is not a key nobody
/// answers. `None` when no chord of his does it there.
fn key_in(keymap: &Keymap, at: ContextId, binding: &Binding) -> Option<KeyLabel> {
    keymap
        .chords(&binding.action)
        .iter()
        .find(|chord| {
            keymap.acting(at, **chord) == Some(binding.action)
                && keymap.translate(at, **chord) != Translated::Unbound
        })
        .map(KeyChord::label)
}

fn action(binding: &Binding, key: KeyLabel) -> Entry {
    Entry {
        row: PaletteRow {
            label: sanitize_line(binding.label),
            key: Some(key),
            config: Some(binding.action),
            kind: RowKind::Action,
        },
        target: PaletteTarget::Action(binding.action),
        search: binding.search,
    }
}

fn action_of(entry: &Entry) -> Option<ActionId> {
    match entry.target {
        PaletteTarget::Action(id) => Some(id),
        PaletteTarget::App(_)
        | PaletteTarget::Note
        | PaletteTarget::Seek(_)
        | PaletteTarget::More(_) => None,
    }
}

/// Decision 4. Empty query: every group with rows. Typed: each row scored
/// by its label and search words (`graft_core::matcher`), groups in their
/// order, rows by score then table order, empty groups dropped. A query
/// that is a chord bound in `at` puts that row first, its group first.
fn filter(
    groups: Vec<(&'static str, Vec<Entry>)>,
    query: &str,
    keymap: &Keymap,
    at: ContextId,
) -> Vec<Group> {
    let typed = query.trim();
    let chord_row = KeyChord::parse(typed)
        .ok()
        .filter(|chord| keymap.translate(at, *chord) != Translated::Unbound)
        .and_then(|chord| keymap.acting(at, chord));
    let mut out: Vec<(bool, Group)> = Vec::new();
    for (header, entries) in groups {
        let mut kept: Vec<(u32, usize, Entry)> = entries
            .into_iter()
            .enumerate()
            .filter_map(|(order, entry)| {
                if typed.is_empty() {
                    return Some((0, order, entry));
                }
                let hit = chord_row.is_some() && action_of(&entry) == chord_row;
                let score = std::iter::once(entry.row.label.as_str())
                    .chain(entry.search.iter().copied())
                    .filter_map(|text| fuzzy_score(typed, text))
                    .min();
                match (hit, score) {
                    // The chord's own row heads its group.
                    (true, _) => Some((0, 0, entry)),
                    (false, Some(score)) => Some((score, order + 1, entry)),
                    (false, None) => None,
                }
            })
            .collect();
        if kept.is_empty() {
            continue;
        }
        kept.sort_by_key(|(score, order, _)| (*score, *order));
        let has_chord = !typed.is_empty()
            && chord_row.is_some()
            && kept
                .first()
                .is_some_and(|(_, _, entry)| action_of(entry) == chord_row);
        let rows = kept
            .into_iter()
            .map(|(_, _, entry)| (entry.row, entry.target))
            .collect();
        out.push((has_chord, (sanitize_line(header), rows)));
    }
    // The chord's group first; the rest keep their order (a stable sort).
    out.sort_by_key(|(has_chord, _)| !has_chord);
    out.into_iter().map(|(_, group)| group).collect()
}

/// The first row `⏎` can run (not a `YOUR CONFIG` note); 0 when none.
fn first_runnable(groups: &[Group]) -> usize {
    groups
        .iter()
        .flat_map(|(_, rows)| rows)
        .position(|(_, target)| *target != PaletteTarget::Note)
        .unwrap_or(0)
}

/// What a key did to the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edited {
    /// Taken; the palette stays.
    Stay,
    /// `esc`: closed, nothing run.
    Close,
    /// `⏎` on a row: closed, run this.
    Run(PaletteTarget),
    /// Not the palette's (`Ctrl+C`): the caller passes it on.
    Pass,
}

/// One key into an open palette: `↑`/`↓` move (clamped), `⏎` runs, `esc`
/// closes, `Ctrl+C` passes, every other key edits the query (§2.12).
/// `groups` is what is on screen now; the query's own edits re-filter.
pub(crate) fn edit(
    palette: &mut PaletteState,
    key: KeyPress,
    groups: &[Group],
    relist: impl Fn(&PaletteState) -> Vec<Group>,
) -> Edited {
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return Edited::Pass;
    }
    let count: usize = groups.iter().map(|(_, rows)| rows.len()).sum();
    let before = palette.query.text().to_owned();
    match key.code {
        KeyCode::Escape => return Edited::Close,
        KeyCode::Up => palette.cursor = palette.cursor.saturating_sub(1),
        KeyCode::Down => palette.cursor = (palette.cursor + 1).min(count.saturating_sub(1)),
        KeyCode::Enter => {
            // Nothing matches: `⏎` has nothing to run, and the line says
            // how out. A placeholder (`reading…`) runs nothing either.
            return groups
                .iter()
                .flat_map(|(_, rows)| rows)
                .nth(palette.cursor)
                .filter(|(row, target)| !is_placeholder(row, *target))
                .map_or(Edited::Stay, |(_, target)| Edited::Run(*target));
        }
        KeyCode::Backspace => palette.query.backspace(),
        KeyCode::Delete => palette.query.delete(),
        KeyCode::Left => palette.query.left(),
        KeyCode::Right => palette.query.right(),
        KeyCode::Home => palette.query.home(),
        KeyCode::End => palette.query.end(),
        KeyCode::Char(character) if !key.modifiers.control && !key.modifiers.alt => {
            palette.query.insert(character);
        }
        // Every other key is swallowed: a text field lets nothing through
        // (§2.1).
        _ => {}
    }
    if palette.query.text() != before {
        palette.cursor = first_runnable(&relist(palette));
    }
    Edited::Stay
}

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
        Edited::Run(PaletteTarget::More(group)) => {
            widen(state, &mut palette, group);
            state.palette = Some(palette);
        }
        Edited::Run(target) => {
            cancel_finds(&palette, commands);
            run(state, commands, target);
        }
    }
    true
}

/// The query changed (a key, a paste): under `⌃P`, the widening reset, the
/// names job in flight cancelled and one asked for the new query; the
/// cursor on the first row that runs.
fn query_changed(state: &mut AppState, commands: &mut Vec<Command>, palette: &mut PaletteState) {
    if palette.mode == PaletteMode::Everything {
        palette.find.widened = None;
        // A space typed or deleted at either end asks nothing new.
        let query = palette.query.text().trim();
        let names = &palette.find.names;
        let asked = names.pending.as_ref().map(|(_, q)| q).or(names
            .answer
            .as_ref()
            .filter(|_| names.pending.is_none())
            .map(|(q, _)| q));
        if asked.is_none_or(|asked| asked != query) {
            ask_names(state, commands, palette);
        }
    }
    palette.cursor = first_runnable(&rows(state, palette));
}

/// Cancel the names job in flight and, when there is a query and something
/// read to match it against, ask one for the query as it stands.
fn ask_names(state: &mut AppState, commands: &mut Vec<Command>, palette: &mut PaletteState) {
    if let Some((id, _)) = palette.find.names.pending.take() {
        commands.push(Command::CancelJob(id));
    }
    let query = palette.query.text().trim().to_owned();
    if query.is_empty() {
        return;
    }
    let branches = match &palette.find.branches {
        ReadOnce::Read(read) => Some(Arc::clone(read)),
        ReadOnce::NotAsked | ReadOnce::Reading(_) | ReadOnce::Failed(_) => None,
    };
    let tags = state.refs.clone();
    if branches.is_none() && tags.is_none() {
        return;
    }
    let id = next_id(state);
    commands.push(Command::PaletteNames(
        id,
        NamesRequest {
            query: Arc::from(query.as_str()),
            branches,
            tags,
        },
    ));
    palette.find.names.pending = Some((id, query));
}

/// The palette closed: its reads in flight are no one's any more.
fn cancel_finds(palette: &PaletteState, commands: &mut Vec<Command>) {
    if let ReadOnce::Reading(id) = palette.find.branches {
        commands.push(Command::CancelJob(id));
    }
    if let Some((id, _)) = &palette.find.names.pending {
        commands.push(Command::CancelJob(*id));
    }
}

/// `+N more` on `group`: show up to [`WIDE_ROWS`], the cursor on the first
/// row it added.
fn widen(state: &AppState, palette: &mut PaletteState, group: FindGroup) {
    palette.find.widened = Some(group);
    let mut flat = 0;
    for (header, rows) in rows(state, palette) {
        if header.as_str() == group.header() {
            palette.cursor = flat + GROUP_ROWS.min(rows.len().saturating_sub(1));
            return;
        }
        flat += rows.len();
    }
}

/// The branch read landed, or failed. Another open's (a stale id) is
/// dropped. Landed: the names job is asked again with the branches.
pub(super) fn branches_read(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: Result<Arc<SwitchRefs>, String>,
) {
    let ours = state
        .palette
        .as_ref()
        .is_some_and(|p| p.find.branches == ReadOnce::Reading(id));
    if !ours {
        return;
    }
    landed(state, |state, palette| {
        palette.find.branches = match result {
            Ok(read) => ReadOnce::Read(read),
            Err(why) => ReadOnce::Failed(why),
        };
        if matches!(palette.find.branches, ReadOnce::Read(_)) {
            ask_names(state, commands, palette);
        }
    });
}

/// The names job answered. Any but the one in flight (an older query's, a
/// closed palette's) is dropped.
pub(super) fn names_answered(state: &mut AppState, id: JobId, hits: Arc<NameHits>) {
    let ours = state
        .palette
        .as_ref()
        .and_then(|p| p.find.names.pending.as_ref())
        .is_some_and(|(pending, _)| *pending == id);
    if !ours {
        return;
    }
    landed(state, |_, palette| {
        if let Some((_, query)) = palette.find.names.pending.take() {
            palette.find.names.answer = Some((query, hits));
        }
    });
}

/// The refs job answered while `⌃P` has a query: TAGS can be matched now,
/// so the names job is asked again.
pub(super) fn refs_landed(state: &mut AppState, commands: &mut Vec<Command>) {
    let typed = state
        .palette
        .as_ref()
        .is_some_and(|p| p.mode == PaletteMode::Everything && !p.query.text().trim().is_empty());
    if typed {
        landed(state, |state, palette| ask_names(state, commands, palette));
    }
}

/// One shown row, flattened: its group's header, the row, what it runs.
type Flat = (String, PaletteRow, PaletteTarget);

fn flat(groups: Vec<Group>) -> Vec<Flat> {
    groups
        .into_iter()
        .flat_map(|(header, rows)| {
            let header = header.as_str().to_owned();
            rows.into_iter()
                .map(move |(row, target)| (header.clone(), row, target))
        })
        .collect()
}

/// Apply `change` to the open palette, keeping the cursor on its row (its
/// group's header and what it runs). A cursor on a placeholder moves to
/// the first runnable row of that group once one lands, else stays on the
/// placeholder; failing both, the index, clamped.
fn landed(state: &mut AppState, change: impl FnOnce(&mut AppState, &mut PaletteState)) {
    let Some(mut palette) = state.palette.take() else {
        return;
    };
    let anchor = flat(rows(state, &palette)).into_iter().nth(palette.cursor);
    change(state, &mut palette);
    let after = flat(rows(state, &palette));
    let found = anchor.and_then(|(header, row, target)| {
        let in_group = |(h, _, _): &&Flat| *h == header;
        if is_placeholder(&row, target) {
            after
                .iter()
                .position(|f| in_group(&f) && f.2 != PaletteTarget::Note)
                .or_else(|| {
                    after
                        .iter()
                        .position(|f| in_group(&f) && f.1.label == row.label)
                })
        } else {
            after.iter().position(|f| in_group(&f) && f.2 == target)
        }
    });
    palette.cursor = found.unwrap_or_else(|| palette.cursor.min(after.len().saturating_sub(1)));
    state.palette = Some(palette);
}

/// `⏎` on a row, the palette already closed: as the row's key would. A
/// command run is kept in COMMANDS RUN RECENTLY; a find row seeks.
fn run(state: &mut AppState, commands: &mut Vec<Command>, target: PaletteTarget) {
    state.palette_recent.ran(target);
    match target {
        PaletteTarget::Action(id) => {
            if let Some(chord) = default_chord(id) {
                super::dispatch_key(state, commands, chord.to_press());
            }
        }
        PaletteTarget::App(AppAction::Theme(kind)) => commands.push(Command::Retheme(kind)),
        // Lands as the picker's `⇥` does: the graph focused, its cursor on
        // the commit, through the walk when it is not laid out (UX §3.1).
        PaletteTarget::Seek(tip) => {
            state.detail.focus = graft_ui::detail::DetailFocus::Graph;
            super::map::seek_to(state, commands, tip);
        }
        // `+N more` widens in place, never closing the palette
        // (`palette_key`, `palette_mouse`); a note runs nothing.
        PaletteTarget::More(_) | PaletteTarget::Note => {}
    }
}

/// The chord an action answers out of the box: what the dispatchers know
/// it by (ADR-0055). `None` for no such row or one with no default — never
/// listed, so never run.
pub(crate) fn default_chord(id: ActionId) -> Option<KeyChord> {
    defaults()
        .iter()
        .find(|b| b.action == id)
        .and_then(|b| b.default.first().copied())
}

/// A paste while the palette is up goes into its query. `true` when taken.
pub(crate) fn paste(state: &mut AppState, commands: &mut Vec<Command>, text: &str) -> bool {
    let Some(mut palette) = state.palette.take() else {
        return false;
    };
    palette.query.insert_str(text);
    query_changed(state, commands, &mut palette);
    state.palette = Some(palette);
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

/// The mouse while the palette is up (m8/11b decision 6, §1.16): first in
/// the routing, and it claims every event. A click on a row moves the cursor
/// there; a second on the same row within the picker's double-click rule
/// runs it, as `⏎` would; the wheel moves the cursor, clamped; a click
/// outside the palette's rectangle closes it with nothing run, as `esc`.
/// `false` when no palette is up.
pub(crate) fn palette_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: graft_core::event::MouseInput,
) -> bool {
    use graft_core::event::{MouseButton, MouseKind};
    let Some(palette) = state.palette.as_ref() else {
        return false;
    };
    let lines = match mouse.kind {
        MouseKind::ScrollDown => 1,
        MouseKind::ScrollUp => -1,
        MouseKind::Down(MouseButton::Left) => 0,
        // Motion, a release, another button: the palette's, and nothing.
        _ => return true,
    };
    let shown = rows(state, palette);
    let count: usize = shown.iter().map(|(_, rows)| rows.len()).sum();
    if lines != 0 {
        let Some(palette) = state.palette.as_mut() else {
            return true;
        };
        palette.cursor = if lines > 0 {
            (palette.cursor + 1).min(count.saturating_sub(1))
        } else {
            palette.cursor.saturating_sub(1)
        };
        return true;
    }
    let view = with_placeholder(
        view_of(shown.clone(), palette, state.size.columns >= WIDE_FROM),
        palette,
    );
    let row = match crate::view::frame::palette_click(state, &view, mouse.column, mouse.row) {
        crate::view::frame::PaletteClick::Outside => {
            if let Some(palette) = state.palette.take() {
                cancel_finds(&palette, commands);
            }
            return true;
        }
        crate::view::frame::PaletteClick::Inside => return true,
        crate::view::frame::PaletteClick::Row(row) => row,
    };
    let now = state.harness.clock.now_millis();
    let Some(palette) = state.palette.as_mut() else {
        return true;
    };
    let double = palette
        .last_click
        .is_some_and(|(at, when)| at == row && now - when <= super::picker::DOUBLE_CLICK_MS);
    palette.cursor = row;
    if !double {
        palette.last_click = Some((row, now));
        return true;
    }
    let target = shown
        .iter()
        .flat_map(|(_, rows)| rows)
        .nth(row)
        .map(|(row, target)| (is_placeholder(row, *target), *target));
    double_clicked(state, commands, target);
    true
}

/// A double click on a row: what `⏎` does there — nothing on a placeholder,
/// `+N more` widens, anything else runs, the palette closed.
fn double_clicked(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    target: Option<(bool, PaletteTarget)>,
) {
    match target {
        // A placeholder: as `⏎`, nothing.
        None | Some((true, _)) => {}
        Some((false, PaletteTarget::More(group))) => {
            if let Some(mut palette) = state.palette.take() {
                widen(state, &mut palette, group);
                state.palette = Some(palette);
            }
        }
        Some((false, target)) => {
            if let Some(palette) = state.palette.take() {
                cancel_finds(&palette, commands);
            }
            run(state, commands, target);
        }
    }
}
