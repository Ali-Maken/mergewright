//! Opening the palette and what it does with keys, pastes and the mouse.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

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
/// branches once, for BRANCHES (ticket 12), and the tracked paths, for
/// FILES (12b); `?` reads nothing.
pub(crate) fn open(state: &mut AppState, commands: &mut Vec<Command>, how: Opens) {
    let opened_from = crate::update::edge::context_of(state);
    let mut palette = opened(&source_of(state), opened_from, how);
    if how == Opens::Everything {
        let id = next_id(state);
        palette.find.branches = ReadOnce::Reading(id);
        commands.push(Command::ReadPaletteBranches(id));
        let id = next_id(state);
        palette.find.files = ReadOnce::Reading(id);
        commands.push(Command::ReadTrackedPaths(id));
    }
    state.palette = Some(Box::new(palette));
    // The first palette open ends the first-launch line for good (m8/18d):
    // gone this frame; the loop ends it for every later place and writes
    // `done` (`update` touches no file), which says a failure once.
    if let Some(config_dir) = state.first_launch.take() {
        commands.push(Command::FirstLaunchDone(*config_dir));
    }
}

pub(super) fn next_id(state: &mut AppState) -> JobId {
    let mut local_jobs = state.local_jobs;
    let id = local_jobs.next_id();
    state.local_jobs = local_jobs;
    id
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

/// The mouse while the palette is up (m8/11b decision 6, §1.16): first in
/// the routing, and it claims every event. A click on a row moves the cursor
/// there; a second on the same row within the picker's double-click rule
/// runs it, as `⏎` would; the wheel moves the cursor, clamped; a click
/// outside the palette's rectangle closes it with nothing run, as `esc`.
/// `false` when no palette is up.
pub(crate) fn palette_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: mergewright_core::event::MouseInput,
) -> bool {
    use mergewright_core::event::{MouseButton, MouseKind};
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
    let double = palette.last_click.is_some_and(|(at, when)| {
        at == row && now - when <= crate::update::picker::DOUBLE_CLICK_MS
    });
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
pub(super) fn double_clicked(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    target: Option<(bool, PaletteTarget)>,
) {
    match target {
        // A placeholder: as `⏎`, nothing.
        None | Some((true, _)) => {}
        Some((false, PaletteTarget::More(group))) if group != FindGroup::Commits => {
            if let Some(mut palette) = state.palette.take() {
                widen(state, &mut palette, group);
                state.palette = Some(palette);
            }
        }
        Some((false, target)) => {
            if let Some(palette) = state.palette.take() {
                cancel_finds(&palette, commands);
                run(state, commands, &palette, target);
            }
        }
    }
}
