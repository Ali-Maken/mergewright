//! The empty state's keys and view model: what Graft does outside a
//! repository (mock-ups §1.15, keymap §2.13; m8-polish-and-ship/08).
//!
//! Pure: a key goes in, an [`OutsideStep`] comes out, and the loop
//! (`event_loop::startup::run_outside`) carries it out — opens a repository,
//! runs `git init`, or quits. A hand dispatcher, like today's screens, until
//! the keymap table takes these keys over (tickets 09/10).
//!
//! The ticket named this module `graft::outside`; `event_loop::outside`
//! already names the `$EDITOR` hand-over, so it is `empty_state` here, after
//! the widget it feeds.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ops::clone::{CloneError, name_from_url};
use graft_term::{sanitize_line, sanitize_path};
use graft_ui::empty_state::{EmptyPrompt, EmptyState, EmptyStateView, RecentRow, recent_age};
use graft_ui::keymap::{ContextId, KeyChord, Keymap, Translated};
use graft_ui::paint::foreground;
use graft_ui::palette::{Palette, WIDE_FROM, palette_area};
use graft_ui::text_input::TextField;
use graft_ui::theme::{Theme, ThemeKind, Token};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::event_loop::Wake;
use crate::first_launch::FirstLaunch;
use crate::jobs::clone::{CloneAnswer, CloneRequest};
use crate::recent::Recent;
use crate::update::credential::{self, Answer, CredentialPrompt, PromptKey};
use crate::update::palette::{
    self as palette, AppAction, Edited, Group, PaletteState, PaletteTarget, Source,
};
use crate::update::{AppEvent, Product, Progress};
use crate::view::rows::home_relative;

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

/// The clone prompt: what was typed, and where its clone is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CloneState {
    /// The URL. Kept after a failure or a stop, for a retry (rulings R6).
    pub url: TextField,
    pub run: CloneRun,
}

/// Where the clone prompt's clone is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CloneRun {
    /// Nothing has run yet: the URL is being typed.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "18c's `c` opens the prompt typing")
    )]
    Typing,
    /// The clone job is running.
    Running {
        job: JobId,
        /// The folder git is making.
        into: PathBuf,
        /// git's last progress line, shortened (`RemoteProgress::brief`, as
        /// the job's `Progress::RemoteProgress` carries it). `None` before
        /// git's first line.
        progress: Option<String>,
    },
    /// It failed or was stopped: `CloneError::explain`, sanitised.
    Ended { words: String },
}

/// What a stop says when the clone was stopped before it began: nothing ran,
/// so nothing can be left (`CloneError::Stopped { left: None }`'s words).
const STOPPED_BEFORE_IT_RAN: &str = "clone stopped \u{b7} nothing kept";

/// Everything the empty state holds between keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutsideState {
    /// The folder Graft was started in (or pointed at).
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
    /// The first-launch line (m8/18d): the process's one, shared with
    /// every repository screen `drive` runs.
    pub first_launch: FirstLaunch,
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
    /// Leave Graft.
    Quit,
    /// A palette theme row: draw in this theme for the rest of the run.
    Retheme(ThemeKind),
    /// Run this clone on the empty state's pool (started on the first).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "18c's clone prompt `⏎` asks for it")
    )]
    StartClone(CloneRequest),
    /// Stop the clone job (`JobRunner::cancel`).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "18c's `esc` while running asks for it")
    )]
    StopClone(JobId),
    /// The credential prompt's answer for this job; `None` is `esc`: stop
    /// the job, then refuse its ask.
    Credential(JobId, Option<Answer>),
    /// The clone landed: open it. `notice` goes on its first frame.
    Cloned {
        path: PathBuf,
        notice: Option<String>,
    },
}

/// The clone `⏎` asks for: `url` into `folder`/<the name git would pick>.
///
/// # Errors
/// The reason, in rulings R6's words, when no folder can be named from the
/// URL or that folder is already there with something in it. Nothing has
/// run either way.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "18c's clone prompt `⏎` calls it")
)]
pub(crate) fn clone_request(folder: &Path, url: &str) -> Result<CloneRequest, String> {
    let name = name_from_url(url)
        .filter(|_| !url.starts_with('-'))
        .ok_or_else(|| CloneError::BadUrl.explain())?;
    let into = folder.join(name);
    // As `graft_ops::clone` refuses (it checks again when it runs): an
    // empty folder is filled, anything else there is his.
    let empty = std::fs::read_dir(&into).is_ok_and(|mut entries| entries.next().is_none());
    if into.exists() && !empty {
        return Err(sanitize_line(&CloneError::TargetNotEmpty(into).explain())
            .as_str()
            .to_owned());
    }
    Ok(CloneRequest {
        url: url.to_owned(),
        into,
    })
}

/// The clone job `job` is running into `into`: the prompt goes read-only
/// on it, `connecting…` until git's first progress line.
pub(crate) fn clone_started(state: &mut OutsideState, job: JobId, into: PathBuf) {
    let run = CloneRun::Running {
        job,
        into,
        progress: None,
    };
    match &mut state.prompt {
        Some(Prompt::Clone(clone)) => clone.run = run,
        // Only the clone prompt asks for a clone; were it gone, the job
        // still has a place on screen, so its answer is never dropped.
        _ => {
            state.prompt = Some(Prompt::Clone(CloneState {
                url: TextField::new(""),
                run,
            }));
        }
    }
}

/// The running clone's job, if one is.
fn running_clone(state: &OutsideState) -> Option<JobId> {
    match &state.prompt {
        Some(Prompt::Clone(CloneState {
            run: CloneRun::Running { job, .. },
            ..
        })) => Some(*job),
        _ => None,
    }
}

/// A job said something: what it means for the running clone. Progress
/// updates its line; git's question opens the credential prompt; an end
/// puts its words on the prompt and closes the question; a landed clone is
/// [`OutsideStep::Cloned`]. Anything about another job is not this
/// screen's: [`OutsideStep::Stay`].
pub(crate) fn outside_job(state: &mut OutsideState, wake: &Wake) -> OutsideStep {
    let Some(running) = running_clone(state) else {
        return OutsideStep::Stay;
    };
    let Wake::Event(event) = wake else {
        return OutsideStep::Stay;
    };
    let ended = match event {
        AppEvent::Progress(id, Progress::RemoteProgress { brief }) if *id == running => {
            if let Some(Prompt::Clone(CloneState {
                run: CloneRun::Running { progress, .. },
                ..
            })) = &mut state.prompt
            {
                *progress = Some(brief.clone());
            }
            return OutsideStep::Stay;
        }
        AppEvent::Progress(id, Progress::CredentialNeeded { prompt, pulse_seed }) => {
            // Nothing here marks a clone as stopping: after `esc` the job's
            // own cancel refuses its asks before they are sent
            // (`ChannelAskPass`), so none reaches this.
            credential::open_prompt(
                &mut state.credential,
                Some(running),
                false,
                *id,
                prompt,
                *pulse_seed,
            );
            return OutsideStep::Stay;
        }
        AppEvent::Produced(id, Product::Cloned(CloneAnswer::Done { path, notice }))
            if *id == running =>
        {
            state.credential = None;
            return OutsideStep::Cloned {
                path: path.clone(),
                notice: notice.clone(),
            };
        }
        AppEvent::Produced(id, Product::Cloned(CloneAnswer::Ended { words })) if *id == running => {
            words.clone()
        }
        // Stopped while still queued: nothing ran.
        AppEvent::Core(Event::JobCancelled(id)) if *id == running => {
            STOPPED_BEFORE_IT_RAN.to_owned()
        }
        AppEvent::Core(Event::JobFailed(id, failure)) if *id == running => {
            sanitize_line(&format!("clone failed: {failure}"))
                .as_str()
                .to_owned()
        }
        _ => return OutsideStep::Stay,
    };
    if let Some(Prompt::Clone(clone)) = &mut state.prompt {
        clone.run = CloneRun::Ended { words: ended };
    }
    let still = running_clone(state);
    credential::close_orphaned(&mut state.credential, still);
    OutsideStep::Stay
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
/// the problem line, and nothing else. (The clone prompt, ticket 18, joins
/// the "prompt open" check.) The init box is a confirm box (§2.13, §2.3c):
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
    if matches!(state.prompt, Some(Prompt::Open(_))) {
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

/// A key past the edge (a default chord): `?`, `⌃P` or `:` open the
/// palette on the list — not over the init box, which swallows them
/// (§2.3c) — and every other key is [`outside_key`]'s.
fn act(state: &mut OutsideState, keymap: &Keymap, key: KeyPress) -> OutsideStep {
    if let Some(how) = palette::opener(key)
        && state.prompt.is_none()
    {
        state.problem = None;
        // The first palette open ends the line, for this launch and every
        // later one; a failure to say so is said where problems go.
        if let Some(config_dir) = state.first_launch.end()
            && let Err(error) = crate::first_launch::first_launch_done(&config_dir)
        {
            state.problem = Some(error.words());
        }
        let source = source_of(&state.config_problems, keymap);
        state.palette = Some(palette::opened(&source, ContextId::EmptyState, how));
        return OutsideStep::Stay;
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
fn palette_key(state: &mut OutsideState, keymap: &Keymap, key: KeyPress) -> OutsideStep {
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
        open.cursor = groups
            .iter()
            .flat_map(|(_, rows)| rows)
            .position(|(_, target)| *target != PaletteTarget::Note)
            .unwrap_or(0);
    } else if let Some(Prompt::Open(field)) = &mut state.prompt {
        field.insert_str(text);
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
        // Its keys are 18c's; until then it keeps every key.
        Some(clone @ Prompt::Clone(_)) => {
            state.prompt = Some(clone);
            OutsideStep::Stay
        }
        None => list_key(state, key),
    }
}

fn list_key(state: &mut OutsideState, key: KeyPress) -> OutsideStep {
    match key.code {
        KeyCode::Char('q') => OutsideStep::Quit,
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

fn open_prompt_key(state: &mut OutsideState, mut field: TextField, key: KeyPress) -> OutsideStep {
    match key.code {
        KeyCode::Escape => return OutsideStep::Stay,
        KeyCode::Enter => {
            if field.text().trim().is_empty() {
                state.prompt = Some(Prompt::Open(field));
                return OutsideStep::Stay;
            }
            let path = resolve(field.text(), &state.folder, state.home.as_deref());
            // The prompt closes. If the open fails, the driver comes back
            // here with the reason as `problem` and the cursor kept.
            return OutsideStep::Open(path);
        }
        KeyCode::Tab => {
            if let Some(completed) = complete(field.text(), &state.folder, state.home.as_deref()) {
                field = TextField::new(&completed);
            }
        }
        KeyCode::Backspace => field.backspace(),
        KeyCode::Delete => field.delete(),
        KeyCode::Left => field.left(),
        KeyCode::Right => field.right(),
        KeyCode::Home => field.home(),
        KeyCode::End => field.end(),
        KeyCode::Char(c) if !key.modifiers.control && !key.modifiers.alt => field.insert(c),
        _ => {}
    }
    state.prompt = Some(Prompt::Open(field));
    OutsideStep::Stay
}

/// A typed path as a real one: `~` is `home`, a relative path is against
/// the shown `folder`.
pub(crate) fn resolve(typed: &str, folder: &Path, home: Option<&Path>) -> PathBuf {
    let typed = typed.trim();
    let expanded = match (typed.strip_prefix('~'), home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(typed),
    };
    if expanded.is_absolute() {
        expanded
    } else {
        folder.join(expanded)
    }
}

/// `⇥`: complete the last part of `typed` to a directory. One match: the
/// whole name and a `/`. Several: their longest common start. None: `None`.
/// Hidden folders only when the typed part starts with `.`.
pub(crate) fn complete(typed: &str, folder: &Path, home: Option<&Path>) -> Option<String> {
    let (head, part) = match typed.rfind('/') {
        Some(at) => (&typed[..=at], &typed[at + 1..]),
        None => ("", typed),
    };
    let dir = if head.is_empty() {
        folder.to_path_buf()
    } else {
        resolve(head, folder, home)
    };
    // A folder that cannot be listed, an entry that cannot be read, a name
    // that is not UTF-8 (the field holds text): nothing to complete to,
    // which `⇥` shows by leaving the field as it is. Never an error to say:
    // completion is a convenience over typing the name.
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(part) && (part.starts_with('.') || !name.starts_with('.')))
        .collect();
    names.sort();
    let first = names.first()?;
    if names.len() == 1 {
        return Some(format!("{head}{first}/"));
    }
    let common = names.iter().skip(1).fold(first.clone(), |common, name| {
        common
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect()
    });
    (common.len() > part.len()).then(|| format!("{head}{common}"))
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
        prompt: state.prompt.as_ref().and_then(|prompt| match prompt {
            Prompt::Open(field) => Some(EmptyPrompt::Open {
                text: sanitize_line(field.text()),
                cursor: field.cursor(),
            }),
            Prompt::ConfirmInit => Some(EmptyPrompt::ConfirmInit),
            // Drawn by 18c (graft-ui has no clone prompt yet).
            Prompt::Clone(_) => None,
        }),
        problem: state.problem.as_deref().map(sanitize_line),
        first_launch: state.first_launch.is_shown(),
    }
}

#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod sweep_tests;
#[cfg(test)]
mod tests;
