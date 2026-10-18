//! The one thing that moves Mergewright forward: an [`Event`].
//!
//! Everything the application does starts here. The terminal produces key,
//! mouse, resize, paste and focus events; the job runner produces the two
//! job-result events. `app`'s event loop turns each one into a new state and a
//! list of commands (`Event ──▶ update ──▶ (state, [Command])`,
//! ARCHITECTURE.md).
//!
//! These types deliberately name nothing from crossterm, `git`, `graph` or
//! `ops` (DESIGN.md §8.2). The keys and buttons below are Mergewright's own
//! vocabulary — the one DESIGN.md §2's keymap is written in — so that the
//! terminal library stays an implementation detail of `app`'s input
//! translation, and `update` stays a pure function over plain data.

use crate::job::{JobFailure, JobId};

/// Something that happened and that `update` must answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A key went down (or repeated). Key *releases* never reach `update`:
    /// Mergewright's keymap is entirely press-driven.
    Key(KeyPress),
    /// The mouse moved, was clicked, dragged or scrolled. First-class, not an
    /// afterthought (PROMPT.md §3).
    Mouse(MouseInput),
    /// The terminal window changed size, in character cells.
    ///
    /// Either number can be zero: a pty that has not been sized yet reports
    /// `0×0`, which is exactly what broke the graph spike (ADR-0012). Layout
    /// code must treat this as data to survive, never as an impossible value.
    Resize {
        /// Width in character cells.
        columns: u16,
        /// Height in character cells.
        rows: u16,
    },
    /// A bracketed paste arrived as one lump, rather than as a burst of key
    /// presses. The text is whatever the user pasted and is therefore
    /// untrusted — sanitise it before it can reach the screen.
    Paste(String),
    /// The terminal window took focus.
    FocusGained,
    /// The terminal window lost focus.
    FocusLost,
    /// A background job finished. What it produced is looked up by id in the
    /// job runner's own table — `core` never sees a job's contents.
    JobFinished(JobId),
    /// A background job failed, and why. Never dropped on the floor: a job
    /// that could not do its work always says so (CLAUDE.md).
    JobFailed(JobId, JobFailure),
    /// A background job stopped because it was cancelled. Its own outcome —
    /// not a success and not a failure, because the application asked for it,
    /// so there is nothing to report to the user.
    JobCancelled(JobId),
}

/// A key going down, as Mergewright's keymap thinks of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyPress {
    /// Which key.
    pub code: KeyCode,
    /// Which modifiers were held with it.
    pub modifiers: Modifiers,
}

impl KeyPress {
    /// A key pressed with no modifiers — the common case, and the shape most
    /// of DESIGN.md §2's table is written in.
    #[must_use]
    pub const fn plain(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::NONE,
        }
    }

    /// A key pressed with control held.
    #[must_use]
    pub const fn control(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::CONTROL,
        }
    }
}

/// The keys DESIGN.md §2's keymap can name.
///
/// Deliberately not every key a terminal can report: media keys, keypad
/// oddities and bare modifier presses have no binding anywhere in Mergewright, so
/// `app`'s translation drops them rather than widening this type with variants
/// nothing will ever match on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A character key, after the terminal has applied shift and any keyboard
    /// layout — `Char('A')` for shift+a on a US layout.
    Char(char),
    /// Enter / Return.
    Enter,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Shift+Tab, which terminals report as its own key rather than as Tab
    /// with a modifier.
    BackTab,
    /// Delete (forward delete).
    Delete,
    /// Insert.
    Insert,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// A function key: `Function(1)` is F1.
    Function(u8),
}

/// The modifiers held during a key press or a mouse event.
///
/// A plain struct of flags rather than a bitset: a keymap reads better as
/// `modifiers.control` than as a mask test, and there are four of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    /// Control.
    pub control: bool,
    /// Alt / Option.
    pub alt: bool,
    /// Shift. Note that for [`KeyCode::Char`] the shift is usually already
    /// applied to the character itself, so this flag is mostly interesting on
    /// the non-character keys.
    pub shift: bool,
    /// The platform's "super" key: Command on macOS, the logo key elsewhere.
    pub command: bool,
}

impl Modifiers {
    /// No modifiers held.
    pub const NONE: Self = Self {
        control: false,
        alt: false,
        shift: false,
        command: false,
    };

    /// Control alone.
    pub const CONTROL: Self = Self {
        control: true,
        ..Self::NONE
    };
}

/// A mouse event, positioned in character cells with the origin at the top
/// left of the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MouseInput {
    /// What the mouse did.
    pub kind: MouseKind,
    /// Column, zero-based.
    pub column: u16,
    /// Row, zero-based.
    pub row: u16,
    /// Which modifiers were held.
    pub modifiers: Modifiers,
}

/// What a [`MouseInput`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseKind {
    /// A button went down.
    Down(MouseButton),
    /// A button came up.
    Up(MouseButton),
    /// The mouse moved with a button held.
    Drag(MouseButton),
    /// The mouse moved with no button held — what hover needs, and why the
    /// session enables any-motion reporting.
    Moved,
    /// The wheel scrolled up.
    ScrollUp,
    /// The wheel scrolled down.
    ScrollDown,
    /// The wheel (or trackpad) scrolled left.
    ScrollLeft,
    /// The wheel (or trackpad) scrolled right.
    ScrollRight,
}

/// The three mouse buttons terminals report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Middle button.
    Middle,
    /// Right button.
    Right,
}
