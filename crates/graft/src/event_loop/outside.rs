//! Handing the terminal to something else and taking it back
//! (m3-working-tree-undo-and-stash/05).
//!
//! Two things need this: `Ctrl+E`, which runs the user's `$EDITOR` on the
//! commit message, and `Ctrl+Z` under `keys.suspend_on_ctrl_z`, which stops
//! Graft the way any other terminal program stops.
//!
//! Both are the same problem. Graft holds the terminal in a state no other
//! program expects — raw mode, the alternate screen, mouse reporting,
//! bracketed paste, focus events, a hidden cursor, the kitty keyboard
//! protocol — and it holds stdin through a reader thread that never stops
//! reading. Handing the terminal over means putting every one of those back
//! and getting the reader out of the way, in that order, and then doing the
//! reverse. PROMPT.md principle 8: a wrong restore leaves the user with a
//! broken shell, which is worse than never having run.
//!
//! The restore itself is [`graft_term::TerminalSession`]'s, not this
//! module's: it already knows what it turned on and undoes it newest-first,
//! and it is idempotent. This module only says *when*, and guarantees the
//! way back even if what ran in between panicked.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use graft_term::input::InputMux;

use crate::terminal::{Suspended, TerminalSession};

/// What the frame loop can reach outside itself.
///
/// Two separate things, because they come and go separately: the repository's
/// files are there in the headless path too, and the terminal is not.
#[derive(Clone, Copy, Debug, Default)]
pub struct Outside<'a> {
    /// The repository's common git directory, where `crate::layout` keeps the
    /// detail pane's split and where `Ctrl+E`'s message file goes.
    pub common_dir: Option<&'a Path>,
    /// The terminal, when there is one to hand over.
    pub handover: Option<Handover<'a>>,
}

/// What the frame loop needs to hand the terminal to another program.
///
/// `None` wherever there is no terminal to hand over — the headless path, and
/// every test that drives [`super::pump`] over a `TestBackend`. The keys still
/// reach `update`; what comes back is a refusal saying so, rather than a
/// silently ignored key.
#[derive(Clone, Copy)]
pub struct Handover<'a> {
    /// The modes to put back and then re-apply.
    pub session: &'a TerminalSession,
    /// The reader to take off the terminal's input.
    pub mux: &'a InputMux,
    /// The repository's git directory: where `Ctrl+E`'s message file goes,
    /// and the directory `git var GIT_EDITOR` is asked in.
    pub git_dir: Option<&'a Path>,
    /// The root of the working tree, or `None` for a bare repository.
    ///
    /// Two jobs, both about `e`. It is the directory the editor is started
    /// in, the way git starts one, and it is what a porcelain path is joined
    /// onto to make it absolute. Without it `graft ~/code/api` run from `~`
    /// opened `~/src/main.rs` — a new empty buffer in vim, "file not found"
    /// in code and zed, and a stray file if the user saved
    /// (m3/05 review, Major 2).
    pub workdir: Option<&'a Path>,
}

impl std::fmt::Debug for Handover<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handover")
            .field("git_dir", &self.git_dir)
            .finish_non_exhaustive()
    }
}

/// Where `Ctrl+E`'s message file goes, relative to the git directory.
///
/// Under `graft/` rather than beside git's own `COMMIT_EDITMSG`, which git
/// rewrites on every commit and which a concurrent `git commit` in another
/// terminal owns.
const MESSAGE_FILE: &str = "graft/COMMIT_EDITMSG";

impl Handover<'_> {
    /// `Ctrl+E`: write `message` out, open it in the user's editor, and read
    /// back what was saved.
    ///
    /// The old text is never thrown away on a failure — the caller keeps the
    /// fields as they were and shows the reason — so every error path here
    /// says what happened rather than returning an empty message.
    ///
    /// # Errors
    /// There is no repository, the file could not be written or read, the
    /// editor could not be found or started, or it exited non-zero.
    pub fn edit_message(&self, message: &str) -> Result<String, String> {
        let path = self.message_path()?;
        std::fs::write(&path, message).map_err(|error| format!("{}: {error}", path.display()))?;
        self.open(&path, None)?;
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// `e` on the changes pane: the file under the cursor, at its line.
    ///
    /// # Errors
    /// As [`Handover::open`].
    pub fn edit_file(&self, path: &Path, line: Option<u32>) -> Result<(), String> {
        self.open(path, line)
    }

    /// `e` on a conflicted file: the user's editor, at the file's first
    /// `<<<<<<< ` (m5-history-and-ghost-preview/09).
    ///
    /// The marker lookup happens here rather than in `update` because it
    /// reads the file, and `update` reads nothing (ARCHITECTURE.md). No
    /// marker is **not** a failure — the file has already been resolved by
    /// hand, and the editor opens at the top, which is the truthful place
    /// when there is no marker to point at. A read that genuinely fails is a
    /// different thing and is reported rather than flattened into "line 1".
    ///
    /// # Errors
    /// The file could not be read, or as [`Handover::open`].
    pub fn edit_conflict(&self, path: &Path) -> Result<(), String> {
        let absolute = in_worktree(path, self.workdir);
        let line = graft_git::history_reads::first_marker_line(&absolute)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        self.open(path, line.and_then(|line| u32::try_from(line).ok()))
    }

    /// `M` on a conflicted file: `git mergetool` on it, with the terminal
    /// to itself (m5-history-and-ghost-preview/09).
    ///
    /// The same shape as [`Handover::open`], and deliberately so: the whole
    /// guarantee that the user's shell survives lives in
    /// [`run_outside_terminal`], which restores on **every** way out of
    /// itself — a clean return, an early `?`, a tool that would not start, a
    /// tool killed by a signal, and an unwinding panic. Nothing here may run
    /// the tool outside that call, and nothing here may return before it.
    ///
    /// `--no-prompt` because the prompt would be asked on a terminal Graft
    /// has just given away and answered by nobody. `merge.tool` being set is
    /// the caller's precondition (`update::banner::open_mergetool` refuses
    /// in words without one), so the tool is never git's own interactive
    /// "which one?" question.
    ///
    /// Run in `workdir`, like the editor, because git's porcelain paths are
    /// relative to the top of the working tree and Graft's own process
    /// directory is wherever the user was standing (m3/05 review, Major 2).
    ///
    /// # Errors
    /// The terminal could not be handed over or taken back, `git` could not
    /// be started, or the tool exited non-zero or was killed — in every case
    /// with the terminal already put back.
    pub fn mergetool(&self, path: &Path) -> Result<(), String> {
        let workdir = self
            .workdir
            .ok_or_else(|| "this repository has no working tree".to_owned())?;
        let status = run_outside_terminal(self.session, self.mux, || {
            mergetool_command(path, workdir).status()
        })
        .map_err(|error| error.to_string())?;

        match status {
            Ok(status) if status.success() => Ok(()),
            // Not swallowed: a mergetool that did not finish leaves the
            // conflict exactly where it was, and "it ran" would send the
            // user to press `c` on a file still full of markers.
            Ok(status) => Err(match status.code() {
                Some(code) => format!("mergetool exited {code} for {}", path.display()),
                None => format!("mergetool was killed for {}", path.display()),
            }),
            Err(error) => Err(format!("could not run git mergetool: {error}")),
        }
    }

    /// `Ctrl+Z` under `keys.suspend_on_ctrl_z`: stop, with the terminal the
    /// shell's again, until `fg`.
    ///
    /// # Errors
    /// The terminal could not be handed over, or `kill` refused.
    pub fn suspend(&self) -> Result<(), String> {
        run_outside_terminal(self.session, self.mux, stop_this_process)
            .map_err(|error| error.to_string())?
            .map_err(|error| format!("could not suspend: {error}"))
    }

    /// Run the user's editor on `path` with the terminal to itself.
    fn open(&self, path: &Path, line: Option<u32>) -> Result<(), String> {
        let editor = self.git_editor()?;
        let workdir = self.workdir;
        let status = run_outside_terminal(self.session, self.mux, || {
            editor_command(&editor, path, line, workdir).status()
        })
        .map_err(|error| error.to_string())?;

        match status {
            Ok(status) if status.success() => Ok(()),
            // Not swallowed and not guessed at: the user's message is still
            // in the fields, and this says why it did not change.
            Ok(status) => Err(match status.code() {
                Some(code) => format!("{editor} exited {code}, message unchanged"),
                None => format!("{editor} was killed, message unchanged"),
            }),
            Err(error) => Err(format!("could not run {editor}: {error}")),
        }
    }

    /// The message file's path, its `graft/` directory made.
    fn message_path(&self) -> Result<PathBuf, String> {
        let dir = self.repository()?;
        let path = dir.join(MESSAGE_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        Ok(path)
    }

    /// The editor git itself would run: `GIT_EDITOR`, then `core.editor`,
    /// then `VISUAL`, then `EDITOR`, then git's built-in default — in one
    /// spawn, rather than four lookups Graft would have to keep in step with
    /// git's.
    fn git_editor(&self) -> Result<String, String> {
        let dir = self.repository()?;
        // Through `graft_git`, not a bare `Command::new("git")`. Every other
        // git spawn in the program goes through `run` — fixed environment,
        // output cap, stop policy — and this one was invisible to all three
        // and to the seam guard besides (m3/05 review, Major 8). It still
        // runs on this thread: only the thread holding the terminal can hand
        // it over, and it is about to block on the editor regardless.
        graft_git::editor::git_editor(dir).map_err(|error| error.to_string())
    }

    fn repository(&self) -> Result<&Path, String> {
        self.git_dir
            .ok_or_else(|| "no repository is open".to_owned())
    }
}

/// Why the terminal could not be handed over, or taken back.
#[derive(Debug)]
pub enum OutsideError {
    /// The terminal could not be put back the way the user's shell expects.
    /// Fatal on the way out: running an editor over a half-restored terminal
    /// is how a shell ends up unusable.
    Restore(io::Error),
    /// Graft could not re-enter the terminal it had just left. The user's
    /// shell is fine — Graft is not.
    Reenter(io::Error),
}

impl std::fmt::Display for OutsideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restore(error) => write!(f, "could not restore the terminal: {error}"),
            Self::Reenter(error) => write!(f, "could not re-enter the terminal: {error}"),
        }
    }
}

impl std::error::Error for OutsideError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Restore(error) | Self::Reenter(error) => Some(error),
        }
    }
}

/// Put the terminal back the way Graft found it, run `f` with it, then take
/// it back.
///
/// The order matters and is the reverse of setup: every mode Graft turned on
/// goes off (the alternate screen last, so the shell's scrollback is what the
/// editor draws over), the input reader is paused so it is not racing the
/// editor for keystrokes, `f` runs, and then everything goes back on.
///
/// The way back is a [`Reenter`] guard rather than a straight line, so that
/// an early return or a panic inside `f` cannot skip it. On the panicking
/// path the guard is not what saves the terminal — `TerminalSession`'s own
/// `Drop` restores again as the stack unwinds, and the panic hook has
/// already printed to the scrollback by then. What the guard buys is that
/// there is exactly one way out of this function, so a future `?` inside it
/// cannot leave the terminal handed over (m3/05 review, Minor 1).
///
/// # Errors
/// [`OutsideError::Restore`] when the terminal could not be given up, in
/// which case `f` never runs; [`OutsideError::Reenter`] when it could not be
/// taken back, in which case `f` has already run and its value is lost — the
/// caller is about to exit anyway.
pub fn run_outside_terminal<T>(
    session: &TerminalSession,
    mux: &InputMux,
    f: impl FnOnce() -> T,
) -> Result<T, OutsideError> {
    // The reader first: if it cannot be taken off the terminal there is no
    // point restoring the modes, because Graft would still be eating the
    // editor's keystrokes. Nothing has changed yet, so this is a clean
    // refusal.
    mux.pause().map_err(OutsideError::Restore)?;
    let suspended = match session.suspend() {
        Ok(suspended) => suspended,
        Err(error) => {
            // `suspend` already put back what it could; the reader goes back
            // on the terminal too, or Graft would take no more keys.
            mux.resume();
            return Err(OutsideError::Restore(error));
        }
    };

    let guard = Reenter {
        session,
        mux,
        suspended: Some(suspended),
        done: false,
    };
    let value = f();
    guard.finish().map(|()| value)
}

/// Re-enters the terminal when it is dropped, panic or no panic.
struct Reenter<'a> {
    session: &'a TerminalSession,
    mux: &'a InputMux,
    suspended: Option<Suspended>,
    /// [`Self::finish`] already re-entered, so [`Drop`] has nothing to do.
    done: bool,
}

impl Reenter<'_> {
    /// Re-enter now, reporting a failure the caller can act on. `Drop` does
    /// the same thing silently, for the panicking path where there is no
    /// caller left to tell.
    fn finish(mut self) -> Result<(), OutsideError> {
        let result = self.reenter();
        self.done = true;
        result.map_err(OutsideError::Reenter)
    }

    /// The modes back on, the reader reading again, and a full redraw asked
    /// for — the editor has scribbled over every cell Graft thought it knew.
    ///
    /// **Modes first, reader second.** Between the two the terminal is
    /// nobody's; a key typed in that window while raw mode is still off is
    /// cooked and echoed, so the user sees their own keystroke printed over
    /// the screen Graft is about to redraw (m3/05 review, Minor 1).
    fn reenter(&mut self) -> io::Result<()> {
        let result = match self.suspended.take() {
            Some(suspended) => self.session.resume(suspended),
            // Only reachable if `finish` ran twice, which it cannot: it
            // takes `self` by value.
            None => Ok(()),
        };
        // The reader goes back on either way: leaving it parked after a
        // failed resume would mean Graft never took another key, which is
        // worse than a terminal whose modes are half back.
        self.mux.resume();
        result
    }
}

impl Drop for Reenter<'_> {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        // Unwinding: there is no caller to return an error to, and the panic
        // hook is about to print. Re-entering is still the right thing —
        // failing to is not swallowed, it is printed, because a terminal
        // left half-restored is the one thing the user must know about.
        if let Err(error) = self.reenter() {
            eprintln!("graft: could not re-enter the terminal after a panic: {error}");
        }
    }
}

/// The command that opens `path` in `git_editor`, at `line` where the editor
/// takes a line and Graft knows how to ask for it.
///
/// `path` is made absolute against `workdir` first, and `workdir` becomes the
/// command's directory. Every path Graft has to offer comes out of git's
/// porcelain and is relative to the top of the working tree; Graft's own
/// process directory is wherever the user was standing when they typed
/// `graft ~/code/api`. Handing the editor a path relative to *that* opened
/// an empty new buffer in vim and "file not found" in code and zed, and
/// created a stray file if the user saved (m3/05 review, Major 2).
///
/// `git_editor` is what `git var GIT_EDITOR` printed, which is a **shell
/// command**, not a program name. Two shapes:
///
/// - A plain word list — `vim`, `emacs -nw`, `code --wait` — is split on
///   whitespace and run directly. The first word is the program; the rest
///   are arguments that must survive, so they stay in front of anything
///   added here.
/// - Anything carrying shell syntax is run by `sh`, which is what git does
///   (`prepare_shell_cmd`). The common macOS `core.editor`
///   `'/Applications/Sublime Text.app/…/subl' -w` is the case that matters:
///   splitting it on whitespace gave "could not run `'/Applications/Sublime`"
///   (m3/05 review, Major 5).
///
/// The line flag is a per-editor table because there is no convention: the
/// `+N` family is the vi lineage, VS Code wants `--goto file:line`, Zed takes
/// `file:line` bare. An editor not in the table gets the file and no line,
/// which is right — a wrong flag would be passed through as a filename and
/// the user would be editing a file called `+12`.
#[must_use]
pub fn editor_command(
    git_editor: &str,
    path: &Path,
    line: Option<u32>,
    workdir: Option<&Path>,
) -> Command {
    let path = in_worktree(path, workdir);
    let program = git_editor.split_whitespace().next().unwrap_or(git_editor);
    let arguments = line_arguments(line_style(program), &path, line);

    let mut command = if needs_a_shell(git_editor) {
        // `sh -c '<editor> "$@"' <editor> <arguments>`: `$0` is the editor
        // string, so a failure `sh` reports names what the user configured.
        let mut command = Command::new("/bin/sh");
        command.arg("-c");
        command.arg(format!("{git_editor} \"$@\""));
        command.arg(git_editor);
        command
    } else {
        let mut words = git_editor.split_whitespace();
        let mut command = Command::new(words.next().unwrap_or(git_editor));
        command.args(words);
        command
    };
    command.args(arguments);
    if let Some(root) = workdir {
        command.current_dir(root);
    }
    command
}

/// `git mergetool --no-prompt -- <path>`, run in the working tree's root.
///
/// A plain [`Command`], not one of `graft_git::run`'s invocations, and for
/// the one reason `editor_command` is too: `run` captures stdout and stderr
/// and caps the output, which is exactly wrong for a full-screen program
/// that must have the real terminal. `Command::new("git")` is what `run`
/// itself spawns (`graft-git/src/run.rs`), so this is the same `git` the
/// rest of the process uses.
///
/// This is not a hole in the seam rule (`tests/it/seam_rule.rs`): it builds
/// no write for `graft_ops` to plan. `git mergetool` edits files in the
/// working tree and touches no ref and no journal entry — the operation it
/// belongs to is already open, and `resume` is what closes it.
///
/// `--` before the path so a file called `-x` is a file and not a flag.
#[must_use]
pub fn mergetool_command(path: &Path, workdir: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("mergetool")
        .arg("--no-prompt")
        .arg("--")
        .arg(path)
        .current_dir(workdir);
    command
}

/// `path` joined onto the working tree's root, unless it is already absolute
/// or there is no root to join it onto (a bare repository).
fn in_worktree(path: &Path, workdir: Option<&Path>) -> PathBuf {
    match workdir {
        Some(root) if path.is_relative() => root.join(path),
        _ => path.to_path_buf(),
    }
}

/// Characters that make a `GIT_EDITOR` a shell command rather than a word
/// list. Git's own set (`prepare_shell_cmd`) also counts a space, which
/// would send every `code --wait` through `sh` for nothing; these are the
/// ones that actually change the meaning of the words.
const SHELL_SYNTAX: &[char] = &[
    '\'', '"', '$', '`', '\\', ';', '&', '|', '<', '>', '(', ')', '*', '?', '[', '\n', '\t',
];

fn needs_a_shell(git_editor: &str) -> bool {
    git_editor.contains(SHELL_SYNTAX)
}

/// The file, and the line if this editor takes one.
fn line_arguments(style: LineStyle, path: &Path, line: Option<u32>) -> Vec<OsString> {
    match (line, style) {
        (Some(line), LineStyle::Plus) => {
            vec![OsString::from(format!("+{line}")), path.into()]
        }
        (Some(line), LineStyle::Goto) => {
            vec![OsString::from("--goto"), suffixed(path, line).into()]
        }
        (Some(line), LineStyle::Suffix) => vec![suffixed(path, line).into()],
        (Some(_), LineStyle::None) | (None, _) => vec![path.into()],
    }
}

/// `path:line`, as VS Code and Zed want it. Lossy on a path that is not
/// UTF-8, which is unavoidable: the argument is one string with a `:` in it,
/// so there is nowhere for the raw bytes to go. A repository path that is
/// not UTF-8 therefore opens at the top of the file rather than not at all.
fn suffixed(path: &Path, line: u32) -> String {
    format!("{}:{line}", path.to_string_lossy())
}

/// How an editor is told which line to open at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineStyle {
    /// `editor +12 path` — the vi lineage, and most of the rest.
    Plus,
    /// `editor --goto path:12` — VS Code and its forks.
    Goto,
    /// `editor path:12` — Zed.
    Suffix,
    /// This editor is not in the table: open the file, say nothing about the
    /// line.
    None,
}

/// The table. Matched on the program's file name, so `/usr/bin/vim` and
/// `vim` are the same editor, and `vim.exe` too.
fn line_style(program: &str) -> LineStyle {
    let name = Path::new(program)
        .file_stem()
        .map_or(program, |stem| stem.to_str().unwrap_or(program));
    match name {
        "vim" | "nvim" | "vi" | "nano" | "emacs" | "emacsclient" | "micro" | "hx" | "helix" => {
            LineStyle::Plus
        }
        "code" | "code-insiders" | "codium" | "cursor" | "windsurf" => LineStyle::Goto,
        "zed" | "zeditor" => LineStyle::Suffix,
        _ => LineStyle::None,
    }
}

/// Stop Graft the way `Ctrl+Z` stops any other terminal program, by sending
/// this process `SIGTSTP`.
///
/// Through `kill(1)` rather than `libc::raise`: neither `libc` nor `nix` is a
/// dependency of this crate, and adding one for a single signal — on a path
/// the user has to turn on in config — costs more than the spawn does
/// (the spec says so too).
///
/// The shell puts Graft back in the foreground with `fg`, and the call
/// returns then; the terminal has been restored around it by
/// [`run_outside_terminal`], so `fg` lands the user back in a redrawn Graft.
///
/// # Errors
/// `kill` could not be run, or refused.
pub fn stop_this_process() -> io::Result<()> {
    let status = Command::new("kill")
        .args(["-TSTP", &std::process::id().to_string()])
        .status()?;
    if status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!("kill -TSTP: {status}")))
}

#[cfg(test)]
mod tests;
