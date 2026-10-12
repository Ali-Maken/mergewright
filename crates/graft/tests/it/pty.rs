//! The real `graft` binary on a pseudo-terminal (m8-polish-and-ship/08): the
//! one way to see what only a tty gets — the empty state, the alternate
//! screen entered once across a switch, the probe asked once.
//!
//! Every run is **isolated**: `XDG_CONFIG_HOME` and `HOME` point into a
//! temporary directory the test owns, so neither the user's config nor his
//! recent list (`~/.config/graft/recent`) is read or written. That isolation
//! lives here, in [`Pty::spawn`], not in each test.
//!
//! [`Screen`] replays the bytes into a character grid — cursor moves, text,
//! clears — enough for ratatui's output; colour is dropped (CLAUDE.md
//! rule 5: captures cannot prove colour).
#![allow(
    clippy::expect_used,
    reason = "a pty that cannot be opened leaves nothing to test; the panic names why"
)]

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use graft_core::on_disk::CURRENT_DIR;
use nix::pty::{Winsize, openpty};

/// How long a wait may take before the test fails, naming what it waited for.
const PATIENCE: Duration = Duration::from_secs(20);

/// The terminal every pty walk runs in, whatever terminal ran the tests: an
/// `xterm-256color` that says `COLORTERM=truecolor`, outside tmux, not over
/// ssh, and naming no emulator.
///
/// Detection picks the Unicode tier only on a truecolor hint or `$TMUX`
/// (`graft-term/src/detect/tier.rs`, DESIGN.md §4.2), and those hints come
/// from the environment. Inherited, they made every walk's tier the
/// developer's: Ghostty's `TERM_PROGRAM=ghostty` gave Unicode on the Mac,
/// and a shell without them (a Linux container, Terminal.app) drew the ASCII
/// tier and failed 40 walks waiting for `▸` (m8-polish-and-ship/30). A test
/// that wants another terminal sets these again after this.
pub(crate) fn as_the_test_terminal(command: &mut Command) -> &mut Command {
    command
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env_remove("TERM_PROGRAM")
        .env_remove("TERM_PROGRAM_VERSION")
        .env_remove("KITTY_WINDOW_ID")
        .env_remove("TMUX")
        .env_remove("SSH_TTY")
        .env_remove("SSH_CONNECTION")
        .env_remove("NO_COLOR")
}

/// A running `graft` on a pty.
pub struct Pty {
    master: File,
    child: Child,
    output: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    columns: u16,
    rows: u16,
}

/// Where a run keeps its config and home.
pub struct Isolated {
    pub root: tempfile::TempDir,
}

impl Isolated {
    /// A scratch config home whose first-launch line is already `done`
    /// (m8/18d): a walk sees the screen as he does after his first palette
    /// open, so no existing screen changes.
    pub fn new() -> Self {
        let isolated = Self::first_launch();
        let file = isolated.launches_file();
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("config dir");
        std::fs::write(file, "done\n").expect("seed launches");
        isolated
    }

    /// A scratch config home as on his very first launch: no `launches`.
    pub fn first_launch() -> Self {
        Self {
            root: tempfile::tempdir().expect("a scratch config home"),
        }
    }

    /// The first-launch count's file (m8/18d).
    pub fn launches_file(&self) -> PathBuf {
        self.config().join(CURRENT_DIR).join("launches")
    }

    /// The root, canonical: on macOS the temp dir is under a `/var` that is
    /// a link to `/private/var`, and the app compares canonical paths with
    /// `$HOME` to draw `~`.
    fn canonical_root(&self) -> PathBuf {
        self.root
            .path()
            .canonicalize()
            .expect("the scratch root resolves")
    }

    /// `$XDG_CONFIG_HOME`.
    pub fn config(&self) -> PathBuf {
        self.canonical_root().join("config")
    }

    /// `$HOME`.
    pub fn home(&self) -> PathBuf {
        self.canonical_root().join("home")
    }

    /// The recent list's file.
    pub fn recent_file(&self) -> PathBuf {
        self.config().join(CURRENT_DIR).join("recent")
    }

    /// Write the recent list: `(unix secs, path)`, newest first.
    pub fn write_recent(&self, entries: &[(u64, &Path)]) {
        let file = self.recent_file();
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("config dir");
        let mut text = String::new();
        for (secs, path) in entries {
            text.push_str(&format!("{secs}\t{}\n", path.display()));
        }
        std::fs::write(file, text).expect("recent written");
    }
}

/// The instant every pty run's session clock is pinned to unless the test
/// names `TRIB_NOW` itself (m8/19d-b). One value for every producer:
/// `graft_fixtures::PINNED_NOW` (m8/19d-b2).
pub const PINNED_NOW: i64 = graft_fixtures::PINNED_NOW;

impl Pty {
    /// Start `graft <arg>` in `cwd` on a `columns`x`rows` pty, isolated by
    /// `isolated`. `GIT_CEILING_DIRECTORIES` stops git's search at `ceiling`
    /// so a repository above the temp dir never counts.
    pub fn spawn(isolated: &Isolated, arg: &Path, ceiling: &Path, size: (u16, u16)) -> Self {
        Self::spawn_with(isolated, arg, ceiling, size, false)
    }

    /// As [`Pty::spawn`], with stderr on the pty too, the way a shell runs
    /// it: one transcript, so a test can read what came after what — the
    /// terminal's restore, then a line on stderr (m8/13b review fix).
    pub fn spawn_with_stderr_on_pty(
        isolated: &Isolated,
        arg: &Path,
        ceiling: &Path,
        size: (u16, u16),
    ) -> Self {
        Self::spawn_with(isolated, arg, ceiling, size, true)
    }

    /// As [`Pty::spawn`], with git's system and global config switched off
    /// (m8/18c): a clone has no repository whose `credential.helper` could
    /// be blanked first, so the machine's keychain helper must neither
    /// answer git's question for the test nor keep its password. The global
    /// file is an empty one under the isolated home.
    pub fn spawn_without_git_config(
        isolated: &Isolated,
        arg: &Path,
        ceiling: &Path,
        size: (u16, u16),
    ) -> Self {
        let global = isolated.home().join("empty.gitconfig");
        std::fs::create_dir_all(isolated.home()).expect("home");
        std::fs::write(&global, "").expect("an empty global git config");
        Self::spawn_env(
            isolated,
            arg,
            ceiling,
            size,
            false,
            &[
                ("GIT_CONFIG_NOSYSTEM", "1".into()),
                ("GIT_CONFIG_GLOBAL", global.into_os_string()),
            ],
        )
    }

    /// As [`Pty::spawn`], with `env` set on top (m8/19b: `TMPDIR`, where
    /// the practice tour builds its repository).
    pub fn spawn_with_env(
        isolated: &Isolated,
        arg: &Path,
        ceiling: &Path,
        size: (u16, u16),
        env: &[(&str, std::ffi::OsString)],
    ) -> Self {
        Self::spawn_env(isolated, arg, ceiling, size, false, env)
    }

    fn spawn_with(
        isolated: &Isolated,
        arg: &Path,
        ceiling: &Path,
        size: (u16, u16),
        stderr_on_pty: bool,
    ) -> Self {
        Self::spawn_env(isolated, arg, ceiling, size, stderr_on_pty, &[])
    }

    fn spawn_env(
        isolated: &Isolated,
        arg: &Path,
        ceiling: &Path,
        (columns, rows): (u16, u16),
        stderr_on_pty: bool,
        env: &[(&str, std::ffi::OsString)],
    ) -> Self {
        std::fs::create_dir_all(isolated.config()).expect("config home");
        std::fs::create_dir_all(isolated.home()).expect("home");
        let size = Winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pty = openpty(Some(&size), None).expect("a pty");
        let slave: OwnedFd = pty.slave;
        let stderr = if stderr_on_pty {
            Stdio::from(slave.try_clone().expect("slave fd"))
        } else {
            Stdio::piped()
        };
        let pinned_git_date = format!("{} +0000", PINNED_NOW - 3_600);
        let mut command = Command::new(env!("CARGO_BIN_EXE_mwr"));
        let mut child = as_the_test_terminal(&mut command)
            .arg(arg)
            .env("XDG_CONFIG_HOME", isolated.config())
            .env("HOME", isolated.home())
            .env("GIT_CEILING_DIRECTORIES", ceiling)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("TRIB_TRACE")
            // Set before `env`, so a caller naming `TRIB_NOW` wins.
            .env("TRIB_NOW", PINNED_NOW.to_string())
            // A commit made in the app is dated an hour before the pin, so
            // it draws `1h` (not `future`) and its id is the same every run
            // (m8/19d-b2, ticket 19d §R5). Graft itself never sets git's
            // dates; a caller naming either variable wins.
            .env("GIT_AUTHOR_DATE", &pinned_git_date)
            .env("GIT_COMMITTER_DATE", &pinned_git_date)
            .envs(env.iter().map(|(name, value)| (*name, value)))
            .stdin(Stdio::from(slave.try_clone().expect("slave fd")))
            .stdout(Stdio::from(slave))
            .stderr(stderr)
            .spawn()
            .expect("graft starts");
        let master = File::from(pty.master);
        let output = drain(master.try_clone().expect("master fd"));
        let stderr = Arc::new(Mutex::new(Vec::new()));
        // On the pty, stderr is in `output`; piped, it is gathered apart.
        if let Some(mut child_err) = child.stderr.take() {
            let err_sink = Arc::clone(&stderr);
            std::thread::spawn(move || {
                let mut all = Vec::new();
                if child_err.read_to_end(&mut all).is_ok() {
                    err_sink.lock().expect("stderr").extend(all);
                }
            });
        }
        Self {
            master,
            child,
            output,
            stderr,
            columns,
            rows,
        }
    }

    /// Type `bytes`.
    pub fn send(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).expect("typed");
        self.master.flush().expect("flushed");
    }

    /// Send the app `signal` (`TERM`, `HUP`) with `kill`, by its pid.
    pub fn signal(&self, signal: &str) {
        let status = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(self.child.id().to_string())
            .status()
            .expect("kill runs");
        assert!(status.success(), "SIG{signal} not delivered");
    }

    /// Everything the app has written so far.
    pub fn output(&self) -> Vec<u8> {
        self.output.lock().expect("output").clone()
    }

    /// The screen as the bytes so far draw it.
    pub fn screen(&self) -> Screen {
        Screen::replay(&self.output(), self.columns, self.rows)
    }

    /// Wait until the screen satisfies `done`, failing with `what` and the
    /// last screen after [`PATIENCE`].
    pub fn wait_for(&self, what: &str, done: impl Fn(&Screen) -> bool) -> Screen {
        let start = Instant::now();
        loop {
            let screen = self.screen();
            if done(&screen) {
                return screen;
            }
            assert!(
                start.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for {what}; the screen:\n{}",
                screen.text()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Wait for the process to end; its status and stderr.
    pub fn wait_exit(mut self) -> (ExitStatus, String, Vec<u8>) {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("waitable") {
                // The readers drain what is left once the child has gone.
                std::thread::sleep(Duration::from_millis(50));
                let stderr =
                    String::from_utf8_lossy(&self.stderr.lock().expect("stderr")).into_owned();
                return (status, stderr, self.output());
            }
            if start.elapsed() > PATIENCE {
                self.child.kill().expect("killed");
                panic!("graft did not exit; the screen:\n{}", self.screen().text());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // A test that failed before `wait_exit` leaves no graft behind. An
        // already-exited child makes `kill` fail, which is what we wanted.
        if matches!(self.child.try_wait(), Ok(None)) {
            if let Err(error) = self.child.kill() {
                eprintln!("could not stop graft: {error}");
            }
            if let Err(error) = self.child.wait() {
                eprintln!("could not reap graft: {error}");
            }
        }
    }
}

/// How many times `needle` occurs in `haystack`.
pub fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

/// A character grid replayed from a terminal byte stream.
pub struct Screen {
    cells: Vec<Vec<char>>,
}

/// A replay in progress: the grid, the cursor, and where in the stream.
struct Replay {
    cells: Vec<Vec<char>>,
    columns: usize,
    rows: usize,
    column: usize,
    row: usize,
    chars: Vec<char>,
    at: usize,
}

impl Replay {
    fn next(&mut self) -> Option<char> {
        let next = self.chars.get(self.at).copied();
        self.at += 1;
        next
    }

    fn print(&mut self, glyph: char) {
        if let Some(cell) = self
            .cells
            .get_mut(self.row)
            .and_then(|line| line.get_mut(self.column))
        {
            *cell = glyph;
        }
        self.column += 1;
    }

    /// After `ESC`: a CSI is acted on; OSC, APC and DCS are skipped to their
    /// terminator (BEL or ST); anything else is one ignored character.
    fn escape(&mut self) {
        match self.next() {
            Some('[') => self.csi(),
            Some(']' | '_' | 'P') => self.skip_string(),
            _ => {}
        }
    }

    fn skip_string(&mut self) {
        while let Some(next) = self.next() {
            if next == '\u{7}' {
                return;
            }
            if next == '\u{1b}' && self.chars.get(self.at) == Some(&'\\') {
                self.at += 1;
                return;
            }
        }
    }

    fn csi(&mut self) {
        let start = self.at;
        while self
            .chars
            .get(self.at)
            .is_some_and(|next| !('@'..='~').contains(next))
        {
            self.at += 1;
        }
        let Some(last) = self.next() else { return };
        let params: String = self.chars[start..self.at - 1].iter().collect();
        let numbers: Vec<usize> = params
            .trim_start_matches(['?', '>', '<', '='])
            .split(';')
            .map(|number| number.parse().unwrap_or(0))
            .collect();
        let arg = |index: usize| {
            numbers
                .get(index)
                .copied()
                .filter(|&value| value > 0)
                .unwrap_or(1)
        };
        match last {
            'H' | 'f' => {
                self.row = arg(0) - 1;
                self.column = arg(1) - 1;
            }
            'A' => self.row = self.row.saturating_sub(arg(0)),
            'B' => self.row += arg(0),
            'C' => self.column += arg(0),
            'D' => self.column = self.column.saturating_sub(arg(0)),
            'G' => self.column = arg(0) - 1,
            'd' => self.row = arg(0) - 1,
            'J' if numbers.first() == Some(&2) => {
                self.cells = vec![vec![' '; self.columns]; self.rows];
            }
            'K' => {
                let column = self.column;
                if let Some(line) = self.cells.get_mut(self.row) {
                    for cell in line.iter_mut().skip(column) {
                        *cell = ' ';
                    }
                }
            }
            _ => {}
        }
    }
}

impl Screen {
    /// Replay `bytes` onto a blank `columns`x`rows` grid. Understood: printable
    /// text, `CR`/`LF`, CSI `H`/`f` (position), `A`/`B`/`C`/`D`/`G`/`d`,
    /// `J` (2 clears), `K`; everything else (SGR, modes, OSC, APC, DCS, the
    /// probe's queries) is skipped.
    ///
    /// Shown as a DEC 2026 terminal shows it: a synchronized update that has
    /// begun and not ended is not on the screen yet. Every frame Graft draws
    /// is one update (ADR-0054 §3), and the pty hands its bytes over in
    /// pieces, so without this a wait could take half a frame for the screen
    /// (13h).
    pub fn replay(bytes: &[u8], columns: u16, rows: u16) -> Self {
        let bytes = ended(bytes);
        let (columns, rows) = (usize::from(columns), usize::from(rows));
        let mut replay = Replay {
            cells: vec![vec![' '; columns]; rows],
            columns,
            rows,
            column: 0,
            row: 0,
            chars: String::from_utf8_lossy(bytes).chars().collect(),
            at: 0,
        };
        while let Some(next) = replay.next() {
            match next {
                '\u{1b}' => replay.escape(),
                '\r' => replay.column = 0,
                '\n' => replay.row += 1,
                control if control.is_control() => {}
                glyph => replay.print(glyph),
            }
        }
        Self {
            cells: replay.cells,
        }
    }

    /// The rows, trailing blanks trimmed.
    pub fn lines(&self) -> Vec<String> {
        self.cells
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_owned())
            .collect()
    }

    /// The whole grid as text.
    pub fn text(&self) -> String {
        self.lines().join("\n")
    }

    /// Does any row contain `needle`?
    pub fn has(&self, needle: &str) -> bool {
        self.lines().iter().any(|line| line.contains(needle))
    }
}

/// `bytes` up to the synchronized update still being written, if one is:
/// what a terminal that holds an update until it ends has shown.
fn ended(bytes: &[u8]) -> &[u8] {
    let last = |needle: &[u8]| {
        bytes
            .windows(needle.len())
            .rposition(|window| window == needle)
    };
    match (last(graft_term::SYNC_BEGIN), last(graft_term::SYNC_END)) {
        (Some(begin), Some(end)) if end > begin => bytes,
        (Some(begin), _) => &bytes[..begin],
        (None, _) => bytes,
    }
}

/// A frame still being written is not on the screen yet (13h): the replay
/// shows what a DEC 2026 terminal shows, the last frame that ended.
#[test]
fn a_frame_not_yet_ended_is_not_on_the_screen() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(graft_term::SYNC_BEGIN);
    bytes.extend_from_slice(b"\x1b[1;1Hfirst");
    bytes.extend_from_slice(graft_term::SYNC_END);
    bytes.extend_from_slice(graft_term::SYNC_BEGIN);
    bytes.extend_from_slice(b"\x1b[1;1Hsecond, half");
    let screen = Screen::replay(&bytes, 20, 2);
    assert_eq!(screen.lines()[0], "first", "{}", screen.text());

    bytes.extend_from_slice(graft_term::SYNC_END);
    let screen = Screen::replay(&bytes, 20, 2);
    assert_eq!(screen.lines()[0], "second, half", "{}", screen.text());
}

/// m8-polish-and-ship/30: whatever terminal ran the tests, a walk's graft
/// sees the same one: the truecolor hint detection gives Unicode on, and
/// none of the hints that would make it some other terminal.
#[test]
fn a_walk_runs_in_the_test_terminal_whatever_ran_the_tests() {
    let mut command = Command::new("env");
    command
        .env("TERM", "dumb")
        .env_remove("COLORTERM")
        .env("TERM_PROGRAM", "Apple_Terminal")
        .env("TERM_PROGRAM_VERSION", "455")
        .env("KITTY_WINDOW_ID", "1")
        .env("TMUX", "/tmp/tmux-501/default,1,0")
        .env("SSH_TTY", "/dev/ttys001")
        .env("SSH_CONNECTION", "10.0.0.1 22 10.0.0.2 22")
        .env("NO_COLOR", "1");
    let out = as_the_test_terminal(&mut command)
        .output()
        .expect("env runs");
    let seen = String::from_utf8(out.stdout).expect("env prints text");
    let mut vars: Vec<&str> = seen
        .lines()
        .filter(|line| {
            [
                "TERM=",
                "COLORTERM=",
                "TERM_PROGRAM",
                "KITTY_WINDOW_ID=",
                "TMUX=",
                "SSH_TTY=",
                "SSH_CONNECTION=",
                "NO_COLOR=",
            ]
            .iter()
            .any(|name| line.starts_with(name))
        })
        .collect();
    vars.sort_unstable();
    assert_eq!(vars, ["COLORTERM=truecolor", "TERM=xterm-256color"]);
}

/// Gathers everything the pty's master reads, on its own thread.
fn drain(mut reader: File) -> Arc<Mutex<Vec<u8>>> {
    let output = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&output);
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        // Ends at EOF or EIO, which is how a pty says its last slave
        // closed: the child has gone.
        while let Ok(read @ 1..) = reader.read(&mut buffer) {
            sink.lock()
                .expect("output")
                .extend_from_slice(&buffer[..read]);
        }
    });
    output
}
