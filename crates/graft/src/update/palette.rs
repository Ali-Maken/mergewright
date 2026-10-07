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

use graft_core::event::{KeyCode, KeyPress};
use graft_core::matcher::fuzzy_score;
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::keymap::{
    ActionId, Binding, ContextId, KeyChord, KeyLabel, Keymap, Translated, defaults,
};
use graft_ui::palette::{PaletteMode, PaletteRow, PaletteView, RowKind, WIDE_FROM};
use graft_ui::text_input::TextField;
use graft_ui::theme::ThemeKind;

use super::{AppState, Command};

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
}

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
    /// A `YOUR CONFIG` row: nothing to run; `⏎` closes.
    Note,
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
}

/// One row before filtering: its words to match on, besides the label.
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
    };
    palette.cursor = first_runnable(&listing(source, &palette));
    palette
}

/// `?`, `⌃P` or `:` reached the dispatchers: open the palette on the place
/// the key would have gone to, unless a text field is open (it types the
/// key, and never reaches here) or a confirm box is up (it swallows every
/// key but its own, §2.3c). `true` when it opened.
pub(crate) fn open_key(state: &mut AppState, key: KeyPress) -> bool {
    let Some(how) = opener(key) else {
        return false;
    };
    if state.palette.is_some() || state.text_field_open() || state.confirm.is_some() {
        return false;
    }
    open(state, how);
    true
}

/// Open the palette over whatever is on screen.
pub(crate) fn open(state: &mut AppState, how: Opens) {
    let opened_from = super::edge::context_of(state);
    let palette = opened(&source_of(state), opened_from, how);
    state.palette = Some(Box::new(palette));
}

fn source_of(state: &AppState) -> Source<'_> {
    Source {
        keymap: &state.keymap,
        problems: &state.config_problems,
        operation_live: super::banner::live(state).is_some(),
    }
}

/// The palette's rows as drawn: groups in order, filtered by the query.
pub(crate) fn rows(state: &AppState, palette: &PaletteState) -> Vec<Group> {
    listing(&source_of(state), palette)
}

/// [`rows`] from a [`Source`]: the groups of decision 2, filtered and
/// ordered by decision 4.
pub(crate) fn listing(source: &Source<'_>, palette: &PaletteState) -> Vec<Group> {
    let groups = groups(source, palette);
    filter(
        groups,
        palette.query.text(),
        source.keymap,
        palette.opened_from,
    )
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
            out.push(("ACTIONS", actions));
        }
    }
    out
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
        PaletteTarget::App(_) | PaletteTarget::Note => None,
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
            // how out.
            return groups
                .iter()
                .flat_map(|(_, rows)| rows)
                .nth(palette.cursor)
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
    let edited = edit(&mut palette, key, &shown, |p| rows(state, p));
    match edited {
        Edited::Stay => state.palette = Some(palette),
        Edited::Close => {}
        Edited::Pass => {
            state.palette = Some(palette);
            return false;
        }
        Edited::Run(target) => run(state, commands, target),
    }
    true
}

/// `⏎` on a row, the palette already closed: as the row's key would.
fn run(state: &mut AppState, commands: &mut Vec<Command>, target: PaletteTarget) {
    match target {
        PaletteTarget::Action(id) => {
            if let Some(chord) = default_chord(id) {
                super::dispatch_key(state, commands, chord.to_press());
            }
        }
        PaletteTarget::App(AppAction::Theme(kind)) => commands.push(Command::Retheme(kind)),
        PaletteTarget::Note => {}
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
pub(crate) fn paste(state: &mut AppState, text: &str) -> bool {
    let Some(mut palette) = state.palette.take() else {
        return false;
    };
    palette.query.insert_str(text);
    palette.cursor = first_runnable(&rows(state, &palette));
    state.palette = Some(palette);
    true
}

/// The palette as the widget draws it.
pub(crate) fn view(state: &AppState, palette: &PaletteState) -> PaletteView {
    view_of(
        rows(state, palette),
        palette,
        state.size.columns >= WIDE_FROM,
    )
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
