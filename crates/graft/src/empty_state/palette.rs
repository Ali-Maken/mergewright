//! The palette on the start screen (m8/11): opening it, its keys, a paste
//! into its query, and drawing it over the list.

use graft_core::event::KeyPress;
use graft_ui::keymap::{ContextId, Keymap};
use graft_ui::paint::foreground;
use graft_ui::palette::{Palette, WIDE_FROM, palette_area};
use graft_ui::theme::{Theme, Token};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::{CloneRun, OutsideState, OutsideStep, Prompt, outside_key};
use crate::update::credential;
use crate::update::palette::{self as palette, AppAction, Edited, Group, PaletteTarget, Source};

/// A key past the edge (a default chord): `?`, `⌃P` or `:` open the
/// palette on the list — not over the init box, which swallows them
/// (§2.3c) — and every other key is [`outside_key`]'s.
pub(super) fn act(state: &mut OutsideState, keymap: &Keymap, key: KeyPress) -> OutsideStep {
    if let Some(how) = palette::opener(key)
        && state.prompt.is_none()
    {
        state.problem = None;
        let source = source_of(&state.config_problems, keymap);
        state.palette = Some(palette::opened(&source, ContextId::EmptyState, how));
        // The first palette open ends the line, for this launch and every
        // later one: gone here this frame; the loop ends it for the process
        // and writes `done`, and says a failure where problems go.
        return state
            .first_launch
            .take()
            .map_or(OutsideStep::Stay, OutsideStep::FirstLaunchDone);
    }
    outside_key(state, key)
}

/// What the palette's rows are built from on the start screen: no
/// operation can be live outside a repository, and there is nothing to
/// find but actions (ticket 12).
fn source_of<'a>(problems: &'a [String], keymap: &'a Keymap) -> Source<'a> {
    palette::outside_source(keymap, problems)
}

/// The palette's rows as drawn now. Empty when it is not up.
pub(crate) fn palette_rows(state: &OutsideState, keymap: &Keymap) -> Vec<Group> {
    state.palette.as_ref().map_or_else(Vec::new, |palette| {
        palette::listing(&source_of(&state.config_problems, keymap), palette)
    })
}

/// A key while the palette is up (decision 8): `⏎` closes it and runs the
/// row as its key would — the default chord, through [`act`] — and a
/// theme row asks the loop for that theme.
pub(super) fn palette_key(state: &mut OutsideState, keymap: &Keymap, key: KeyPress) -> OutsideStep {
    let Some(mut open) = state.palette.take() else {
        return outside_key(state, key);
    };
    let source = source_of(&state.config_problems, keymap);
    let shown = palette::listing(&source, &open);
    match palette::edit(&mut open, key, &shown, |p| palette::listing(&source, p)) {
        Edited::Stay => {
            state.palette = Some(open);
            OutsideStep::Stay
        }
        // No find rows outside a repository: a seek or `+N more` is never
        // listed here, so never run.
        Edited::Close
        | Edited::Run(
            PaletteTarget::Note
            | PaletteTarget::Seek(_)
            | PaletteTarget::File(_)
            | PaletteTarget::More(_),
        ) => OutsideStep::Stay,
        // `Ctrl+C`: quits, as everywhere (§2.1).
        Edited::Pass => {
            state.palette = Some(open);
            outside_key(state, key)
        }
        Edited::Run(PaletteTarget::Action { id, .. }) => palette::default_chord(id)
            .map_or(OutsideStep::Stay, |chord| {
                act(state, keymap, chord.to_press())
            }),
        Edited::Run(PaletteTarget::App(AppAction::Theme(kind))) => OutsideStep::Retheme(kind),
        // Listed only when no key row starts the tour; outside a
        // repository `empty_state.tour` always is, but the row means the
        // same wherever it runs.
        Edited::Run(PaletteTarget::App(AppAction::Tour)) => OutsideStep::Practise,
    }
}

/// A paste: into the palette's query or the open prompt, else nowhere.
pub(crate) fn outside_paste(state: &mut OutsideState, keymap: &Keymap, text: &str) {
    if credential::prompt_paste(&mut state.credential, text) {
        return;
    }
    if let Some(open) = &mut state.palette {
        open.query.insert_str(text);
        let groups = palette::listing(&source_of(&state.config_problems, keymap), open);
        open.cursor = palette::first_runnable(&groups);
    } else if let Some(Prompt::Open(field)) = &mut state.prompt {
        field.insert_str(text);
    } else if let Some(Prompt::Clone(clone)) = &mut state.prompt
        && !matches!(clone.run, CloneRun::Running { .. })
    {
        // A URL pasted with its newline is still one line.
        clone.url.insert_str(text.trim());
        clone.run = CloneRun::Typing;
    }
}

/// The palette over the list, its keys on the bar row, while it is up.
pub(super) fn render_palette(
    state: &OutsideState,
    keymap: &Keymap,
    theme: &Theme,
    area: Rect,
    buf: &mut Buffer,
) {
    let Some(open) = &state.palette else {
        return;
    };
    let groups = palette_rows(state, keymap);
    let shown = palette::view_of(groups, open, area.width >= WIDE_FROM);
    // Over the whole screen bar the bar row, which carries its keys.
    let over = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    Palette {
        view: &shown,
        theme,
    }
    .render(palette_area(over, area), buf);
    if area.height == 0 {
        return;
    }
    let bar = area.y + area.height - 1;
    for x in area.x..area.x + area.width {
        buf[(x, bar)].reset();
    }
    buf.set_stringn(
        area.x + 1,
        bar,
        palette::bar(),
        usize::from(area.width.saturating_sub(2)),
        foreground(theme, Token::TextMuted),
    );
}
