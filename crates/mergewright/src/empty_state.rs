//! The empty state's keys and view model: what Mergewright does outside a
//! repository (mock-ups §1.15, keymap §2.13; m8-polish-and-ship/08).
//!
//! Pure: a key goes in, an [`OutsideStep`] comes out, and the loop
//! (`event_loop::startup::run_outside`) carries it out — opens a repository,
//! runs `git init`, or quits. A hand dispatcher, like today's screens, until
//! the keymap table takes these keys over (tickets 09/10).
//!
//! The ticket named this module `mergewright::outside`; `event_loop::outside`
//! already names the `$EDITOR` hand-over, so it is `empty_state` here, after
//! the widget it feeds.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_term::{sanitize_line, sanitize_path};
use mergewright_ui::credential_prompt::{CredentialPrompt as CredentialBox, credential_area};
use mergewright_ui::empty_state::{EmptyPrompt, EmptyState, EmptyStateView, RecentRow, recent_age};
use mergewright_ui::keymap::{ContextId, KeyChord, Keymap, Translated};
use mergewright_ui::text_input::TextField;
use mergewright_ui::theme::{Theme, ThemeKind, Token};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::jobs::clone::CloneRequest;
use crate::recent::Recent;
use crate::update::credential::{self, Answer, CredentialPrompt, PromptKey};
use crate::update::palette::PaletteState;
use crate::view::rows::home_relative;

mod clone;
mod open_prompt;
mod palette;

pub(crate) use clone::{CloneRun, CloneState, clone_started, outside_job};
use clone::{clone_prompt_key, clone_view};
use open_prompt::open_prompt_key;
pub(crate) use palette::outside_paste;
use palette::{act, palette_key, render_palette};
// Only the tests reach these through `empty_state`; the children use them.
#[cfg(test)]
use clone::clone_request;
#[cfg(test)]
use open_prompt::{complete, resolve};
#[cfg(test)]
use palette::palette_rows;

/// The prompt open over the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Prompt {
    /// `o`: the path being typed.
    Open(TextField),
    /// `i`: `Make <folder> a Git repository?`.
    ConfirmInit,
    /// `c`: the URL being typed, and the clone it started (m8/18b; the key,
    /// its keys and its drawing are 18c's).
    Clone(CloneState),
}

/// Everything the empty state holds between keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutsideState {
    /// The folder Mergewright was started in (or pointed at).
    pub folder: PathBuf,
    /// `folder` does not exist.
    pub missing: bool,
    /// `$HOME`, for `~` both ways.
    pub home: Option<PathBuf>,
    /// The recent list, newest first.
    pub recents: Vec<Recent>,
    /// Index into `recents`.
    pub cursor: usize,
    pub prompt: Option<Prompt>,
    /// Read in the bar row until the next key.
    pub problem: Option<String>,
    /// `?` / `⌃P` / `:` (m8/11): the palette, over the list. A text field
    /// while it is up — every key but `Ctrl+C` is its own.
    pub palette: Option<PaletteState>,
    /// His config's problems, for the palette's `YOUR CONFIG` rows.
    pub config_problems: Vec<String>,
    /// The first-launch line (m8/18d): shown while `Some(config dir)`. A
    /// plain value seeded from the shell's switch (m8/18e); the first
    /// palette open sets it `None` and asks the loop to end the switch
    /// ([`OutsideStep::FirstLaunchDone`]).
    pub first_launch: Option<PathBuf>,
    /// git's credential question for the running clone (m8/18b, rulings
    /// R5): the same prompt a repository's fetch opens, modal over
    /// everything here while it is up.
    pub credential: Option<CredentialPrompt>,
}

/// What a key asks the loop to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutsideStep {
    /// Nothing outside the state: redraw.
    Stay,
    /// Open this path as a repository.
    Open(PathBuf),
    /// `git init` this folder.
    Init(PathBuf),
    /// Leave Mergewright.
    Quit,
    /// `t`: into the practice tour, coming back to this screen (m8/19b).
    Practise,
    /// A palette theme row: draw in this theme for the rest of the run.
    Retheme(ThemeKind),
    /// Run this clone on the empty state's pool (started on the first).
    StartClone(CloneRequest),
    /// Stop the clone job (`JobRunner::cancel`).
    StopClone(JobId),
    /// The credential prompt's answer for this job; `None` is `esc`: stop
    /// the job, then refuse its ask.
    Credential(JobId, Option<Answer>),
    /// The clone landed: open it. `notice` goes on its first frame.
    Cloned {
        path: PathBuf,
        notice: Option<String>,
    },
    /// The first palette open ended the first-launch line here: end the
    /// shell's switch and write `done` under this config folder; a failure
    /// is said in the bar row (m8/18e).
    FirstLaunchDone(PathBuf),
}

/// A key while git's question is open: the prompt's own (modal), carried
/// out by the loop.
fn credential_key(state: &mut OutsideState, key: KeyPress) -> OutsideStep {
    match credential::prompt_key(&mut state.credential, key) {
        PromptKey::Kept => OutsideStep::Stay,
        PromptKey::Quit => OutsideStep::Quit,
        PromptKey::Send(job, answer) => OutsideStep::Credential(job, Some(answer)),
        PromptKey::Cancel(job) => OutsideStep::Credential(job, None),
    }
}

/// The empty state's edge (ADR-0055): a key, through his `[keys]` remaps,
/// to [`outside_key`]. The open path prompt is a text field, so what is
/// typed there goes on as typed; otherwise the press is translated in
/// `empty_state` — a moved action's chord becomes its default, and a
/// default whose action moved does what a key nobody answers does: clears
/// the problem line, and nothing else. The clone prompt is a text field
/// too (m8/18c). The init box is a confirm box (§2.13, §2.3c):
/// while it is open the press translates in `Confirm`, whose own row is
/// `confirm.yes`, so the start screen's rows behind it (`c` clone, …) are
/// not live there (m8/09d D6).
pub(crate) fn outside_edge(
    state: &mut OutsideState,
    keymap: &Keymap,
    key: KeyPress,
) -> OutsideStep {
    // git's question is modal over everything, the palette included, and
    // keeps the shipped keys whatever his `[keys]` say, as on a repository
    // screen (`update::credential::credential_key` runs first there).
    if state.credential.is_some() {
        return credential_key(state, key);
    }
    if state.palette.is_some() {
        return palette_key(state, keymap, key);
    }
    // Both prompts are text fields: what is typed goes on as typed.
    if matches!(state.prompt, Some(Prompt::Open(_) | Prompt::Clone(_))) {
        return outside_key(state, key);
    }
    let Some(chord) = KeyChord::from_press(&key) else {
        return outside_key(state, key);
    };
    let context = if matches!(state.prompt, Some(Prompt::ConfirmInit)) {
        ContextId::Confirm
    } else {
        ContextId::EmptyState
    };
    match keymap.translate(context, chord) {
        Translated::Pass => act(state, keymap, key),
        Translated::Default(default) => act(state, keymap, default.to_press()),
        Translated::Unbound => {
            state.problem = None;
            OutsideStep::Stay
        }
    }
}

/// The start screen as drawn: the list, and the palette over it with its
/// keys on the bar row while it is up.
pub(crate) fn render(
    state: &OutsideState,
    keymap: &Keymap,
    theme: &Theme,
    ascii: bool,
    now: SystemTime,
    area: Rect,
    buf: &mut Buffer,
) {
    let drawn = view(state, now);
    EmptyState {
        view: &drawn,
        theme,
        ascii,
        keys: keymap,
    }
    .render(area, buf);
    render_palette(state, keymap, theme, area, buf);
    render_credential(state, theme, ascii, area, buf);
}

/// git's question for the clone, last, over everything (it is modal): the
/// box's band cleared full width, as a repository screen clears it
/// (`view::frame::boxes::draw_credential`, ADR-0040). The bar row under it
/// already carries the box's keys (`EmptyStateView::credential`).
fn render_credential(
    state: &OutsideState,
    theme: &Theme,
    ascii: bool,
    area: Rect,
    buf: &mut Buffer,
) {
    let Some(prompt) = &state.credential else {
        return;
    };
    let footer = format!("{} the clone", crate::view::credential::footer_keys());
    let view = crate::view::credential::prompt_view(prompt, &footer);
    // Over the whole screen bar the rule and the bar row.
    let over = Rect {
        height: area.height.saturating_sub(2),
        ..area
    };
    let rect = credential_area(over).intersection(over);
    if rect.is_empty() {
        return;
    }
    let band = Rect {
        x: area.x,
        width: area.width,
        ..rect
    };
    for y in band.y..band.y + band.height {
        for x in band.x..band.x + band.width {
            buf[(x, y)].reset();
        }
    }
    buf.set_style(
        rect,
        ratatui::style::Style::new().bg(mergewright_ui::paint::color(
            theme.token(Token::SurfaceRaised),
        )),
    );
    CredentialBox {
        view: &view,
        theme,
        ascii,
    }
    .render(rect, buf);
}

/// Apply one key. A `problem` is cleared by any key, which is then acted on
/// as usual (ticket 08 Scope: "until the next key, which is also acted on").
pub(crate) fn outside_key(state: &mut OutsideState, key: KeyPress) -> OutsideStep {
    state.problem = None;
    if key.modifiers.control && key.code == KeyCode::Char('c') {
        return OutsideStep::Quit;
    }
    match state.prompt.take() {
        Some(Prompt::Open(field)) => open_prompt_key(state, field, key),
        Some(Prompt::ConfirmInit) => match key.code {
            KeyCode::Char('y') => OutsideStep::Init(state.folder.clone()),
            // A confirm box: `esc` cancels, any other key is swallowed
            // (§2.3c :296). No raw `n` — no keymap row names it (m8/09e).
            KeyCode::Escape => OutsideStep::Stay,
            _ => {
                state.prompt = Some(Prompt::ConfirmInit);
                OutsideStep::Stay
            }
        },
        Some(Prompt::Clone(clone)) => clone_prompt_key(state, clone, key),
        None => list_key(state, key),
    }
}

fn list_key(state: &mut OutsideState, key: KeyPress) -> OutsideStep {
    match key.code {
        KeyCode::Char('q') => OutsideStep::Quit,
        // `empty_state.tour` (§2.13): the practice tour. Not gated on a
        // missing folder: the tour builds its own, and comes back here.
        KeyCode::Char('t') => OutsideStep::Practise,
        KeyCode::Char('o') => {
            let shown = home_relative(&state.folder, state.home.as_deref());
            let mut text = shown.to_string_lossy().into_owned();
            if !text.ends_with('/') {
                text.push('/');
            }
            state.prompt = Some(Prompt::Open(TextField::new(&text)));
            OutsideStep::Stay
        }
        // Not answered for a missing folder: a typo must not become a
        // repository (the ticket's open question; default no). The one
        // place to change if the user says yes.
        KeyCode::Char('i') if !state.missing => {
            state.prompt = Some(Prompt::ConfirmInit);
            OutsideStep::Stay
        }
        // Gated as `i` (rulings R6): a missing folder offers neither.
        KeyCode::Char('c') if !state.missing => {
            state.prompt = Some(Prompt::Clone(CloneState {
                url: TextField::new(""),
                run: CloneRun::Typing,
            }));
            OutsideStep::Stay
        }
        KeyCode::Enter => state
            .recents
            .get(state.cursor)
            .map_or(OutsideStep::Stay, |recent| {
                OutsideStep::Open(recent.path.clone())
            }),
        // Clamped at both ends, as the branch picker moves
        // (`update::picker`, `saturating_sub` / `min(last)`).
        KeyCode::Char('j') | KeyCode::Down => {
            let last = state.recents.len().saturating_sub(1);
            state.cursor = (state.cursor + 1).min(last);
            OutsideStep::Stay
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.cursor = state.cursor.saturating_sub(1);
            OutsideStep::Stay
        }
        _ => OutsideStep::Stay,
    }
}

/// The view model the widget draws, ages read against `now`.
pub(crate) fn view(state: &OutsideState, now: SystemTime) -> EmptyStateView {
    let secs = |time: SystemTime| {
        time.duration_since(UNIX_EPOCH).map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
    };
    let home = state.home.as_deref();
    EmptyStateView {
        folder: sanitize_path(&home_relative(&state.folder, home)),
        missing: state.missing,
        recent: state
            .recents
            .iter()
            .map(|recent| RecentRow {
                name: recent.path.file_name().map_or_else(
                    || sanitize_path(&recent.path),
                    |name| sanitize_path(Path::new(name)),
                ),
                path: sanitize_path(&home_relative(&recent.path, home)),
                age: sanitize_line(&recent_age(secs(recent.opened), secs(now))),
            })
            .collect(),
        cursor: state.cursor,
        prompt: state.prompt.as_ref().map(|prompt| match prompt {
            Prompt::Open(field) => EmptyPrompt::Open {
                text: sanitize_line(field.text()),
                cursor: field.cursor(),
            },
            Prompt::ConfirmInit => EmptyPrompt::ConfirmInit,
            Prompt::Clone(clone) => EmptyPrompt::Clone(clone_view(state, clone)),
        }),
        problem: state.problem.as_deref().map(sanitize_line),
        first_launch: state.first_launch.is_some(),
        credential: state.credential.is_some(),
    }
}

#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod sweep_tests;
#[cfg(test)]
mod tests;
